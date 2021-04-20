use alpm::{Alpm, SigLevel, AlpmListMut};
use anyhow::Result;
use archer_lib::alpm::AlpmBuilder;
use archer_lib::load_alpm;
use archer_lib::parser::PacmanParser;
use archer_lib::repository::pacman::{PacmanLocal, PacmanRemote};
use archer_lib::repository::{pacman, Repository};
use archer_lib::types::{OwnedPacmanPackage, Depend};
use archer_lib::resolver::types::{ResolvePolicy, DepList};
use std::sync::{Arc, Mutex};
use itertools::Itertools;
use archer_lib::resolver::tree_resolv::TreeResolver;
use std::thread;
use std::collections::HashSet;
use archer_lib::repository::aur::AurRepo;

fn main() -> Result<()> {
    let config = PacmanParser::with_default()?;
    let remote_repo = PacmanRemote::new();
    let local_repo = PacmanLocal::new();
    let aur = AurRepo::new();
    let policy = ResolvePolicy {
        from_repo: vec![Arc::new(Mutex::new(remote_repo)), Arc::new(Mutex::new(aur))],
        skip_repo: vec![Arc::new(Mutex::new(local_repo.clone()))],
        immortal_repo: vec![Arc::new(Mutex::new(local_repo))]
    };
    let builder = thread::Builder::new().name(String::from("electron")).stack_size(1*1024*1024);
    let handler = builder.spawn(move || {
        let mut resolver = TreeResolver::new(policy, true);
        let solution = resolver.resolve(DepList::new(), Depend::from_str("electron"), HashSet::new(), 0);
        return solution
    })?;
    let solution = handler.join().unwrap();
    println!("{:#?}", solution.into_iter().map(|sol|sol.map(|sol| sol.packages.values().map(|pkg|pkg.to_string()).join(", "))).collect_vec());
    Ok(())
}
