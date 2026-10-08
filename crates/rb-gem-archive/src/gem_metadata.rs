use rb_gem_types::{PackageId, Requirement};
use std::collections::BTreeMap;

/// Declared gem identity and metadata; payload extraction and activation are separate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GemMetadata {
    pub id: PackageId,
    pub dependencies: BTreeMap<String, Requirement>,
    pub ruby: Requirement,
    pub rubygems: Requirement,
    pub require_paths: Vec<String>,
    pub executables: Vec<String>,
    pub bindir: String,
    pub extensions: Vec<String>,
    pub files: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}
