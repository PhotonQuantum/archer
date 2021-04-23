use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use itertools::Itertools;

use archer_lib::repository::aur::AurRepo;
use archer_lib::repository::cached::CachedRepository;
use archer_lib::repository::merged::MergedRepository;
use archer_lib::repository::pacman::{PacmanLocal, PacmanRemote};
use archer_lib::repository::Repository;
use archer_lib::resolver::tree_resolv::TreeResolver;
use archer_lib::resolver::types::ResolvePolicy;
use archer_lib::types::Depend;

fn main() -> Result<()> {
    let remote_repo = Arc::new(Mutex::new(PacmanRemote::new())) as Arc<Mutex<dyn Repository>>;
    let local_repo = Arc::new(Mutex::new(PacmanLocal::new())) as Arc<Mutex<dyn Repository>>;
    let aur = Arc::new(Mutex::new(AurRepo::new())) as Arc<Mutex<dyn Repository>>;
    let remote_repo = Arc::new(Mutex::new(CachedRepository::new(MergedRepository::new(
        vec![remote_repo.clone(), aur],
    ))));
    let policy = ResolvePolicy::new(
        remote_repo.clone(),
        Arc::new(Mutex::new(CachedRepository::new(MergedRepository::new(
            vec![local_repo.clone()],
        )))),
        Arc::new(Mutex::new(CachedRepository::new(MergedRepository::new(
            vec![local_repo],
        ))))
    );
    let mut resolver = TreeResolver::new(policy, false);
    let initial_package = remote_repo
        .lock()
        .unwrap()
        .find_package(&Depend::from_str("crossover").unwrap())?
        .iter()
        .find(|p| p.name() == "crossover")
        .unwrap()
        .clone();
    let solution = resolver.resolve(&[initial_package])?;
    println!(
        "{} packages: \n{:#?}",
        solution.packages.len(),
        solution
            .packages
            .values()
            .map(|pkg| pkg.to_string())
            .join(", ")
    );
    Ok(())
}
