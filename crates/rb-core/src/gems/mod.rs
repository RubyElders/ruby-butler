use crate::butler::runtime_provider::RuntimeProvider;
use log::debug;
use semver::Version;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemRuntime {
    pub gem_home: PathBuf,
    pub gem_bin: PathBuf,
}

impl GemRuntime {
    /// Create a GemRuntime from a base directory
    ///
    /// The gem_home will be base/ruby/version where version is the full version (x.y.z)
    /// base: e.g. ~/.gem, /usr/lib/ruby/gems
    pub fn for_base_dir(base: &Path, ruby_version: &Version) -> Self {
        debug!(
            "Creating GemRuntime for base: {}, Ruby version: {}",
            base.display(),
            ruby_version
        );

        let ver = format!(
            "{}.{}.{}",
            ruby_version.major, ruby_version.minor, ruby_version.patch
        );
        debug!("Using full version string: {}", ver);

        let gem_home = base.join("ruby").join(ver);
        let gem_bin = gem_home.join("bin");

        debug!(
            "Created GemRuntime - gem_home: {}, gem_bin: {}",
            gem_home.display(),
            gem_bin.display()
        );

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

    #[test]
    fn test_for_base_dir_uses_full_version() {
        let base = Path::new("/home/user/.gem");
        let ver = Version::parse("3.4.5").unwrap();
        let gem = GemRuntime::for_base_dir(base, &ver);

        let expected_gem_home = base.join("ruby").join("3.4.5");
        let expected_gem_bin = expected_gem_home.join("bin");

        assert_eq!(gem.gem_home, expected_gem_home);
        assert_eq!(gem.gem_bin, expected_gem_bin);
    }
}
