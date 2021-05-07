use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use itertools::Itertools;
use petgraph::dot::{Config, Dot};
use petgraph::Graph;

use archer_lib::repository::aur::AurRepo;
use archer_lib::repository::cached::CachedRepository;
use archer_lib::repository::empty::EmptyRepository;
use archer_lib::repository::merged::MergedRepository;
use archer_lib::repository::pacman::{PacmanLocal, PacmanRemote};
use archer_lib::repository::Repository;
use archer_lib::resolver::tree_resolv::TreeResolver;
use archer_lib::resolver::types::{ResolvePolicy, DependChoice, always_depend};
use archer_lib::types::Depend;
use enumflags2::BitFlags;
use archer_lib::resolver::planner::PlanBuilder;

fn main() -> Result<()> {
    let mut planner = PlanBuilder::new();
    println!("finding agda");
    // planner.add_package(&Depend::from_str("firedragon").unwrap())?;
    planner.add_package(&Depend::from_str("fcft").unwrap())?;
    println!("building plan");
    let result = planner.build()?;
    println!("Plan: {:#?}", result.into_iter().map(|act|act.to_string()).collect_vec());
    return Ok(());

    let pacman_remote_repo = Arc::new(PacmanRemote::new()) as Arc<dyn Repository>;
    let local_repo = Arc::new(PacmanLocal::new()) as Arc<dyn Repository>;
    let aur = Arc::new(AurRepo::new()) as Arc<dyn Repository>;
    let remote_repo = Arc::new(CachedRepository::new(Arc::new(MergedRepository::new(
        vec![pacman_remote_repo.clone(), aur],
    ))));
    let policy = ResolvePolicy::new(
        remote_repo.clone(),
        Arc::new(EmptyRepository::new()),
        // Arc::new(CachedRepository::new(pacman_remote_repo)),
        // Arc::new(EmptyRepository::new()),
        Arc::new(CachedRepository::new(local_repo)),
    );
    let resolver = TreeResolver::new(policy, false);
    let initial_package = remote_repo
        .find_package(&Depend::from_str("m4").unwrap())?
        .iter()
        .find(|p| p.name() == "m4")
        .unwrap()
        .clone();
    let solution = resolver.resolve(&[initial_package], always_depend)?;
    println!(
        "{} packages: \n{:#?}",
        solution.packages.len(),
        solution
            .packages
            .values()
            .map(|pkg| pkg.to_string())
            .join(", ")
    );
    let mut f = File::create("output.dot")?;
    let graph = Graph::from(&solution);
    write!(f, "{}", Dot::with_config(&graph, &[Config::EdgeNoLabel])).unwrap();
    Ok(())
}
