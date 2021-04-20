use alpm::Alpm;
use anyhow::Result;
use archer_lib::alpm::AlpmBuilder;
use archer_lib::load_alpm;
use archer_lib::parser::PacmanParser;
use archer_lib::repository::pacman::{PacmanLocal, PacmanRemote};
use archer_lib::repository::{pacman, Repository};
use archer_lib::types::{OwnedPacmanPackage, Depend};
use archer_lib::resolver::types::{ResolvePolicy, DepList};
use archer_lib::resolver::tree_resolv::tree_resolve;
use std::sync::{Arc, Mutex};
use itertools::Itertools;

fn main() -> Result<()> {
    let config = PacmanParser::with_default()?;
    let builder = AlpmBuilder::new(&config);
    let remote_repo = PacmanRemote::new();
    let builder = AlpmBuilder::new(&config);
    let local_repo = PacmanLocal::new();
    let policy = ResolvePolicy {
        from_repo: vec![Arc::new(Mutex::new(remote_repo))],
        skip_repo: vec![Arc::new(Mutex::new(local_repo.clone()))],
        immortal_repo: vec![Arc::new(Mutex::new(local_repo))]
    };
    let solution = tree_resolve(DepList::new(), policy, Depend::from_str("electron"), true);
    println!("{:#?}", solution.map(|sol|sol.map(|sol| sol.packages.values().map(|pkg|pkg.to_string()).join(", "))).collect_vec());
    Ok(())
}
