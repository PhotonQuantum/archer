#![deny(clippy::all)]

use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use itertools::Itertools;
use petgraph::dot::{Config, Dot};
use petgraph::Graph;

use archer_lib::prelude::*;

fn main() -> Result<()> {
    demo_planner()?;
    demo_deps()
}

fn demo_planner() -> Result<()> {
    let mut planner = PlanBuilder::new();
    println!("finding package");
    // planner.add_package(&Depend::from_str("firedragon").unwrap())?;
    planner.add_package(&Depend::from_str("fcft").unwrap())?;
    println!("building plan");
    let result = planner.build()?;
    println!(
        "Plan: {:#?}",
        result.into_iter().map(|act| act.to_string()).collect_vec()
    );
    Ok(())
}

fn demo_deps() -> Result<()> {
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
    let resolver = TreeResolver::new(policy);
    let initial_package = remote_repo
        .find_package(&Depend::from_str("electron").unwrap())?
        .iter()
        .find(|p| p.name() == "electron")
        .unwrap()
        .clone();
    let solution = resolver.resolve(&[initial_package], always_depend, allow_if_pacman)?;
    println!(
        "{} packages: \n{:#?}",
        solution.packages.len(),
        solution
            .strongly_connected_components()
            .into_iter()
            .map(|pkgs| format!("[{}]", pkgs.iter().map(|pkg| pkg.to_string()).join(", ")))
            .join(", ")
    );
    let mut f = File::create("output.dot")?;
    let graph = Graph::from(&solution);
    write!(f, "{}", Dot::with_config(&graph, &[Config::EdgeNoLabel])).unwrap();
    Ok(())
}
