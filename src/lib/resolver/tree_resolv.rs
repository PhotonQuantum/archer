use super::types::*;
use crate::error::{res_to_resop, DependencyError, Error};
use crate::repository::Repository;
use crate::types::*;
use fallible_iterator::{convert, FallibleIterator};
use futures::TryStreamExt;
use std::sync::{Arc, Mutex};
use itertools::{Product, Itertools};

type Solution = DepList<PackageWithParent>;

pub fn tree_resolve(base: Arc<Solution>, policy: Arc<ResolvePolicy>, pkg: &Depend, allow_cyclic: bool) -> ArcedIterator<Result<Solution>> {
    println!("solving for - {}", pkg.name);
    policy.from_repo.iter().map(
        |repo: &Arc<dyn Repository>| {
            match repo.find_package(&pkg.name){
                Ok(pkg) => {
                    let solution = Arc::new(Mutex::new(pkg.into_iter().map(PackageWithParent::from).map(|candidate|
                        if (*base).contains_exact(&candidate) {
                            println!("Great! {} is already satisfied.", candidate);
                            ArcedIterator::new(Arc::new(Mutex::new(vec![Ok((*base).clone())].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
                        } else if !base.is_compatible(&candidate) {
                            // ensure that this package is compatible with base
                            println!("However, {} conflicts with current solution", candidate);
                            ArcedIterator::new(Arc::new(Mutex::new(vec![].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
                        } else {
                            match convert(policy.immortal_repo.iter().map(Ok)).any(|repo: &Arc<dyn Repository>| {
                                Ok(repo
                                    .find_package(&candidate.name())?
                                    .into_iter()
                                    .any(|immortal| immortal.version() != candidate.version()))
                            }) {
                                Ok(conflict) => if conflict {
                                    // ensure that package won't conflict with immortal set (we won't need to uninstall any immortal package)
                                    println!("However, {} conflicts with immortal packages.", candidate);
                                    ArcedIterator::new(Arc::new(Mutex::new(vec![].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
                                } else {
                                    let dep_solutions: Vec<_> = candidate.dependencies().into_iter().map(|dep: Depend| {
                                        println!("Trying to solve dependency of {} - {}", candidate, dep);
                                        tree_resolve(base.clone(), policy.clone(), &dep, allow_cyclic)
                                    }).collect();
                                    let merged_dep_solution = dep_solutions.into_iter().multi_cartesian_product().map(|i| i.into_iter().fold(Ok((*base).clone()), |acc: Result<Solution>, x: Result<Solution>| {
                                        acc.and_then(|acc|x.and_then(|x|acc.union(x).ok_or(Error::NoneError)))
                                    })).filter(|solution|matches!(solution, Err(Error::NoneError)));
                                    ArcedIterator::new(Arc::new(Mutex::new(merged_dep_solution)) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
                                }
                                Err(e) => ArcedIterator::new(Arc::new(Mutex::new(vec![Err(e)].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
                            }
                        }).flatten())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>;
                    ArcedIterator::new(solution)
                },
                Err(e) => ArcedIterator::new(Arc::new(Mutex::new(vec![Err(e)].into_iter())) as Arc<Mutex<dyn Iterator<Item=Result<Solution>>>>)
            }.flatten()
        }
    ).flatten();
    todo!()
}