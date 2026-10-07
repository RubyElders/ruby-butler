use rb_gem_types::{PackageId, Platform, Requirement, Version};
use std::collections::BTreeMap;

/// Resolution inputs and locked package preferences.
#[derive(Clone, Debug)]
pub struct ResolveRequest {
    pub roots: BTreeMap<String, Requirement>,
    pub ruby: Version,
    pub target: Platform,
    pub locked: Vec<PackageId>,
    pub allow_prereleases: bool,
}

impl ResolveRequest {
    pub fn new(roots: BTreeMap<String, Requirement>, ruby: Version, target: Platform) -> Self {
        Self {
            roots,
            ruby,
            target,
            locked: Vec::new(),
            allow_prereleases: false,
        }
    }
}
