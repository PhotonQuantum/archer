use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use itertools::Itertools;

use archer_lib::repository::aur::AurRepo;
use archer_lib::repository::pacman::{PacmanLocal, PacmanRemote};
use archer_lib::resolver::tree_resolv::TreeResolver;
use archer_lib::resolver::types::{DepList, ResolvePolicy};
use archer_lib::types::Depend;

fn main() -> Result<()> {
    let remote_repo = PacmanRemote::new();
    let local_repo = PacmanLocal::new();
    let aur = AurRepo::new();
    let policy = ResolvePolicy {
        from_repo: vec![Arc::new(Mutex::new(remote_repo)), Arc::new(Mutex::new(aur))],
        skip_repo: vec![Arc::new(Mutex::new(local_repo.clone()))],
        immortal_repo: vec![Arc::new(Mutex::new(local_repo))],
    };
    let mut resolver = TreeResolver::new(policy, true);
    let solution = resolver.resolve(
        DepList::new(),
        Depend::from_str("electron"),
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
