use std::path::PathBuf;

use crate::gems::gem_path_detector::CompositeGemPathDetector;
use crate::ruby::version_detector::CompositeDetector;

pub trait RuntimeProvider {
    /// Returns the bin directory, if available.
    fn bin_dir(&self) -> Option<PathBuf>;
    /// Returns the gem directory, if available.
    fn gem_dir(&self) -> Option<PathBuf>;

    /// Compose a version detector appropriate for this runtime environment
    ///
    /// Each environment must explicitly define which version detectors it uses
    /// and in what order. This ensures clear, environment-specific detection logic.
    fn compose_version_detector(&self) -> CompositeDetector;

    /// Compose a gem path detector appropriate for this runtime environment
    ///
    /// Each environment must explicitly define which gem path detectors it uses
    /// and in what priority order. This ensures clear, environment-specific gem resolution.
    fn compose_gem_path_detector(&self) -> CompositeGemPathDetector;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct DummyProvider;
    impl RuntimeProvider for DummyProvider {
        fn bin_dir(&self) -> Option<PathBuf> {
            Some(PathBuf::from("/dummy/bin"))
        }

        fn gem_dir(&self) -> Option<PathBuf> {
            None
        }

        fn compose_version_detector(&self) -> CompositeDetector {
            use crate::ruby::version_detector::{GemfileDetector, RubyVersionFileDetector};

            CompositeDetector::new(vec![
                Box::new(RubyVersionFileDetector),
                Box::new(GemfileDetector),
            ])
        }

        fn compose_gem_path_detector(&self) -> CompositeGemPathDetector {
            use crate::gems::gem_path_detector::{
                BundlerIsolationDetector, CustomGemBaseDetector, UserGemsDetector,
            };

            CompositeGemPathDetector::new(vec![
                Box::new(CustomGemBaseDetector),
                Box::new(BundlerIsolationDetector),
                Box::new(UserGemsDetector),
            ])
        }
    }

    #[test]
    fn runtime_provider_trait_basic() {
        let p = DummyProvider;
        assert_eq!(p.bin_dir(), Some(PathBuf::from("/dummy/bin")));
        assert_eq!(p.gem_dir(), None);
    }
}
