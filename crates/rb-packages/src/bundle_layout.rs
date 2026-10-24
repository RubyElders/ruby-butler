use crate::constants::{GEM_HOME_DIRECTORY, STATE_DIRECTORY};
use std::path::{Path, PathBuf};

/// Installation paths shared by preparation and read-only discovery.
pub struct BundleLayout {
    pub bundle: PathBuf,
    pub gem_home: PathBuf,
}

impl BundleLayout {
    pub fn new(project: &Path, engine: &str, version: &str) -> Self {
        let bundle = project.join(STATE_DIRECTORY).join(GEM_HOME_DIRECTORY);
        let abi = version.split('.').take(2).collect::<Vec<_>>().join(".") + ".0";
        let gem_home = bundle.join(engine).join(abi);
        Self { bundle, gem_home }
    }
}
