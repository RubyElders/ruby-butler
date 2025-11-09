//! Custom gem base detector - handles explicit -G flag override

use super::{GemPathConfig, GemPathContext, GemPathDetector};
use crate::gems::GemRuntime;
use log::debug;

/// Detector for custom gem base directories (via -G flag)
///
/// This detector has the highest priority as it represents an explicit
/// user override of where gems should be installed and loaded from.
pub struct CustomGemBaseDetector;

impl GemPathDetector for CustomGemBaseDetector {
    fn detect(&self, context: &GemPathContext) -> Option<GemPathConfig> {
        let custom_base = context.custom_gem_base?;

        debug!(
            "Custom gem base specified: {}, creating gem runtime",
            custom_base.display()
        );

        // Create gem runtime for the custom base
        let gem_runtime = GemRuntime::for_base_dir(custom_base, &context.ruby_runtime.version);

        let gem_dirs = vec![gem_runtime.gem_home.clone()];
        let gem_bin_dirs = vec![gem_runtime.gem_bin.clone()];

        Some(GemPathConfig::new(gem_dirs, gem_bin_dirs))
    }

    fn name(&self) -> &'static str {
        "custom-gem-base"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruby::{RubyRuntime, RubyType};
    use semver::Version;
    use std::path::{Path, PathBuf};

    fn create_test_ruby() -> RubyRuntime {
        RubyRuntime::new(
            RubyType::CRuby,
            Version::parse("3.2.0").unwrap(),
            PathBuf::from("/rubies/ruby-3.2.0"),
        )
    }

    #[test]
    fn test_detects_when_custom_base_provided() {
        let ruby = create_test_ruby();
        let context = GemPathContext::new(
            Path::new("/project"),
            &ruby,
            Some(Path::new("/custom/gems")),
        );

        let detector = CustomGemBaseDetector;
        let config = detector.detect(&context);

        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.gem_dirs().len(), 1);
        assert!(
            config
                .gem_home()
                .unwrap()
                .to_string_lossy()
                .contains("custom/gems")
        );
    }

    #[test]
    fn test_returns_none_when_no_custom_base() {
        let ruby = create_test_ruby();
        let context = GemPathContext::new(Path::new("/project"), &ruby, None);

        let detector = CustomGemBaseDetector;
        let config = detector.detect(&context);

        assert!(config.is_none());
    }
}
