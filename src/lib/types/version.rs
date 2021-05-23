use std::cmp::Ordering;
use std::collections::Bound;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::RangeBounds;

use alpm::DepModVer;
use ranges::{Domain, GenericRange, Ranges};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// TODO figure out a way to handle `epoch` field. see https://wiki.archlinux.org/index.php/PKGBUILD#Version
#[derive(Debug, Clone)]
pub struct Version(pub String);

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&*self.0)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        struct VisitorImpl;

        impl<'de> Visitor<'de> for VisitorImpl {
            type Value = Version;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "version")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Version(String::from(v)))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Version(v))
            }
        }

        deserializer.deserialize_string(VisitorImpl)
    }
}

impl From<&alpm::Ver> for Version {
    fn from(ver: &alpm::Ver) -> Self {
        Self(ver.to_string())
    }
}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // FIX: not reliable because of custom partial eq implementation
        self.0.hash(state)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<'a> AsRef<str> for Version {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        matches!(alpm::vercmp(self.as_ref(), other.as_ref()), Ordering::Equal)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(alpm::vercmp(self.as_ref(), other.as_ref()))
    }
}

impl Eq for Version {}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        alpm::vercmp(self.as_ref(), other.as_ref())
    }
}

impl Domain for Version {}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct DependVersion(pub Ranges<Version>);

const fn bound_of(bound: Bound<&Version>) -> Option<&Version> {
    match bound {
        Bound::Included(v) | Bound::Excluded(v) => Some(v),
        Bound::Unbounded => None,
    }
}

impl Display for DependVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0.as_slice().len() > 1 {
            write!(f, "multi_ranges") // archlinux doesn't support multi range constraint
        } else if let Some(range) = self.0.as_slice().first() {
            if range.is_full() {
                write!(f, "")
            } else if range.is_empty() {
                write!(f, " ∅")
            } else if range.is_singleton() {
                write!(f, " = {}", bound_of(range.start_bound()).unwrap())
            } else if !range.is_right_unbounded() && range.is_left_unbounded() {
                if range.is_right_closed() {
                    write!(f, " <= {}", bound_of(range.end_bound()).unwrap())
                } else {
                    write!(f, " < {}", bound_of(range.end_bound()).unwrap())
                }
            } else if !range.is_left_unbounded() && range.is_right_unbounded() {
                if range.is_left_closed() {
                    write!(f, " >= {}", bound_of(range.start_bound()).unwrap())
                } else {
                    write!(f, " > {}", bound_of(range.start_bound()).unwrap())
                }
            } else {
                write!(f, "double_ended_range") // archlinux doesn't support double end constraint in one string
            }
        } else {
            write!(f, ": ∅")
        }
    }
}

impl DependVersion {
    pub fn is_empty(&self) -> bool {
        !self.0.as_slice().iter().any(|range| !range.is_empty())
    }

    pub fn is_legal(&self) -> bool {
        !(self.is_empty() || self.0.as_slice().len() > 1)
    }

    pub fn split(&self) -> Vec<Self> {
        // TODO support <>
        if self.is_legal() {
            let range = self.0.as_slice().first().unwrap();
            if !range.is_left_unbounded() && !range.is_right_unbounded() {
                vec![
                    Self(Ranges::from(GenericRange::new_with_bounds(
                        range.start_bound().cloned(),
                        Bound::Unbounded,
                    ))),
                    Self(Ranges::from(GenericRange::new_with_bounds(
                        Bound::Unbounded,
                        range.end_bound().cloned(),
                    ))),
                ]
            } else {
                vec![self.clone()]
            }
        } else {
            vec![]
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        Self(self.0.clone().intersect(other.0.clone()))
    }

    pub fn union(&self, other: &Self) -> Self {
        Self(self.0.clone().union(other.0.clone()))
    }

    pub fn contains(&self, other: &Self) -> bool {
        self.0.clone().intersect(other.0.clone()) == other.0
    }

    pub fn complement(&self) -> Self {
        Self(self.0.clone().invert())
    }

    pub fn satisfied_by(&self, target: &Version) -> bool {
        self.0.contains(target)
    }
}

impl<'a> From<alpm::DepModVer<'a>> for DependVersion {
    fn from(dep_ver: DepModVer<'a>) -> Self {
        Self(match dep_ver {
            DepModVer::Any => Ranges::full(),
            DepModVer::Eq(ver) => Ranges::from(Version::from(ver)),
            DepModVer::Ge(ver) => Ranges::from(Version::from(ver)..),
            DepModVer::Le(ver) => Ranges::from(..=Version::from(ver)),
            DepModVer::Gt(ver) => Ranges::from(GenericRange::new_greater_than(Version::from(ver))),
            DepModVer::Lt(ver) => Ranges::from(..Version::from(ver)),
        })
    }
}
