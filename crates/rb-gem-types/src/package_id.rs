use crate::{Platform, Version};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifies a gem archive by name, version and platform; source provenance is separate.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
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
