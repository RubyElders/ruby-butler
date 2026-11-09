use crate::constants::DEFAULT_WORKERS;
use rb_gem_installer::BuildOptions;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct InstallOptions {
    pub workers: usize,
    pub build: BuildOptions,
}

impl InstallOptions {
    pub fn with_inputs(
        ruby: impl Into<PathBuf>,
        directory: PathBuf,
        environment: std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
    ) -> Self {
        Self {
            workers: DEFAULT_WORKERS,
            build: BuildOptions::with_inputs(ruby, directory, environment),
        }
    }

    pub fn new(ruby: impl Into<PathBuf>) -> Self {
        Self {
            workers: DEFAULT_WORKERS,
            build: BuildOptions::new(ruby),
        }
    }
}
