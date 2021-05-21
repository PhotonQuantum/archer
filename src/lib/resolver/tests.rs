use std::str::FromStr;
use std::sync::Arc;

use itertools::Itertools;
use rstest::rstest;

use crate::tests::*;

#[rstest]
#[case(vec![pkg!("a"), pkg!("b", "1.0.0", deps!("a")), pkg!("c", "1.0.0", deps!("a")), pkg!("d"), pkg!("e", "1.0.0", deps!("b")), pkg!("f", "1.0.0", deps!("c", "e"))],
    "f",
    vec![asrt!("a" < "b" < "e" < "f"), asrt!("a" < "c" < "f"), asrt!(!"d")])]
#[case(vec![pkg!("b", "1.0.0", deps!("a>=2")), pkg!("a", "1.0.0"), pkg!("a", "2.0.0")], "b", vec![asrt!("a=2.0.0" < "b"), asrt!(!"a=1.0.0")])]
fn simple_deps(
    #[case] pkgs: Vec<Package>,
    #[case] target: &str,
    #[case] asrts: Vec<PkgsAssertion>,
) {
    let repo = Arc::new(CustomRepository::new(pkgs));
    let empty_repo = Arc::new(EmptyRepository::new());
    let policy = ResolvePolicy::new(repo.clone(), empty_repo.clone(), empty_repo);
    let resolver = TreeResolver::new(policy, box always_depend, box allow_if_pacman);

    let pkg = repo
        .find_package(&Depend::from_str(target).unwrap())
        .unwrap()
        .pop()
        .unwrap();
    let result = resolver
        .resolve(&[pkg])
        .expect("can't find solution");
    let scc = result.strongly_connected_components();
    println!(
        "{:?}",
        scc.iter()
            .map(|pkgs| format!("[{}]", pkgs.iter().map(|pkg| pkg.to_string()).join(", ")))
            .collect_vec()
    );
    for asrt in asrts {
        asrt.assert(&scc.iter().flatten().map(|pkg| pkg.as_ref()).collect_vec())
    }
}

#[rstest]
#[case(vec![pkg!("a", "1.0.0", deps!("c")), pkg!("b", "1.0.0", deps!("a")), pkg!("c", "1.0.0", deps!("b"))], "a",
    vec![asrt!("a"), asrt!("b"), asrt!("c")])]
#[case(vec![pkg!("a", "1.0.0", deps!("c")), pkg!("b", "1.0.0", deps!("a")), pkg!("c", "1.0.0", deps!("b"))], "c",
    vec![asrt!("a"), asrt!("b"), asrt!("c")])]
fn cyclic_deps(
    #[case] pkgs: Vec<Package>,
    #[case] target: &str,
    #[case] asrts: Vec<PkgsAssertion>,
) {
    let repo = Arc::new(CustomRepository::new(pkgs));
    let empty_repo = Arc::new(EmptyRepository::new());
    let policy = ResolvePolicy::new(repo.clone(), empty_repo.clone(), empty_repo);
    let resolver = TreeResolver::new(policy, box always_depend, box allow_if_pacman);

    let pkg = repo
        .find_package(&Depend::from_str(target).unwrap())
        .unwrap()
        .pop()
        .unwrap();
    let result = resolver
        .resolve(&[pkg])
        .expect("can't find solution");
    let scc = result.strongly_connected_components();
    println!(
        "{:?}",
        scc.iter()
            .map(|pkgs| format!("[{}]", pkgs.iter().map(|pkg| pkg.to_string()).join(", ")))
            .collect_vec()
    );
    for asrt in asrts {
        asrt.assert(&scc.iter().flatten().map(|pkg| pkg.as_ref()).collect_vec())
    }
}
