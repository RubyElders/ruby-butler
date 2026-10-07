use crate::{Dependency, ParseError, Source};
use rb_gem_types::Version;
use std::collections::BTreeMap;

/// Source provenance, dependency declarations and environment sections from Gemfile.lock.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Lockfile {
    pub sources: Vec<Source>,
    pub dependencies: BTreeMap<String, Dependency>,
    pub platforms: Vec<String>,
    pub checksums: Option<BTreeMap<String, Option<String>>>,
    pub ruby_version: Option<String>,
    pub bundled_with: Option<Version>,
}

impl Lockfile {
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        crate::parser::parse(text)
    }

    pub fn render(&self) -> Result<String, ParseError> {
        crate::render::render(self)
    }
}
