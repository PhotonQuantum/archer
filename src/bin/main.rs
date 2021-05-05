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
use archer_lib::resolver::types::ResolvePolicy;
use archer_lib::types::Depend;

fn main() -> Result<()> {
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
    let mut resolver = TreeResolver::new(policy, false);
    let initial_package = remote_repo
        .find_package(&Depend::from_str("com.tencent.meeting.deepin").unwrap())?
        .iter()
        .find(|p| p.name() == "com.tencent.meeting.deepin")
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
    let mut f = File::create("output.dot")?;
    let graph = Graph::from(&solution);
    write!(f, "{}", Dot::with_config(&graph, &[Config::EdgeNoLabel])).unwrap();
    Ok(())
}
