use crate::constants::{DEFAULT_BUILD_JOBS, DEFAULT_BUILD_TIMEOUT};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf, time::Duration};

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub ruby: PathBuf,
    pub jobs: usize,
    pub arguments: Vec<OsString>,
    pub environment: BTreeMap<OsString, OsString>,
    pub load_paths: Vec<PathBuf>,
    pub dependencies: Vec<rb_microgems::InstalledGem>,
    pub timeout: Duration,
    pub make: Option<OsString>,
    pub cargo: OsString,
    pub cmake: OsString,
}

impl BuildOptions {
    pub fn new(ruby: impl Into<PathBuf>) -> Self {
        Self {
            ruby: ruby.into(),
            jobs: DEFAULT_BUILD_JOBS,
            arguments: Vec::new(),
            environment: BTreeMap::new(),
            load_paths: Vec::new(),
            dependencies: Vec::new(),
            timeout: DEFAULT_BUILD_TIMEOUT,
            make: None,
            cargo: "cargo".into(),
            cmake: "cmake".into(),
        }
    }
}
