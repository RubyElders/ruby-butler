use crate::constants::{DEFAULT_BUILD_JOBS, DEFAULT_BUILD_TIMEOUT};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf, time::Duration};

#[derive(Clone)]
pub struct BuildOptions {
    pub ruby: PathBuf,
    pub(crate) directory: PathBuf,
    pub(crate) inherited_environment: BTreeMap<OsString, OsString>,
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

impl std::fmt::Debug for BuildOptions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BuildOptions")
            .field("ruby", &self.ruby)
            .field("jobs", &self.jobs)
            .field("arguments", &self.arguments)
            .field("environment", &self.environment)
            .field("load_paths", &self.load_paths)
            .field("dependencies", &self.dependencies)
            .field("timeout", &self.timeout)
            .field("make", &self.make)
            .field("cargo", &self.cargo)
            .field("cmake", &self.cmake)
            .finish()
    }
}

impl BuildOptions {
    pub fn directory(&self) -> &std::path::Path {
        &self.directory
    }

    pub fn inherited_environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.inherited_environment
    }

    pub fn new(ruby: impl Into<PathBuf>) -> Self {
        Self::with_inputs(
            ruby,
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            std::env::vars_os().collect(),
        )
    }

    pub fn with_inputs(
        ruby: impl Into<PathBuf>,
        directory: PathBuf,
        inherited_environment: BTreeMap<OsString, OsString>,
    ) -> Self {
        #[cfg(windows)]
        let inherited_environment = inherited_environment
            .into_iter()
            .map(|(key, value)| {
                let key = key
                    .to_str()
                    .map(|key| OsString::from(key.to_ascii_uppercase()))
                    .unwrap_or(key);
                (key, value)
            })
            .collect();
        Self {
            ruby: ruby.into(),
            directory,
            inherited_environment,
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
