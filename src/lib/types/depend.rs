use std::fmt::{Display, Formatter};
use std::str::FromStr;

use alpm::Dep;
use ranges::{GenericRange, Ranges};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::*;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Depend {
    pub name: String,
    pub version: DependVersion,
}

impl<'de> Deserialize<'de> for Depend {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        struct VisitorImpl;

        impl<'de> Visitor<'de> for VisitorImpl {
            type Value = Depend;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    "dependency name(like 'test') with or without version constraint"
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Depend::from_str(v).unwrap())
            }
        }

        deserializer.deserialize_str(VisitorImpl)
    }
}

impl Serialize for Depend {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl Depend {
    pub fn satisfied_by(&self, candidate: &Package) -> bool {
        (candidate.name() == self.name && self.version.satisfied_by(&candidate.version()))
            || candidate
                .provides()
                .iter()
                .any(|provide| provide.name == self.name && self.version.contains(&provide.version))
    }
    pub fn split_ver(&self) -> Vec<Self> {
        self.version
            .split()
            .into_iter()
            .map(|ver| Self {
                name: self.name.clone(),
                version: ver,
            })
            .collect()
    }
}

impl Display for Depend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.split_ver().as_slice() {
            [v] => write!(f, "{} {}", self.name, v.version),
            [v1, v2] => write!(f, "{} {} && {}", self.name, v1.version, v2.version),
            _ => write!(f, "{} {}", self.name, self.version),
        }
    }
}

impl From<&Package> for Depend {
    fn from(pkg: &Package) -> Self {
        Self {
            name: pkg.name().to_string(),
            version: DependVersion(Ranges::from(pkg.version().into_owned())),
        }
    }
}

macro_rules! split_cmp_op {
    ($s: ident, $sep: expr, $rel: expr) => {
        $s.split_once($sep).map(|(name, ver)| {
            (
                name.to_string(),
                DependVersion(Ranges::from($rel(Version(ver.to_string())))),
            )
        })
    };
}

impl FromStr for Depend {
    type Err = !;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // TODO parse neq (!=? <>?)
        let (name, version) = split_cmp_op!(s, ">=", GenericRange::new_at_least)
            .or_else(|| split_cmp_op!(s, "<=", GenericRange::new_at_most))
            .or_else(|| split_cmp_op!(s, ">", GenericRange::new_greater_than))
            .or_else(|| split_cmp_op!(s, "<", GenericRange::new_less_than))
            .or_else(|| split_cmp_op!(s, "=", GenericRange::singleton))
            .unwrap_or((s.to_string(), DependVersion(Ranges::full())));
        Ok(Self { name, version })
    }
}

impl<'a> From<alpm::Dep<'a>> for Depend {
    fn from(dep: Dep<'a>) -> Self {
        Self {
            name: dep.name().to_string(),
            version: dep.depmodver().into(),
        }
    }
}
