use super::types::*;
use crate::error::{res_to_resop, DependencyError, Error};
use crate::repository::Repository;
use crate::types::*;
use fallible_iterator::{convert, FallibleIterator};
use futures::TryStreamExt;
use std::sync::{Arc, Mutex};
use itertools::{Product, Itertools};

type Solution = DepList<PackageWithParent>;

pub fn tree_resolve(base: Solution, policy: ResolvePolicy, pkg: Depend, allow_cyclic: bool) -> ArcedIterator<Result<Solution>> {
    println!("solving for - {}", pkg.name);
    let bases = policy.from_repo.iter().map(|_|base.clone()).collect_vec();
    let policies = policy.from_repo.iter().map(|_|policy.clone()).collect_vec();
    let base_policy_fr = policy.from_repo.clone();
    ArcedIterator::new(Arc::new(Mutex::new(base_policy_fr.into_iter().enumerate().map(move |(i, repo)|(bases[i].clone(), policies[i].clone(), repo)).map(
        move |(base, mut policy, mut repo)| {
            let found_package = {
                repo.lock().unwrap().find_package(&pkg.name)
            };
            match found_package{
                Ok(pkg) => {
                    let base = base.clone();
                    let solution = Arc::new(Mutex::new(pkg.into_iter().map(PackageWithParent::from).map(move |candidate|
                        if base.contains_exact(&candidate) {
                            println!("Great! {} is already satisfied.", candidate);
                            ArcedIterator::new(Arc::new(Mutex::new(vec![Ok(base.clone())].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
                        } else if !base.is_compatible(&candidate) {
                            // ensure that this package is compatible with base
                            println!("However, {} conflicts with current solution", candidate);
                            ArcedIterator::new(Arc::new(Mutex::new(vec![].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
                        } else {
                            match convert(policy.immortal_repo.iter_mut().map(Ok)).any(|repo: &mut Arc<Mutex<dyn Repository>>| {
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
                                    ArcedIterator::new(Arc::new(Mutex::new(vec![].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
                                } else {
                                    let mut dep_solutions: Vec<_> = candidate.dependencies().into_iter().map(|dep: Depend| {
                                        println!("Trying to solve dependency of {} - {}", candidate, dep);
                                        tree_resolve(base.clone(), policy.clone(), dep, allow_cyclic)
                                    }).collect();
                                    if dep_solutions.is_empty() {
                                        dep_solutions.push(ArcedIterator::new(Arc::new(Mutex::new(vec![Ok(DepList::new())].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>));
                                    }
                                    println!("Dependency solving yields solutions {} for {}", dep_solutions.len(), candidate);
                                    let base = base.clone();
                                    let merged_dep_solution = dep_solutions.into_iter().multi_cartesian_product().map(move |i| i.into_iter().fold(Ok(base.clone()), |acc: Result<Solution>, x: Result<Solution>| {
                                        // println!("merging {:?} and {:?}", acc, x);
                                        acc.and_then(|acc|x.and_then(|x|acc.union(x).ok_or(Error::NoneError)))
                                    })).filter(|solution|!(matches!(solution, Err(Error::NoneError))));
                                    let candidate = Arc::new(Box::new(candidate.clone()));
                                    let final_solution = merged_dep_solution.map(move |solution|solution.map(|solution|solution.insert(candidate.clone()).unwrap()));
                                    ArcedIterator::new(Arc::new(Mutex::new(final_solution)) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
                                }
                                Err(e) => ArcedIterator::new(Arc::new(Mutex::new(vec![Err(e)].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
                            }
                        }).flatten())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>;
                    ArcedIterator::new(solution)
                },
                Err(e) => ArcedIterator::new(Arc::new(Mutex::new(vec![Err(e)].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
            }
        }
    ).flatten())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
}