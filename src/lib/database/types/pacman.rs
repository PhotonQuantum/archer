use std::str::FromStr;

use derive_builder::Builder;
use pkginfo::PkgInfo;
use serde::{Deserialize, Serialize};

use crate::types::*;

use super::date_serde;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug, Builder)]
#[builder(pattern = "owned")]
pub struct PacmanEntry {
    /// file name
    #[serde(rename = "FILENAME")]
    pub file_name: String,
    /// name
    #[serde(rename = "NAME")]
    pub name: String,
    /// base
    #[serde(rename = "BASE", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub base: Option<String>,
    /// version
    #[serde(rename = "VERSION")]
    pub version: Version,
    /// description
    #[serde(rename = "DESC", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub description: Option<String>,
    /// package groups
    #[serde(rename = "GROUPS", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub groups: Option<Vec<String>>,
    /// tar.xz archive size
    #[serde(rename = "CSIZE")]
    pub compressed_size: u64,
    /// installed files size
    #[serde(rename = "ISIZE")]
    pub installed_size: u64,
    /// MD5 checksum
    #[serde(rename = "MD5SUM")]
    pub md5_sum: String,
    /// SHA256 checksum
    #[serde(rename = "SHA256SUM")]
    pub sha256_sum: String,
    /// PGP signature
    #[serde(rename = "PGPSIG", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub pgp_signature: Option<String>,
    /// package home url
    #[serde(rename = "URL", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub url: Option<String>,
    /// license name
    #[serde(rename = "LICENSE", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub license: Option<Vec<String>>,
    /// processor architecture
    #[serde(rename = "ARCH")]
    pub arch: String,
    /// build date
    #[serde(rename = "BUILDDATE", with = "date_serde")]
    pub build_date: chrono::NaiveDateTime,
    /// who created this package
    #[serde(rename = "PACKAGER")]
    pub packager: String,
    /// packages which this package replaces
    #[serde(rename = "REPLACES", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub replaces: Option<Vec<Depend>>,
    /// packages which cannot be used with this package
    #[serde(rename = "CONFLICTS", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub conflicts: Option<Vec<Depend>>,
    /// packages provided by this package
    #[serde(rename = "PROVIDES", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub provides: Option<Vec<Depend>>,
    /// run-time dependencies
    #[serde(rename = "DEPENDS", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub depends: Option<Vec<Depend>>,
    #[serde(rename = "OPTDEPENDS", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub optdepends: Option<Vec<Depend>>,
    /// build-time dependencies
    #[serde(rename = "MAKEDEPENDS", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub makedepends: Option<Vec<Depend>>,
    #[serde(rename = "CHECKDEPENDS", skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub checkdepends: Option<Vec<Depend>>,
}

impl From<PkgInfo> for PacmanEntryBuilder {
    fn from(info: PkgInfo) -> Self {
        // missing fields? (e.g. checkdepends)
        PacmanEntryBuilder::default()
            .name(info.pkg_name)
            .base(info.pkg_base)
            .version(Version(info.pkg_ver))
            .description((!info.pkg_desc.is_empty()).then_some(info.pkg_desc))
            .url(info.url)
            .installed_size(u64::from(info.size))
            .arch(info.arch.to_string())
            .packager(info.packager)
            .build_date(chrono::NaiveDateTime::from_timestamp(
                info.build_date as i64,
                0,
            ))
            .groups((!info.groups.is_empty()).then_some(info.groups))
            .license((!info.license.is_empty()).then(|| {
                info.license
                    .into_iter()
                    .map(|item| item.to_string())
                    .collect()
            }))
            .conflicts((!info.conflict.is_empty()).then(|| {
                info.conflict
                    .into_iter()
                    .map(|item| Depend::from_str(&*item).unwrap())
                    .collect()
            }))
            .provides((!info.provides.is_empty()).then(|| {
                info.provides
                    .into_iter()
                    .map(|item| Depend::from_str(&*item).unwrap())
                    .collect()
            }))
            .replaces((!info.replaces.is_empty()).then(|| {
                info.replaces
                    .into_iter()
                    .map(|item| Depend::from_str(&*item).unwrap())
                    .collect()
            }))
            .depends((!info.depend.is_empty()).then(|| {
                info.depend
                    .into_iter()
                    .map(|item| Depend::from_str(&*item).unwrap())
                    .collect()
            }))
            .makedepends((!info.make_depend.is_empty()).then(|| {
                info.make_depend
                    .into_iter()
                    .map(|item| Depend::from_str(&*item).unwrap())
                    .collect()
            }))
            .checkdepends((!info.check_depend.is_empty()).then(|| {
                info.check_depend
                    .into_iter()
                    .map(|item| Depend::from_str(&*item).unwrap())
                    .collect()
            }))
            .optdepends((!info.opt_depend.is_empty()).then(|| {
                info.opt_depend
                    .into_iter()
                    .map(|item| Depend::from_str(&*item).unwrap())
                    .collect()
            }))
    }
}
