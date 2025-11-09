//! Bundler isolation detector - prevents user gems from polluting bundler projects

use super::{GemPathConfig, GemPathContext, GemPathDetector};
use log::debug;

/// Detector for bundler project isolation
///
/// When in a bundler-managed project, this detector returns an empty config
/// to indicate that NO gem paths should be set. Bundler will manage its own
/// gem isolation through BUNDLE_PATH and the vendor/bundle directory.
///
/// This prevents user gems from polluting the bundler environment and causing
/// version conflicts.
pub struct BundlerIsolationDetector;

impl GemPathDetector for BundlerIsolationDetector {
    fn detect(&self, context: &GemPathContext) -> Option<GemPathConfig> {
        // Check if bundler was detected (respects --no-bundler flag)
        if !context.bundler_detected {
            return None;
        }

        debug!("Bundler project detected - using bundler isolation (no gem paths)");

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
    fn test_detects_bundler_project() {
        let sandbox = BundlerSandbox::new().unwrap();
        sandbox.add_bundler_project("app", false).unwrap();
        let app_dir = sandbox.root().join("app");

        let ruby = create_test_ruby();
        let context = GemPathContext::new(&app_dir, &ruby, None).with_bundler(true); // Bundler was detected

        let detector = BundlerIsolationDetector;
        let config = detector.detect(&context);

        assert!(config.is_some());
        let config = config.unwrap();
        // Should have NO gem dirs (bundler isolation)
        assert_eq!(config.gem_dirs().len(), 0);
        assert_eq!(config.gem_home(), None);
    }

    #[test]
    fn test_returns_none_for_non_bundler_project() {
        let sandbox = BundlerSandbox::new().unwrap();
        // No Gemfile added

        let ruby = create_test_ruby();
        let context = GemPathContext::new(sandbox.root(), &ruby, None).with_bundler(false); // No bundler detected

        let detector = BundlerIsolationDetector;
        let config = detector.detect(&context);

        assert!(config.is_none());
    }
}
