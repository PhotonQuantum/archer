use alpm::{Alpm, AlpmListMut, SigLevel};
use anyhow::Result;
use archer_lib::alpm::AlpmBuilder;
use archer_lib::load_alpm;
use archer_lib::parser::PacmanParser;
use archer_lib::repository::aur::AurRepo;
use archer_lib::repository::pacman::{PacmanLocal, PacmanRemote};
use archer_lib::repository::{pacman, Repository};
use archer_lib::resolver::tree_resolv::TreeResolver;
use archer_lib::resolver::types::{DepList, ResolvePolicy};
use archer_lib::types::{Depend, OwnedPacmanPackage};
use itertools::Itertools;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> Result<()> {
    let config = PacmanParser::with_default()?;
    let remote_repo = PacmanRemote::new();
    let local_repo = PacmanLocal::new();
    let aur = AurRepo::new();
    let policy = ResolvePolicy {
        from_repo: vec![Arc::new(Mutex::new(remote_repo)), Arc::new(Mutex::new(aur))],
        skip_repo: vec![Arc::new(Mutex::new(local_repo.clone()))],
        immortal_repo: vec![Arc::new(Mutex::new(local_repo))],
    };
    // let builder = thread::Builder::new().name(String::from("ros-lunar-desktop")).stack_size(1*1024*1024);
    // let handler = builder.spawn(move || {
    let mut resolver = TreeResolver::new(policy, true);
    let solution = resolver.resolve(
        DepList::new(),
        Depend::from_str("electron"),
        HashSet::new(),
        0,
    );
    // })?;
    // let solution = handler.join().unwrap();
    println!(
        "{:#?}",
        solution
            .into_iter()
            .map(|sol| sol.map(|sol| sol.packages.values().map(|pkg| pkg.to_string()).join(", ")))
            .collect_vec()
    );
    Ok(())
}
