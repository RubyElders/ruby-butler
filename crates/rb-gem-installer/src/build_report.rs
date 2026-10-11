use std::{path::PathBuf, time::Duration};

#[derive(Clone, Debug)]
pub struct BuildReport {
    pub ruby_version: String,
    pub architecture: String,
    pub extensions: Vec<PathBuf>,
    pub artifacts: Vec<PathBuf>,
    pub elapsed: Duration,
}
