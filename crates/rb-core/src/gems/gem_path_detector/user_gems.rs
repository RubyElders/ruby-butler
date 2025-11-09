//! User gems detector - standard gem path configuration

use super::{GemPathConfig, GemPathContext, GemPathDetector};
use crate::gems::GemRuntime;
use log::debug;

/// Detector for standard user gem directories
///
/// This is the default fallback detector that always succeeds.
/// It provides the standard Ruby gem path configuration:
/// - Ruby's lib gems directory
/// - User's home gem directory (~/.gem/ruby/X.Y.Z)
pub struct UserGemsDetector;

impl GemPathDetector for UserGemsDetector {
    fn detect(&self, context: &GemPathContext) -> Option<GemPathConfig> {
        debug!("Using standard user gems configuration");

        // Get Ruby's built-in gem directory
        let ruby_gem_runtime = context.ruby_runtime.infer_gem_runtime().ok()?;

        // Get user's home gem directory
        let user_gem_base = home::home_dir()?.join(".gem");
        let user_gem_runtime =
            GemRuntime::for_base_dir(&user_gem_base, &context.ruby_runtime.version);

        // Compose gem directories: user gems first (GEM_HOME), then Ruby's lib
        let gem_dirs = vec![
            user_gem_runtime.gem_home.clone(),
            ruby_gem_runtime.gem_home.clone(),
        ];

        // Compose bin directories
        let gem_bin_dirs = vec![
            user_gem_runtime.gem_bin.clone(),
            ruby_gem_runtime.gem_bin.clone(),
        ];

        Some(GemPathConfig::new(gem_dirs, gem_bin_dirs))
    }

    fn name(&self) -> &'static str {
        "user-gems"
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
    fn test_always_detects() {
        let ruby = create_test_ruby();
        let context = GemPathContext::new(Path::new("/any/directory"), &ruby, None);

        let detector = UserGemsDetector;
        let config = detector.detect(&context);

        assert!(config.is_some());
    }

    #[test]
    fn test_includes_both_user_and_ruby_gems() {
        let ruby = create_test_ruby();
        let context = GemPathContext::new(Path::new("/project"), &ruby, None);

        let detector = UserGemsDetector;
        let config = detector.detect(&context).unwrap();

        // Should have both user gems and Ruby's lib gems
        assert_eq!(config.gem_dirs().len(), 2);

        // First should be user gems (GEM_HOME)
        let gem_home = config.gem_home().unwrap();
        assert!(gem_home.to_string_lossy().contains(".gem"));

        // Should have bin directories for both
        assert!(!config.gem_bin_dirs().is_empty());
    }
}
