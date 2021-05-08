use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use itertools::Itertools;
use maplit::hashset;

use crate::error::{DependencyError, Error};
use crate::types::*;

use super::types::*;

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

    fn insert_into_ctx(
        &self,
        mut ctx: Context,
        pkg: Arc<Package>,
        reason: HashSet<Arc<Package>>,
    ) -> Result<Option<Context>> {
        Ok(self
            .insert_into_ctx_mut(&mut ctx, pkg, reason)?
            .then(|| ctx))
    }

    fn insert_into_ctx_mut(
        &self,
        ctx: &mut Context,
        pkg: Arc<Package>,
        reason: HashSet<Arc<Package>>,
    ) -> Result<bool> {
        if self.policy.is_mortal_blade(&*pkg)? {
            Err(Error::DependencyError(DependencyError::ConflictDependency(
                String::from("conflict with immortal package"),
            )))
        } else {
            Ok(ctx.insert_mut(pkg, reason))
        }
    }

    // TODO dfs only, no topo sort yet
    pub fn resolve(
        &self,
        pkgs: &[Package],
        depend_policy_fn: impl Fn(&Package) -> DependPolicy + Copy,
    ) -> Result<Context> {
        let mut stage_ctxs: Vec<Box<dyn Iterator<Item = Context>>> = vec![];
        let mut depth = 0;

        // push initial set
        let initial_ctx = pkgs
            .iter()
            .filter(|pkg| {
                self.policy
                    .skip_repo
                    .find_package(&Depend::from(*pkg))
                    .unwrap()
                    .is_empty()
            })
            .cloned()
            .fold(Ok(Context::new()), |acc: Result<_>, x| {
                let name = x.to_string();
                let x = Arc::new(x);
                acc.and_then(|ctx| {
                    let ctx = self.insert_into_ctx(ctx, x.clone(), hashset!())?.ok_or(
                        Error::DependencyError(DependencyError::ConflictDependency(name)),
                    )?;
                    Ok(ctx)
                })
            })?;
        let initial_pkgs =
            self.next_candidates(&initial_ctx, initial_ctx.clone(), depend_policy_fn)?;
        if let Ok(initial_pkgs) = initial_pkgs {
            stage_ctxs.push(initial_pkgs);
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
                    drop(stage_ctxs.pop().unwrap());
                    depth -= 1;
                    println!("rewinding to {}", depth);
                }
            }
            let partial_solution = partial_solutions.get(depth).unwrap().clone();
            if let Some(candidates) = stage_ctxs.get_mut(depth).unwrap().next() {
                let solution_found = partial_solution.is_superset(
                    candidates
                        .pkgs()
                        .map(|i| i.as_ref())
                        .collect_vec()
                        .as_slice(),
                ); // no new dependency, solution found

                let partial_solution = match partial_solution.union(candidates.clone()) {
                    Some(v) => v,
                    None => continue, // not accepted, try the next set of candidates
                };

                if solution_found {
                    return Ok(partial_solution);
                }

                // Current set of candidates accepted, start forming next stage

                println!("searching candidates");
                let next_candidates =
                    self.next_candidates(&candidates, partial_solution.clone(), depend_policy_fn);
                let next_candidates = if let Err(Error::DependencyError(_)) = next_candidates {
                    continue; // all solutions derived from current set of candidates will cause a conflict, try the next set of candidates
                } else {
                    next_candidates
                }?;
                let next_candidates = match next_candidates {
                    Ok(v) => v,
                    Err(ctx) => {
                        let partial_solution = partial_solution.union(ctx).unwrap();
                        return Ok(partial_solution)
                    }
                };

                depth += 1;
                println!("step into depth {}", depth);
                depth_try_count = 0;
                partial_solutions.push(partial_solution);
                stage_ctxs.push(next_candidates);
            } else {
                rewind = true; // current node depleted with no solution found. backtracking...
            }
        }
    }

    fn next_candidates<'a>(
        &'a self,
        candidates: &Context,
        partial_solution: Context,
        depend_policy_fn: impl Fn(&Package) -> DependPolicy,
    ) -> Result<std::result::Result<Box<dyn Iterator<Item = Context> + 'a>, Context>> {
        let mut base_ctx = candidates.clone();

        // get deps of all candidates and merge them
        // NOTE
        // There's a possibility that merging derives a suboptimal graph structure
        // e.g. candidates: A==1.0, B, C
        // B depends on A(any) and C depends on A > 2.0
        // This should resolves to A==1.0, B, A==3.0 -> C, but now it resolves to A==1.0, A==3.0 -> (B, C)
        let mut map_dep_parents: HashMap<Depend, Vec<Arc<Package>>> = candidates
            .pkgs()
            .fold(
                HashMap::new(),
                |mut acc: HashMap<String, (Depend, Vec<Arc<Package>>)>, x| {
                    let depend_policy = depend_policy_fn(x.as_ref());
                    if depend_policy.contains(DependChoice::Depends) {
                        x.depends()
                            .iter()
                            .filter(|dep| {
                                self.policy.skip_repo.find_package(dep).unwrap().is_empty()
                            }) // TODO error handling
                            .for_each(|dep| {
                                acc.entry(dep.name.clone())
                                    .and_modify(|(original_dep, pkgs)| {
                                        original_dep.version =
                                            original_dep.version.union(&dep.version.clone());
                                        pkgs.push(x.clone())
                                    })
                                    .or_insert((dep.clone(), vec![x.clone()]));
                            });
                    }
                    if depend_policy.contains(DependChoice::MakeDepends) {
                        x.make_depends()
                            .iter()
                            .filter(|dep| {
                                self.policy.skip_repo.find_package(dep).unwrap().is_empty()
                            }) // TODO error handling
                            .for_each(|dep| {
                                acc.entry(dep.name.clone())
                                    .and_modify(|(original_dep, pkgs)| {
                                        original_dep.version =
                                            original_dep.version.union(&dep.version.clone());
                                        pkgs.push(x.clone())
                                    })
                                    .or_insert((dep.clone(), vec![x.clone()]));
                            });
                    }
                    acc
                },
            )
            .into_values()
            .into_grouping_map()
            .fold_first(|mut acc, _, v| {
                acc.extend(v);
                acc
            });

        // exclude already satisfied deps
        map_dep_parents
            .drain_filter(|dep, requesting_pkgs| {
                let satisfied = partial_solution.satisfies(dep);
                if satisfied {
                    // TODO optimize this (add package info to provide set)
                    partial_solution
                        .pkgs()
                        .filter(|pkg| dep.satisfied_by(pkg))
                        .for_each(|existing_pkg| {
                            // need to update the install reason for existing package
                            base_ctx.append_reasons(
                                existing_pkg.clone(),
                                requesting_pkgs.iter().cloned().collect(),
                            );
                        });
                }
                satisfied
            })
            .for_each(drop);

        // the dep set itself conflicts with current solution, abort
        if map_dep_parents
            .keys()
            .any(|dep| partial_solution.conflicts(dep))
        {
            return Err(Error::DependencyError(DependencyError::ConflictDependency(
                String::from("new dependencies conflicts with previous partial solution"),
            )));
        }

        // maybe a dep of candidates is fulfilled by another candidate
        map_dep_parents
            .drain_filter(|dep, requesting_pkgs| {
                let satisfied = candidates.satisfies(dep);

                if satisfied {
                    // TODO optimize this (add package info to provide set)
                    candidates
                        .pkgs()
                        .filter(|pkg| dep.satisfied_by(pkg))
                        .for_each(|existing_pkg| {
                            // need to update the install reason for existing package
                            base_ctx.append_reasons(
                                existing_pkg.clone(),
                                requesting_pkgs.iter().cloned().collect(),
                            );
                        });
                }
                satisfied
            })
            .for_each(drop);

        // no new deps needed
        if map_dep_parents.is_empty() {
            return Ok(Err(base_ctx));
        }

        let cloned_policy = self.policy.clone(); // clone for closure use

        // Layout of `resolved_deps`:
        // [[solution1, solution2, ...]: dep1, dep2, ...]
        let resolved_deps = self
            .policy
            .from_repo
            .find_packages(&*map_dep_parents.keys().cloned().collect_vec())
            .map(move |i| {
                i.into_iter()
                    .sorted_by(|(_, a), (_, b)| b.len().cmp(&a.len())) // heuristic strategy: iter solution from packages with less deps
                    .map(|i| (i, cloned_policy.clone(), cloned_policy.clone())) // clone for closure use
                    .map(move |((dep, pkgs), cloned_policy, cloned_policy_2)| {
                        pkgs.into_iter()
                            .filter(move |pkg| !cloned_policy.is_mortal_blade(pkg).unwrap())
                            .sorted_by(|a, b| {
                                // let a = PackageNode::from(a);
                                // let b = PackageNode::from(b);
                                if partial_solution.contains_exact(a)          // prefer chosen packages
                                    || cloned_policy_2.is_immortal(a).unwrap()
                                // prefer immortal packages
                                {
                                    Ordering::Less
                                } else if partial_solution.contains_exact(b)
                                    || cloned_policy_2.is_immortal(b).unwrap()
                                {
                                    Ordering::Greater
                                } else {
                                    Ordering::Equal
                                }
                            })
                            .take(5) // limit search space
                            .map(move |pkg| (dep.clone(), Arc::new(pkg)))
                    })
                    .collect_vec()
            })?;

        // [solution1, solution2, ...]
        let next_candidates = resolved_deps
            .into_iter()
            .multi_cartesian_product()
            .filter_map(move |pkgs| {
                pkgs.into_iter()
                    .fold(Some(base_ctx.clone()), |acc, (dep, pkg)| {
                        acc.and_then(|acc| {
                            acc.insert(
                                pkg,
                                map_dep_parents.get(&dep).unwrap().iter().cloned().collect(),
                            )
                        })
                    })
            });
        Ok(Ok(Box::new(next_candidates)))
    }
}