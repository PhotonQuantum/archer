use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use itertools::Itertools;

use crate::error::{DependencyError, Error};
use crate::types::*;

use super::types::*;

type Solution = DepList<PackageWithParent>;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct CacheUnit {
    base: Solution,
    dep: Depend,
}

#[derive(Clone)]
pub struct TreeResolver {
    policy: ResolvePolicy,
    allow_cyclic: bool,
    max_depth: u32,
    cache: HashMap<CacheUnit, Vec<Result<Solution>>>,
    visited: HashSet<Depend>,
}

impl TreeResolver {
    pub fn new(policy: ResolvePolicy, allow_cyclic: bool) -> Self {
        TreeResolver {
            policy,
            allow_cyclic,
            max_depth: 100,
            cache: HashMap::new(),
            visited: Default::default(),
        }
    }

    pub fn initialize(&mut self) {
        self.visited.clear()
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear()
    }

    pub fn resolve(
        &mut self,
        base: Solution,
        pkg: Package,
        mut visited: HashSet<Depend>,
        cur_depth: u32,
    ) -> Vec<Result<Solution>> {
        // recursion guard
        if cur_depth > self.max_depth {
            return vec![Err(Error::RecursionError)];
        }

        let dep = Depend::from(&pkg);
        // detect cyclic dependency
        if visited.contains(&dep) {
            return if !self.allow_cyclic {
                // e.g. aur package building
                vec![Err(Error::DependencyError(DependencyError::CyclicDependency))]
            } else {
                // e.g. installing pacman packages
                println!("cyclic dependency detected.");
                vec![Ok(base)]
            };
        }
        visited.insert(dep.clone());

        // try to fetch solution from cache
        let cache_unit = CacheUnit {
            base: base.clone(),
            dep,
        };
        if let Some(cached_solution) = self.cache.get(&cache_unit) {
            return cached_solution.clone();
        }

        // println!("solving for - {} - {}", pkg.name, cur_depth);
        let pkg = PackageWithParent::from(pkg);
        let result = if base.contains_exact(&pkg) {
            println!("Great! {} is already satisfied.", pkg);
            vec![Ok(base)]
        } else if !base.is_compatible(&pkg) {
            // ensure that this package is compatible with base
            println!("However, {} conflicts with current solution", pkg);
            vec![]
        } else {
            let conflict = match self
                .policy
                .immortal_repo
                .lock()
                .unwrap()
                .find_package(&Depend::from(&pkg))
                .map(|immortals| {
                    immortals
                        .into_iter()
                        .any(|immortal| immortal.version() != pkg.version())
                }) {
                Ok(conflict) => conflict,
                Err(e) => return vec![Err(e)],
            };
            if conflict {
                // ensure that package won't conflict with immortal set (we won't need to uninstall any immortal package)
                println!("However, {} conflicts with immortal packages.", pkg);
                vec![]
            } else {
                let resolved_deps = match self
                    .policy
                    .from_repo
                    .lock()
                    .unwrap()
                    .find_packages(&*pkg.dependencies())
                {
                    Ok(deps) => deps,
                    Err(e) => return vec![Err(e)],
                };
                // Layout of `dep_solutions`:
                // [[solution1, solution2, ...]: dep1, dep2, ...]
                let mut dep_solutions = resolved_deps
                    .into_iter()
                    .map(|(_, pkgs)| {
                        // println!("Trying to solve dependency of {} - {}", pkg, dep);
                        pkgs
                            .into_iter()
                            .take(5)    // there might be multiple packages satisfying dependency. choose at most 5 candidates.
                            .map(|pkg| self.resolve(base.clone(), pkg, visited.clone(), cur_depth + 1))
                            .map(|sol|sol.into_iter().take(3))  // take at most 3 solutions per candidate
                            // TODO maybe shuffle to take solutions?
                            .flatten()  // we don't care about which candidate each solution is yielded from.
                            .collect_vec()
                    })
                    .collect_vec();
                if dep_solutions.is_empty() {
                    // there's no dependency. add an empty solution set to collection.
                    dep_solutions.push(vec![Ok(DepList::new())]);
                }
                let merged_dep_solution = dep_solutions
                    .into_iter()
                    .multi_cartesian_product()  // select one solution per dependency and form a set each iter
                    .map(move |i| {
                        i.into_iter()
                            .fold(Ok(base.clone()), |acc: Result<Solution>, x: Result<Solution>| {
                                // println!("merging {:?} and {:?}", acc, x);
                                acc.and_then(|acc| x.and_then(|x| {
                                    acc.union(x).ok_or(Error::NoneError)} // drop incompatible solution sets
                                ))
                            })
                    })
                    .filter(|solution| !(matches!(solution, Err(Error::NoneError))));   // remove invalidated solution sets
                let candidate = Arc::new(Box::new(pkg));
                let final_solution = merged_dep_solution.map(move |solution| {
                    solution.map(|mut solution| {
                        solution.insert_mut(candidate.clone());
                        solution
                    })
                });
                final_solution.take(5).collect_vec()
                // beam width: 5
            }
        };
        self.cache.insert(cache_unit, result.clone());
        result
    }
}
