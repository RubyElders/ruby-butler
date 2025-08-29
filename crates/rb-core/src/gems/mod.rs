use semver::Version;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemRuntime {
    pub gem_home: PathBuf,
    pub gem_bin: PathBuf,
    pub gem_path: Vec<PathBuf>,
}

impl GemRuntime {
    /// base: e.g. ~/.gem
    pub fn for_base_dir(base: &Path, ruby_version: &Version) -> Self {
        let ver = format!("{}.{}.0", ruby_version.major, ruby_version.minor);
        let gem_home = base.join("ruby").join(ver);
        let gem_bin = gem_home.join("bin");
        let gem_path = vec![gem_home.clone()];
        Self { gem_home, gem_bin, gem_path }
    }
}

