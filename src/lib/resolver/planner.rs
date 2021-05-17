use std::collections::VecDeque;
use std::sync::Arc;

use itertools::Itertools;

use crate::error::Result;
use crate::repository::*;
use crate::types::*;

use super::{tree_resolv::TreeResolver, types::*};

#[derive(Clone)]
pub struct PlanBuilder {
    pkgs: Vec<Package>,
    local_repo: Arc<CachedRepository>,
    aur_repo: Arc<CachedRepository>,
    global_repo: Arc<MergedRepository>,
    aur_resolver: TreeResolver,
    pacman_resolver: TreeResolver,
    global_resolver: TreeResolver,
}

impl Default for PlanBuilder {
    fn default() -> Self {
        let aur_repo = Arc::new(CachedRepository::new(Arc::new(AurRepo::new())));
        let local_repo = Arc::new(CachedRepository::new(Arc::new(PacmanLocal::new())));
        let remote_repo = Arc::new(CachedRepository::new(Arc::new(PacmanRemote::new())));
        let global_repo = Arc::new(MergedRepository::new(vec![
            remote_repo.clone(),
            aur_repo.clone(),
        ]));

        let aur_policy =
            ResolvePolicy::new(global_repo.clone(), remote_repo.clone(), local_repo.clone());
        let remote_policy = ResolvePolicy::new(remote_repo, local_repo.clone(), local_repo.clone());
        let global_policy =
            ResolvePolicy::new(global_repo.clone(), local_repo.clone(), local_repo.clone());

        let aur_resolver = TreeResolver::new(aur_policy);
        let pacman_resolver = TreeResolver::new(remote_policy);
        let global_resolver = TreeResolver::new(global_policy);
        Self {
            pkgs: vec![],
            local_repo,
            aur_repo,
            global_repo,
            aur_resolver,
            pacman_resolver,
            global_resolver,
        }
    }
}

impl PlanBuilder {
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_package(&mut self, pkg: &Depend) -> Result<()> {
        let mut pkg = self.global_repo.find_package(pkg)?;
        if let Some(pkg) = pkg.pop() {
            self.add_package_exact(pkg);
        }
        Ok(())
    }

    pub fn add_package_exact(&mut self, pkg: Package) {
        if let Package::AurPackage(_) = pkg {
            self.pkgs.push(pkg);
        }
    }

    pub fn build(self) -> Result<Vec<PlanAction>> {
        let mut plan = vec![];
        let mut pkgs_to_build: VecDeque<Package> = VecDeque::new();
        pkgs_to_build.extend(self.pkgs);
        while let Some(pkg_to_build) = pkgs_to_build.pop_front() {
            // search makedepends
            let make_deps = self
                .global_repo
                .find_packages(&**pkg_to_build.make_depends())?;

            // search aur depends
            let aur_custom_deps = self
                .global_repo
                .find_packages(&**pkg_to_build.depends())?
                .into_iter()
                .filter_map(|(_, mut v)| {
                    let first_pkg = v.pop().unwrap();
                    match first_pkg {
                        Package::PacmanPackage(_) => None,
                        Package::AurPackage(_) => Some(first_pkg),
                        Package::CustomPackage(_) => Some(first_pkg),
                    }
                })
                .collect_vec();
            let mut aur_custom_make_deps = vec![];
            let mut pacman_make_deps = vec![];

            // split make depends by source (aur/pacman)
            for (_, mut deps) in make_deps {
                let mut skip = false;
                for dep in &deps {
                    if !self.local_repo.find_package(&Depend::from(dep))?.is_empty() {
                        // skip existing packages in local repo
                        skip = true;
                        break;
                    }
                }
                if skip {
                    continue;
                }
                if let Some(pkg) = deps.pop() {
                    match pkg {
                        Package::PacmanPackage(_) => pacman_make_deps.push(pkg),
                        Package::AurPackage(_) => aur_custom_make_deps.push(pkg),
                        Package::CustomPackage(_) => aur_custom_make_deps.push(pkg),
                    }
                }
            }

            // build & install aur make dependencies
            for mut pkgs in self
                .global_resolver
                .resolve(
                    &*aur_custom_make_deps,
                    makedepend_if_aur_custom,
                    allow_if_pacman,
                )?
                .strongly_connected_components()
            {
                // TODO avoid dup build
                if pkgs.len() > 1 {
                    plan.push(PlanAction::InstallGroup(
                        pkgs.into_iter().map(|p| p.as_ref().clone()).collect(),
                    ))
                } else {
                    let pkg = pkgs.pop().unwrap();
                    if let Package::AurPackage(_) = pkg.as_ref() {
                        plan.push(PlanAction::Build(pkg.as_ref().clone()));
                    }
                    plan.push(PlanAction::Install(pkg.as_ref().clone()))
                }
            }

            // install pacman make dependencies
            // Note
            // pacman makedeps are installed behind aur deps to avoid being uninstalled later by deps of aur makedeps
            for pkgs in self
                .pacman_resolver
                .resolve(&*pacman_make_deps, always_depend, allow_if_pacman)?
                .strongly_connected_components()
            {
                plan.push(PlanAction::InstallGroup(
                    pkgs.into_iter().map(|p| p.as_ref().clone()).collect(),
                ))
            }

            // need to build its aur dependencies
            pkgs_to_build.extend(aur_custom_deps);

            // build this package
            // TODO avoid dup build
            plan.push(PlanAction::Build(pkg_to_build.clone()));
            plan.push(PlanAction::CopyToDest(pkg_to_build));
        }
        Ok(plan)
    }
}
