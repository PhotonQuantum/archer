use std::str::FromStr;
use std::sync::Arc;

use archer_lib::repository::aur::AurRepo;
use archer_lib::repository::cached::CachedRepository;
use archer_lib::repository::empty::EmptyRepository;
use archer_lib::repository::merged::MergedRepository;
use archer_lib::repository::pacman::{PacmanLocal, PacmanRemote};
use archer_lib::repository::Repository;
use archer_lib::resolver::planner::PlanBuilder;
use archer_lib::resolver::tree_resolv::TreeResolver;
use archer_lib::resolver::types::{always_depend, ResolvePolicy};
use archer_lib::types::*;

use rstest::rstest;
use itertools::Itertools;

fn must_plan(pkg: &str) {
    println!("Planning {}", pkg);
    let mut planner = PlanBuilder::new();
    planner.add_package(&Depend::from_str(pkg).unwrap()).expect("can't search package");
    let plan = planner.build().expect("can't build plan");
    assert!(!plan.is_empty(), "plan is empty");
    println!("Plan: {:#?}", plan.iter().map(|act|act.to_string()).collect_vec());
}

fn must_resolve(pkg: &str, skip_remote: bool) {
    println!("Resolving {}", pkg);
    let pacman_remote_repo = Arc::new(PacmanRemote::new()) as Arc<dyn Repository>;
    let local_repo = Arc::new(PacmanLocal::new()) as Arc<dyn Repository>;
    let aur = Arc::new(AurRepo::new()) as Arc<dyn Repository>;
    let remote_repo = Arc::new(CachedRepository::new(Arc::new(MergedRepository::new(
        vec![pacman_remote_repo.clone(), aur],
    ))));
    let policy = ResolvePolicy::new(
        remote_repo.clone(),
        if skip_remote { Arc::new(CachedRepository::new(pacman_remote_repo)) } else { Arc::new(EmptyRepository::new()) },
        Arc::new(CachedRepository::new(local_repo)),
    );
    let resolver = TreeResolver::new(policy, false);
    let initial_package = remote_repo
        .find_package(&Depend::from_str(pkg).unwrap())
        .expect("can't search package")
        .iter()
        .find(|p| p.name() == pkg)
        .unwrap()
        .clone();
    let solution = resolver.resolve(&[initial_package], always_depend).expect("can't resolve");
    assert!(!solution.packages.is_empty(), "solution is empty");
    println!("Result: {:#?}", solution.pkgs().map(|pkg|pkg.to_string()).collect_vec());
}

#[rstest]
#[case("systemd", false)]
#[case("electron", false)]
#[case("agda-git", false)]
#[case("agda-git", true)]
fn test_resolve(#[case] pkg: &str, #[case] skip: bool) {
    must_resolve(pkg, skip);
}

#[rstest]
#[case("fcft")]
#[case("agda-git")]
#[case("firedragon")]
fn test_plan(#[case] pkg: &str) {
    must_plan(pkg);
}