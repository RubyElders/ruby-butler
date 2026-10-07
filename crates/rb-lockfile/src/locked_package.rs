use rb_gem_types::{Requirement, Version};
use std::collections::BTreeMap;

/// Lockfile identity retains the archive's platform spelling without target normalization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockedPackage {
    pub name: String,
    pub version: Version,
    pub platform: String,
    pub dependencies: BTreeMap<String, Vec<Requirement>>,
}

impl LockedPackage {
    pub fn archive_version(&self) -> String {
        if self.platform == "ruby" {
            self.version.to_string()
        } else {
            format!("{}-{}", self.version, self.platform)
        }
    }

    pub fn identity(&self) -> String {
        format!("{} ({})", self.name, self.archive_version())
    }
}
