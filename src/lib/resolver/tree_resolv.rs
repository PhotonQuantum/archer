use super::types::*;
use crate::error::{res_to_resop, DependencyError, Error};
use crate::repository::Repository;
use crate::types::*;
use fallible_iterator::{convert, FallibleIterator};
use futures::TryStreamExt;
use std::sync::{Arc, Mutex};
use itertools::{Product, Itertools};
use std::collections::{HashMap, HashSet};

type Solution = DepList<PackageWithParent>;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct CacheUnit {
    base: Solution,
    dep: Depend
}

#[derive(Clone)]
pub struct TreeResolver {
    policy: ResolvePolicy,
    allow_cyclic: bool,
    cache: HashMap<CacheUnit, Vec<Result<Solution>>>,
    visited: HashSet<Depend>
}

impl TreeResolver {
    pub fn new(policy: ResolvePolicy, allow_cyclic: bool) -> Self {
        TreeResolver { policy, allow_cyclic, cache: HashMap::new(), visited: Default::default() }
    }

    pub fn initialize(&mut self) {
        self.visited.clear()
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear()
    }

    pub fn resolve(&mut self, base: Solution, pkg: Depend, mut visited: HashSet<Depend>, cur_depth: u64) -> Vec<Result<Solution>> {
        // println!("solving for - {} - {}", pkg.name, cur_depth);
        if visited.contains(&pkg) {
            if !self.allow_cyclic {
                vec![Err(Error::DependencyError(DependencyError::CyclicDependency))]
            } else {
                println!("cyclic dependency detected.");
                vec![Ok(base)]
            }
        } else {
            visited.insert(pkg.clone());
            let cache_unit = CacheUnit { base: base.clone(), dep: pkg.clone() };
            if let Some(cached_solution) = self.cache.get(&cache_unit) {
                return cached_solution.clone();
            }
            let result = self.policy.from_repo.clone().into_iter().map(
                |repo| {
                    let found_package = {
                        repo.lock().unwrap().find_package(&pkg.name)
                    };
                    match found_package {
                        Ok(pkgs) => {
                            let solution = pkgs.into_iter().map(PackageWithParent::from).map(|candidate|
                                if base.contains_exact(&candidate) {
                                    println!("Great! {} is already satisfied.", candidate);
                                    vec![Ok(base.clone())]
                                } else if !base.is_compatible(&candidate) {
                                    // ensure that this package is compatible with base
                                    println!("However, {} conflicts with current solution", candidate);
                                    vec![]
                                } else {
                                    match convert(self.policy.immortal_repo.iter_mut().map(Ok)).any(|repo: &mut Arc<Mutex<dyn Repository + Send>>| {
                                        Ok(repo
                                            .lock()
                                            .unwrap()
                                            .find_package(&candidate.name())?
                                            .into_iter()
                                            .any(|immortal| immortal.version() != candidate.version()))
                                    }) {
                                        Ok(conflict) => if conflict {
                                            // ensure that package won't conflict with immortal set (we won't need to uninstall any immortal package)
                                            println!("However, {} conflicts with immortal packages.", candidate);
                                            vec![]
                                        } else {
                                            let mut dep_solutions: Vec<_> = candidate.dependencies().into_iter().map(|dep: Depend| {
                                                // println!("Trying to solve dependency of {} - {}", candidate, dep);
                                                self.resolve(base.clone(), dep, visited.clone(), cur_depth + 1)
                                            }).collect();
                                            if dep_solutions.is_empty() {
                                                dep_solutions.push(vec![Ok(DepList::new())]);
                                            }
                                            let base = base.clone();
                                            let merged_dep_solution = dep_solutions.into_iter().multi_cartesian_product().map(move |i| i.into_iter().fold(Ok(base.clone()), |acc: Result<Solution>, x: Result<Solution>| {
                                                // println!("merging {:?} and {:?}", acc, x);
                                                acc.and_then(|acc| x.and_then(|x| acc.union(x).ok_or(Error::NoneError)))
                                            })).filter(|solution| !(matches!(solution, Err(Error::NoneError))));
                                            let candidate = Arc::new(Box::new(candidate));
                                            let final_solution = merged_dep_solution.map(move |solution| solution.map(|mut solution| {
                                                solution.insert_mut(candidate.clone());
                                                solution
                                            }));
                                            final_solution.take(1).collect_vec()  // beam width: 3
                                        }
                                        Err(e) => vec![Err(e)]
                                    }
                                }).flatten().collect_vec();
                            solution
                        }
                        Err(e) => vec![Err(e)]
                    }
                }
            ).flatten().collect_vec();
            self.cache.insert(cache_unit, result.clone());
            result
        }
    }
}
