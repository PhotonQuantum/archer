use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use itertools::Itertools;

use crate::error::{DependencyError, Error};
use crate::types::*;

use super::types::*;

type Ctx = Context<PackageWithParent>;
type VecPackageWithParent = Vec<Arc<PackageWithParent>>;

#[derive(Clone)]
pub struct TreeResolver {
    policy: ResolvePolicy,
    allow_cyclic: bool,
}

impl TreeResolver {
    pub fn new(policy: ResolvePolicy, allow_cyclic: bool) -> Self {
        TreeResolver {
            policy,
            allow_cyclic,
        }
    }

    fn union_into_ctx(&self, ctx: Ctx, pkgs: VecPackageWithParent) -> Result<Ctx> {
        pkgs.into_iter()
            .fold(Some(Ctx::new()), |acc, x| acc.and_then(|acc| acc.insert(x)))
            .and_then(|ctx_merge| ctx.union(ctx_merge))
            .ok_or_else(|| {
                Error::DependencyError(DependencyError::ConflictDependency(String::from(
                    "can't be merged",
                )))
            })
    }

    fn insert_into_ctx(&self, mut ctx: Ctx, pkg: Arc<PackageWithParent>) -> Result<Option<Ctx>> {
        Ok(self.insert_into_ctx_mut(&mut ctx, pkg)?.then(|| ctx))
    }

    fn insert_into_ctx_mut(&self, ctx: &mut Ctx, pkg: Arc<PackageWithParent>) -> Result<bool> {
        if self.policy.is_mortal_blade(&*pkg)? {
            Err(Error::DependencyError(DependencyError::ConflictDependency(
                String::from("conflict with immortal package"),
            )))
        } else {
            Ok(ctx.insert_mut(pkg))
        }
    }

    // TODO dfs only, no topo sort yet
    pub fn resolve(&mut self, pkgs: &[Package]) -> Result<Ctx> {
        let mut stage_pkgs: Vec<Box<dyn Iterator<Item = VecPackageWithParent>>> = vec![];
        let mut depth = 0;

        // push initial set
        let (initial_ctx, initial_pkgs) = pkgs.iter().map(PackageWithParent::from).fold(
            Ok((Ctx::new(), vec![])),
            |acc: Result<_>, x| {
                let name = x.to_string();
                let x = Arc::new(x);
                acc.and_then(|(ctx, mut pkgs)| {
                    let ctx =
                        self.insert_into_ctx(ctx, x.clone())?
                            .ok_or(Error::DependencyError(DependencyError::ConflictDependency(
                                name,
                            )))?;
                    pkgs.push(x);
                    Ok((ctx, pkgs))
                })
            },
        )?;
        let initial_pkgs = self.next_candidates(&initial_pkgs, initial_ctx.clone())?;
        if let Some(initial_pkgs) = initial_pkgs {
            stage_pkgs.push(initial_pkgs);
        } else {
            return Ok(initial_ctx);
        }
        let mut partial_solutions = vec![initial_ctx];

        let mut depth_try_count = 0u32;
        let mut rewind = false;
        loop {
            if rewind || depth_try_count > 300 {
                // limit search space, backtrack earlier when there's little hope (maybe an earlier step is to blame)
                if depth == 0 {
                    // TODO better error reporting
                    // stack depleted
                    return Err(Error::DependencyError(DependencyError::ConflictDependency(
                        String::from("can't find solution"),
                    )));
                } else {
                    partial_solutions.pop().unwrap();
                    drop(stage_pkgs.pop().unwrap());
                    depth -= 1;
                    println!("rewinding to {}", depth);
                }
            }
            let partial_solution = partial_solutions.get(depth).unwrap().clone();
            if let Some(candidates) = stage_pkgs.get_mut(depth).unwrap().next() {
                if partial_solution.is_superset(
                    candidates
                        .iter()
                        .map(|i| i.as_ref())
                        .collect_vec()
                        .as_slice(),
                ) {
                    return Ok(partial_solution); // no new dependency, solution found
                }

                let partial_solution =
                    match self.union_into_ctx(partial_solution, candidates.clone()) {
                        Ok(v) => v,
                        Err(_) => continue, // not accepted, try the next set of candidates
                    };

                // Current set of candidates accepted, start forming next stage

                println!("searching candidates");
                let next_candidates =
                    self.next_candidates(&*candidates, partial_solution.clone())?;
                let next_candidates = if let Some(v) = next_candidates {
                    v
                } else {
                    return Ok(partial_solution);
                };

                depth += 1;
                println!("step into depth {}", depth);
                depth_try_count = 0;
                partial_solutions.push(partial_solution);
                stage_pkgs.push(next_candidates);
            } else {
                rewind = true; // current node depleted with no solution found. backtracking...
            }
        }
    }

    fn next_candidates<'a>(
        &'a self,
        candidates: &[Arc<PackageWithParent>],
        partial_solution: Ctx,
    ) -> Result<Option<Box<dyn Iterator<Item = VecPackageWithParent> + 'a>>> {
        // get deps of all candidates and merge them
        let merged_deps: Vec<_> = candidates
            .iter()
            .fold(HashMap::new(), |mut acc: HashMap<_, DependVersion>, x| {
                let deps = x.dependencies();
                for dep in deps {
                    acc.entry(dep.name.clone()) // Oops we don't need to clone there! Watch out borrow checker :(
                        .and_modify(|e| *e = e.union(&dep.version.clone()))
                        .or_insert(dep.version);
                }
                acc
            })
            .into_iter()
            .map(|(name, version)| Depend { name, version })
            .collect();
        if merged_deps.is_empty() {
            return Ok(None); // no new deps needed
        }

        let cloned_policy = self.policy.clone(); // clone for closure use

        // Layout of `resolved_deps`:
        // [[solution1, solution2, ...]: dep1, dep2, ...]
        let resolved_deps = self
            .policy
            .from_repo
            .lock()
            .unwrap()
            .find_packages(&*merged_deps)
            // TODO build dependency tree in each depth
            .map(move |i| {
                i.into_values()
                    .sorted_by(|a, b| b.len().cmp(&a.len())) // heuristic strategy: iter solution from packages with less deps
                    .map(|pkg| (pkg, cloned_policy.clone(), cloned_policy.clone())) // clone for closure use
                    .map(move |(pkg, cloned_policy, cloned_policy_2)| {
                        pkg.into_iter()
                            .filter(move |pkg| !cloned_policy.is_mortal_blade(pkg).unwrap())
                            .sorted_by(|a, b| {
                                let a = PackageWithParent::from(a);
                                let b = PackageWithParent::from(b);
                                if partial_solution.contains_exact(&a)          // prefer chosen packages
                                    || cloned_policy_2.is_immortal(&a).unwrap()
                                // prefer immortal packages
                                {
                                    Ordering::Less
                                } else if partial_solution.contains_exact(&b)
                                    || cloned_policy_2.is_immortal(&b).unwrap()
                                {
                                    Ordering::Greater
                                } else {
                                    Ordering::Equal
                                }
                            })
                            .take(5) // limit search space
                            .map(PackageWithParent::from)
                            .map(Arc::new)
                    })
                    .collect_vec()
            })?;

        // [solution1, solution2, ...]
        let next_candidates = resolved_deps.into_iter().multi_cartesian_product();
        Ok(Some(Box::new(next_candidates)))
    }
}
