use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use itertools::Itertools;
use maplit::hashset;

use crate::error::Result;
use crate::error::{DependencyError, Error};
use crate::types::*;

use super::types::*;

type CtxWithCycles = (Context, Vec<Vec<ArcPackage>>);

pub struct TreeResolver {
    resolve_policy: ResolvePolicy,
    depend_policy: Box<dyn Fn(&Package) -> DependPolicy>,
    cyclic_policy: Box<dyn Fn(&[&Package]) -> bool>,
}

enum Candidate<'a> {
    Continue(Box<dyn Iterator<Item = CtxWithCycles> + 'a>),
    Finish((Box<Context>, Option<Vec<ArcPackage>>)),
}

impl TreeResolver {
    #[must_use]
    pub fn new(
        resolve_policy: ResolvePolicy,
        depend_policy: Box<dyn Fn(&Package) -> DependPolicy>,
        cyclic_policy: Box<dyn Fn(&[&Package]) -> bool>,
    ) -> Self {
        Self {
            resolve_policy,
            depend_policy,
            cyclic_policy,
        }
    }

    fn insert_into_ctx(
        &self,
        mut ctx: Context,
        pkg: &ArcPackage,
        reason: HashSet<ArcPackage>,
    ) -> Result<Option<CtxWithCycles>> {
        Ok(self
            .insert_into_ctx_mut(&mut ctx, pkg, reason)?
            .map(|maybe_cycle| {
                (
                    ctx,
                    maybe_cycle
                        .map(|component| vec![component])
                        .unwrap_or_default(),
                )
            }))
    }

    fn insert_into_ctx_mut(
        &self,
        ctx: &mut Context,
        pkg: &ArcPackage,
        reason: HashSet<ArcPackage>,
    ) -> Result<Option<MaybeCycle>> {
        if self.resolve_policy.is_mortal_blade(&*pkg)? {
            Err(Error::DependencyError(DependencyError::ConflictDependency(
                String::from("conflict with immortal package"),
            )))
        } else {
            Ok(ctx.insert_mut(pkg, reason))
        }
    }

    pub fn resolve(&self, pkgs: &[Package]) -> Result<Context> {
        let mut stage_ctxs: Vec<Box<dyn Iterator<Item = CtxWithCycles>>> = vec![];
        let mut depth = 0;

        // push initial set
        let initial_ctx = self.context_from_pkgs(pkgs)?;
        let initial_pkgs = self.next_candidates(&initial_ctx, initial_ctx.clone())?;
        if let Candidate::Continue(initial_pkgs) = initial_pkgs {
            stage_ctxs.push(initial_pkgs);
        } else {
            return Ok(initial_ctx);
        }
        let mut partial_solutions = vec![initial_ctx];

        let mut depth_try_count = 0_u32;
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
                }
                partial_solutions.pop().unwrap();
                drop(stage_ctxs.pop().unwrap());
                depth -= 1;
                println!("rewinding to {}", depth);
            }
            let partial_solution = partial_solutions.get(depth).unwrap().clone();
            if let Some((candidates, maybe_cycle)) = stage_ctxs.get_mut(depth).unwrap().next() {
                if !maybe_cycle.is_empty() {
                    println!("cycle detected, question");
                }
                if !maybe_cycle.is_empty()
                    && maybe_cycle.into_iter().all(|cycle| {
                        !(self.cyclic_policy)(&cycle.iter().map(AsRef::as_ref).collect_vec())
                    })
                {
                    // TODO error report
                    // return Err(Error::DependencyError(DependencyError::CyclicDependency(cycle)))
                    println!("cycle detected, reject");
                    continue; // cycle detected, try the next set of candidates
                }

                let solution_found = partial_solution.is_superset(
                    candidates
                        .pkgs()
                        .map(AsRef::as_ref)
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
                let next_candidates = self.next_candidates(&candidates, partial_solution.clone());
                let next_candidates = if let Err(Error::DependencyError(_)) = next_candidates {
                    continue; // all solutions derived from current set of candidates will cause a conflict, try the next set of candidates
                } else {
                    next_candidates
                }?;
                let next_candidates = match next_candidates {
                    Candidate::Continue(v) => v,
                    Candidate::Finish((ctx, maybe_cycle)) => {
                        if let Some(cycle) = maybe_cycle {
                            if !(self.cyclic_policy)(&cycle.iter().map(AsRef::as_ref).collect_vec())
                            {
                                continue;
                            }
                        }
                        let partial_solution = partial_solution.union(*ctx).unwrap();
                        return Ok(partial_solution);
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

    fn context_from_pkgs(&self, pkgs: &[Package]) -> Result<Context> {
        pkgs.iter()
            .filter(|pkg| {
                self.resolve_policy
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
                    let (ctx, _) = self.insert_into_ctx(ctx, &x, hashset!())?.ok_or(
                        Error::DependencyError(DependencyError::ConflictDependency(name)),
                    )?;
                    Ok(ctx)
                })
            })
    }

    fn merge_depends<'a>(
        &'a self,
        mut depends: HashMap<String, (Depend, Vec<ArcPackage>)>,
        new_depends: impl Iterator<Item = &'a Depend>,
        pkg: &ArcPackage,
    ) -> HashMap<String, (Depend, Vec<ArcPackage>)> {
        new_depends
            .filter(|dep| {
                self.resolve_policy
                    .skip_repo
                    .find_package(dep)
                    .unwrap()
                    .is_empty()
            }) // TODO error handling
            .for_each(|dep| {
                depends
                    .entry(dep.name.clone())
                    .and_modify(|(original_dep, pkgs)| {
                        original_dep.version = original_dep.version.union(&dep.version.clone());
                        pkgs.push(pkg.clone());
                    })
                    .or_insert((dep.clone(), vec![pkg.clone()]));
            });
        depends
    }

    fn next_candidates<'a>(
        &'a self,
        candidates: &Context,
        partial_solution: Context,
    ) -> Result<Candidate<'a>> {
        let base_ctx = candidates.clone();

        // get deps of all candidates and merge them
        // NOTE
        // There's a possibility that merging derives a suboptimal graph structure
        // e.g. candidates: A==1.0, B, C
        // B depends on A(any) and C depends on A > 2.0
        // This should resolves to A==1.0, B, A==3.0 -> C, but now it resolves to A==1.0, A==3.0 -> (B, C)
        let mut map_dep_parents: HashMap<Depend, Vec<ArcPackage>> = candidates
            .pkgs()
            .fold(
                HashMap::new(),
                |mut acc: HashMap<String, (Depend, Vec<ArcPackage>)>, x| {
                    let depend_policy = (self.depend_policy)(x.as_ref());
                    if depend_policy.contains(DependChoice::Depends) {
                        acc = self.merge_depends(acc, x.depends().iter(), x);
                    }
                    if depend_policy.contains(DependChoice::MakeDepends) {
                        acc = self.merge_depends(acc, x.make_depends().iter(), x);
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
        let (base_ctx, maybe_cycle) = TreeResolver::exclude_satisfied_deps(
            &mut map_dep_parents,
            base_ctx,
            &partial_solution,
            None,
        );

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
        let (base_ctx, maybe_cycle) = TreeResolver::exclude_satisfied_deps(
            &mut map_dep_parents,
            base_ctx,
            candidates,
            maybe_cycle,
        );

        // no new deps needed
        if map_dep_parents.is_empty() {
            return Ok(Candidate::Finish((Box::new(base_ctx), maybe_cycle)));
        }

        let cloned_policy = self.resolve_policy.clone(); // clone for closure use

        // Layout of `resolved_deps`:
        // [[solution1, solution2, ...]: dep1, dep2, ...]
        let resolved_deps = self
            .resolve_policy
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
                                Self::sort_candidates(&partial_solution, &cloned_policy_2, a, b)
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
                Self::merge_pkgs_into_ctx(pkgs, &base_ctx, &map_dep_parents, &maybe_cycle)
            });
        Ok(Candidate::Continue(
            Box::new(next_candidates) as Box<dyn Iterator<Item = CtxWithCycles> + 'a>
        ))
    }

    fn merge_pkgs_into_ctx(
        pkgs: Vec<(Depend, ArcPackage)>,
        base_ctx: &Context,
        map_dep_parents: &HashMap<Depend, Vec<ArcPackage>>,
        maybe_cycle: &MaybeCycle,
    ) -> Option<CtxWithCycles> {
        pkgs.into_iter().fold(
            Some((
                base_ctx.clone(),
                maybe_cycle
                    .clone()
                    .map(|cycle| vec![cycle])
                    .unwrap_or_default(),
            )),
            |acc, (dep, pkg)| {
                acc.and_then(|(ctx, mut maybe_cycle)| {
                    ctx.insert(
                        &pkg,
                        map_dep_parents.get(&dep).unwrap().iter().cloned().collect(),
                    )
                    .map(|(ctx, maybe_cycle_new)| {
                        if let Some(cycle) = maybe_cycle_new {
                            maybe_cycle.push(cycle);
                        }
                        (ctx, maybe_cycle)
                    })
                })
            },
        )
    }

    fn sort_candidates(
        partial_solution: &Context,
        resolve_policy: &ResolvePolicy,
        a: &Package,
        b: &Package,
    ) -> Ordering {
        if partial_solution.contains_exact(a)          // prefer chosen packages
            || resolve_policy.is_immortal(a).unwrap()
        // prefer immortal packages
        {
            Ordering::Less
        } else if partial_solution.contains_exact(b) || resolve_policy.is_immortal(b).unwrap() {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }

    fn exclude_satisfied_deps(
        map_dep_parents: &mut HashMap<Depend, Vec<ArcPackage>>,
        base_ctx: Context,
        satisfied_by_ctx: &Context,
        maybe_cycle: MaybeCycle,
    ) -> (Context, MaybeCycle) {
        let (base_ctx, maybe_cycle) = map_dep_parents
            .drain_filter(|dep, _| satisfied_by_ctx.satisfies(dep))
            .fold(
                (base_ctx, maybe_cycle),
                |(ctx, maybe_cycle), (dep, requesting_pkgs)| {
                    let (ctx, new_cycle) =
                        Self::satisfied_dep_effect(ctx, satisfied_by_ctx, &dep, &requesting_pkgs);
                    (
                        ctx,
                        if maybe_cycle.is_none() {
                            new_cycle
                        } else {
                            maybe_cycle
                        },
                    )
                },
            );
        (base_ctx, maybe_cycle)
    }

    fn satisfied_dep_effect(
        base_ctx: Context,
        satisfied_by_ctx: &Context,
        dep: &Depend,
        requesting_pkgs: &[ArcPackage],
    ) -> (Context, MaybeCycle) {
        // TODO optimize this (add package info to provide set)
        satisfied_by_ctx
            .pkgs()
            .filter(|pkg| dep.satisfied_by(pkg))
            .fold(
                (base_ctx, None),
                |(mut ctx, mut maybe_cycle), existing_pkg| {
                    // need to update the install reason for existing package
                    for pkg in requesting_pkgs.iter().cloned() {
                        let effect = ctx.add_edge(&pkg, existing_pkg).unwrap();
                        if let EdgeEffect::NewEdge(Some(cycle)) = effect {
                            maybe_cycle = Some(cycle);
                        };
                    }
                    (ctx, maybe_cycle)
                },
            )
    }
}
