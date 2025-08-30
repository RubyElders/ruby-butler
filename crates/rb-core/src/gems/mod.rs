use crate::butler::runtime_provider::RuntimeProvider;
use semver::Version;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemRuntime {
    pub gem_home: PathBuf,
    pub gem_bin: PathBuf,
}

impl GemRuntime {
    /// base: e.g. ~/.gem
    pub fn for_base_dir(base: &Path, ruby_version: &Version) -> Self {
        let ver = format!("{}.{}.0", ruby_version.major, ruby_version.minor);
        let gem_home = base.join("ruby").join(ver);
        let gem_bin = gem_home.join("bin");
        Self { gem_home, gem_bin }
    }
}

impl RuntimeProvider for GemRuntime {
    fn bin_dir(&self) -> Option<PathBuf> {
        Some(self.gem_bin.clone())
    }
    fn gem_dir(&self) -> Option<PathBuf> {
        Some(self.gem_home.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;
    use std::path::Path;

    #[test]
    fn test_gem_runtime_provider_bin_and_gem_dir() {
        let base = Path::new("/tmp/gemtest");
        let ver = Version::parse("3.2.1").unwrap();
        let gem = GemRuntime::for_base_dir(base, &ver);
        assert_eq!(gem.bin_dir(), Some(gem.gem_bin.clone()));
        assert_eq!(gem.gem_dir(), Some(gem.gem_home.clone()));
    }
}