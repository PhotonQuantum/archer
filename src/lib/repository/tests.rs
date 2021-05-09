use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use itertools::Itertools;
use rstest::rstest;

use crate::repository::aur::AurRepo;
use crate::repository::cached::CachedRepository;
use crate::repository::custom::CustomRepository;
use crate::repository::merged::MergedRepository;
use crate::repository::pacman::{PacmanLocal, PacmanRemote};
use crate::repository::Repository;
use crate::tests::*;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct DebugRepository {
    inner: ArcRepo,
    query_map: Arc<Mutex<HashMap<Depend, usize>>>,
}

impl DebugRepository {
    pub fn new(inner: ArcRepo) -> Self {
        DebugRepository {
            inner,
            query_map: Default::default(),
        }
    }

    pub fn get_count(&self, pkg: &Depend) -> usize {
        *self.query_map.lock().unwrap().get(pkg).unwrap_or(&0)
    }
}

impl Repository for DebugRepository {
    fn find_package(&self, pkg: &Depend) -> Result<Vec<Package>> {
        self.query_map
            .lock()
            .unwrap()
            .entry(pkg.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        self.inner.find_package(pkg)
    }
}

#[rstest]
#[case(PacmanLocal::new(), "m4")]
#[case(PacmanRemote::new(), "electron")]
#[case(AurRepo::new(), "systemd-git")]
#[case(CustomRepository::new(vec![pkg!("a-alt", "1.0.0", vec![], vec![], vec![], deps!("a")), pkg!("b"), pkg!("a")]), "a")]
fn must_search(#[case] repo: impl Repository, #[case] name: &str) {
    let dep = dep!(name);
    let results = repo
        .find_package(&dep)
        .expect("failed to search aur package");
    let first_pkg = results.first().expect("no package found");
    assert_eq!(first_pkg.name(), name, "first package incorrect");
    assert!(
        results.iter().all(|pkg| dep.satisfied_by(pkg)),
        "packages incorrect"
    );
}

#[rstest]
#[case(PacmanLocal::new(), deps!("m4", "bash"))]
#[case(PacmanRemote::new(), deps!("electron", "systemd"))]
#[case(AurRepo::new(), deps!("systemd-git", "agda-git"))]
#[case(CustomRepository::new(vec![pkg!("a-alt", "1.0.0", vec![], vec![], vec![], deps!("a")), pkg!("b"), pkg!("a")]), deps!("a", "b"))]
fn must_search_multi(#[case] repo: impl Repository, #[case] deps: Vec<Depend>) {
    let results = repo
        .find_packages(&*deps)
        .expect("failed to search aur package");
    assert!(
        results.keys().all(|key| deps.contains(key)),
        "unexpected key in result"
    );
    for req in &deps {
        let pkgs = results.get(req).expect("missing package in result");
        let first_pkg = pkgs.first().expect("no package found");
        assert_eq!(first_pkg.name(), req.name, "first package incorrect");
        assert!(
            pkgs.iter().all(|pkg| req.satisfied_by(pkg)),
            "packages incorrect"
        );
    }
}

#[test]
fn must_merge() {
    let repo_1 = CustomRepository::new(vec![
        pkg!("a"),
        pkg!("a-alt", "1.0.0", vec![], vec![], vec![], deps!("a")),
        pkg!("b"),
    ]);
    let repo_2 = CustomRepository::new(vec![
        pkg!("a"),
        pkg!("a-alt-2", "1.0.0", vec![], vec![], vec![], deps!("a")),
        pkg!("c"),
    ]);
    let merged = MergedRepository::new(vec![Arc::new(repo_1), Arc::new(repo_2)]);

    let result = merged.find_package(&dep!("c")).expect("can't find package");
    asrt!("c").assert(&result.iter().collect_vec());

    let asrts = vec![asrt!("a"), asrt!("a-alt"), asrt!(!"a-alt2")];
    let result = merged.find_package(&dep!("a")).expect("can't find package");
    for asrt in asrts {
        asrt.assert(&result.iter().collect_vec());
    }

    let results = merged
        .find_packages(&*deps!("b", "c"))
        .expect("can't find packages");
    results
        .get(&dep!("b"))
        .expect("can't find packages")
        .iter()
        .any(|pkg| dep!("b").satisfied_by(pkg));
    results
        .get(&dep!("c"))
        .expect("can't find packages")
        .iter()
        .any(|pkg| dep!("c").satisfied_by(pkg));
}

#[test]
fn must_cache() {
    let inner_repo = CustomRepository::new(vec![pkg!("a"), pkg!("b"), pkg!("c"), pkg!("d")]);
    let debug_repo = Arc::new(DebugRepository::new(Arc::new(inner_repo)));
    let repo = CachedRepository::new(debug_repo.clone());

    assert_eq!(debug_repo.get_count(&dep!("a")), 0);
    repo.find_package(&dep!("a")).unwrap();
    assert_eq!(debug_repo.get_count(&dep!("a")), 1);
    repo.find_package(&dep!("a")).unwrap();
    assert_eq!(
        debug_repo.get_count(&dep!("a")),
        1,
        "single find request not cached"
    );

    assert_eq!(debug_repo.get_count(&dep!("b")), 0);
    assert_eq!(debug_repo.get_count(&dep!("c")), 0);
    repo.find_packages(&*deps!("b", "c")).unwrap();
    assert_eq!(
        debug_repo.get_count(&dep!("b")),
        1,
        "multiple find request not cached"
    );
    assert_eq!(
        debug_repo.get_count(&dep!("c")),
        1,
        "multiple find request not cached"
    );

    repo.find_packages(&*deps!("a", "c")).unwrap();
    assert_eq!(
        debug_repo.get_count(&dep!("a")),
        1,
        "multiple find request not cached"
    );
    assert_eq!(
        debug_repo.get_count(&dep!("c")),
        1,
        "multiple find request not cached"
    );
}
