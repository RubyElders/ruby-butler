use crate::{Platform, Version};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
};

/// Identifies a gem archive by name, version and platform; source provenance is separate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageId {
    pub name: String,
    pub version: Version,
    pub platform: Platform,
}

impl PackageId {
    pub fn archive_version(&self) -> String {
        if self.platform.os == "ruby" {
            self.version.to_string()
        } else {
            format!("{}-{}", self.version, self.platform)
        }
    }

    pub fn full_name(&self) -> String {
        format!("{}-{}", self.name, self.archive_version())
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.full_name())
    }
}

impl PartialEq for PackageId {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.version.as_str() == other.version.as_str()
            && self.platform == other.platform
    }
}
impl Eq for PackageId {}
impl Hash for PackageId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.version.as_str().hash(state);
        self.platform.hash(state);
    }
}
impl Ord for PackageId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name
            .cmp(&other.name)
            .then_with(|| self.version.cmp(&other.version))
            .then_with(|| self.version.as_str().cmp(other.version.as_str()))
            .then_with(|| self.platform.cmp(&other.platform))
    }
}
impl PartialOrd for PackageId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
