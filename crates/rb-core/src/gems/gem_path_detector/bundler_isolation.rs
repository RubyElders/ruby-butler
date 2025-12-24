//! Bundler isolation detector - prevents user gems from polluting bundler projects

use super::{GemPathConfig, GemPathContext, GemPathDetector};
use log::debug;

/// Detector for bundler project isolation
///
/// When used in a bundler environment's detector chain, this detector returns
/// an empty config to indicate that NO gem paths should be set. Bundler will
/// manage its own gem isolation through BUNDLE_PATH and the vendor/bundle directory.
///
/// This prevents user gems from polluting the bundler environment and causing
/// version conflicts.
///
/// Note: This detector is only included in BundlerRuntime's detector composition,
/// not in standard GemRuntime composition.
pub struct BundlerIsolationDetector;

impl GemPathDetector for BundlerIsolationDetector {
    fn detect(&self, _context: &GemPathContext) -> Option<GemPathConfig> {
        debug!("Bundler environment - using bundler isolation (no gem paths)");

        // Return empty config to indicate: don't set GEM_HOME/GEM_PATH, bundler handles it
        Some(GemPathConfig::new(vec![], vec![]))
    }

    fn name(&self) -> &'static str {
        "bundler-isolation"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruby::{RubyRuntime, RubyType};
    use rb_tests::bundler_sandbox::BundlerSandbox;
    use semver::Version;
    use std::path::PathBuf;

    fn create_test_ruby() -> RubyRuntime {
        RubyRuntime::new(
            RubyType::CRuby,
            Version::parse("3.2.0").unwrap(),
            PathBuf::from("/rubies/ruby-3.2.0"),
        )
    }

    #[test]
    fn test_always_returns_empty_config() {
        let sandbox = BundlerSandbox::new().unwrap();
        sandbox.add_bundler_project("app", false).unwrap();
        let app_dir = sandbox.root().join("app");

        let ruby = create_test_ruby();
        let context = GemPathContext::new(&app_dir, &ruby, None);

        let detector = BundlerIsolationDetector;
        let config = detector.detect(&context);

        assert!(config.is_some());
        let config = config.unwrap();
        // Should have NO gem dirs (bundler isolation)
        assert_eq!(config.gem_dirs().len(), 0);
        assert_eq!(config.gem_home(), None);
    }

    #[test]
    fn test_empty_config_regardless_of_directory() {
        let sandbox = BundlerSandbox::new().unwrap();
        // No Gemfile - detector doesn't care since it's only used in bundler runtime

        let ruby = create_test_ruby();
        let context = GemPathContext::new(sandbox.root(), &ruby, None);

        let detector = BundlerIsolationDetector;
        let config = detector.detect(&context);

        // Always returns empty config when used (only included in BundlerRuntime composition)
        assert!(config.is_some());
        assert_eq!(config.unwrap().gem_dirs().len(), 0);
    }
}
