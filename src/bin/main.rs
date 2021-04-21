use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use itertools::Itertools;

use archer_lib::repository::aur::AurRepo;
use archer_lib::repository::cached::CachedRepository;
use archer_lib::repository::pacman::{PacmanLocal, PacmanRemote};
use archer_lib::resolver::tree_resolv::TreeResolver;
use archer_lib::resolver::types::{DepList, ResolvePolicy};
use archer_lib::types::Depend;
use archer_lib::repository::Repository;
use archer_lib::repository::merged::MergedRepository;

fn main() -> Result<()> {
    let remote_repo = Arc::new(Mutex::new(PacmanRemote::new())) as Arc<Mutex<dyn Repository>>;
    let local_repo = Arc::new(Mutex::new(PacmanLocal::new())) as Arc<Mutex<dyn Repository>>;
    let aur = Arc::new(Mutex::new(AurRepo::new())) as Arc<Mutex<dyn Repository>>;
    let remote_repo = Arc::new(Mutex::new(CachedRepository::new(MergedRepository::new(vec![remote_repo.clone(), aur]))));
    let policy = ResolvePolicy {
        from_repo: remote_repo.clone(),
        skip_repo: Arc::new(Mutex::new(CachedRepository::new(MergedRepository::new(vec![local_repo.clone()])))),
        immortal_repo: Arc::new(Mutex::new(CachedRepository::new(MergedRepository::new(vec![local_repo])))),
    };
    let mut resolver = TreeResolver::new(policy, true);
    let initial_package = remote_repo.lock().unwrap().find_package(&Depend::from_str("python2-pillow").unwrap())?.pop().unwrap();
    let solution = resolver.resolve(
        DepList::new(),
        initial_package,
        HashSet::new(),
        0,
    );
    println!(
        "{:#?}",
        solution
            .into_iter()
            .map(|sol| sol.map(|sol| sol.packages.values().map(|pkg| pkg.to_string()).join(", ")))
            .collect_vec()
    );
    Ok(())
}
