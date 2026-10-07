use crate::LockedPackage;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SourceKind {
    Git,
    Registry,
    Path,
}

impl SourceKind {
    pub(crate) fn section(self) -> &'static str {
        match self {
            Self::Git => "GIT",
            Self::Registry => "GEM",
            Self::Path => "PATH",
        }
    }
}

/// A source section owns its packages, including their platform variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Source {
    pub kind: SourceKind,
    pub remotes: Vec<String>,
    pub options: BTreeMap<String, String>,
    pub packages: Vec<LockedPackage>,
}

impl Source {
    pub fn new(kind: SourceKind, remote: impl Into<String>) -> Self {
        Self {
            kind,
            remotes: vec![remote.into()],
            options: BTreeMap::new(),
            packages: Vec::new(),
        }
    }
}
