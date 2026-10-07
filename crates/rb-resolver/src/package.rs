use rb_gem_types::{PackageId, Requirement};
use std::collections::BTreeMap;

/// A candidate's identity, runtime dependencies and Ruby version requirement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    pub id: PackageId,
    pub dependencies: BTreeMap<String, Requirement>,
    pub ruby: Requirement,
}

impl Package {
    pub fn new(id: PackageId) -> Self {
        Self {
            id,
            dependencies: BTreeMap::new(),
            ruby: Requirement::default(),
        }
    }
}
