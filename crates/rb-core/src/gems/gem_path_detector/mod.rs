//! Gem path detection using various sources
//!
//! This module provides a **modular, extensible architecture** for detecting
//! and composing gem paths based on the runtime environment - bundler isolation,
//! custom gem bases, or standard user gem directories.
//!
//! # Architecture
//!
//! The system uses the **Strategy Pattern** with a trait-based design:
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │     GemPathDetector (trait)             │
//! │  - detect(&self, ctx) -> Option<GP>     │
//! │  - name(&self) -> &str                  │
//! └────────────┬────────────────────────────┘
//!              │
//!     ┌────────┴─────────┬──────────┬────────────┐
//!     │                  │          │            │
//! ┌───▼────────┐  ┌──────▼──────┐  ▼      ┌─────▼──────────┐
//! │ Bundler    │  │  Custom     │  ...    │ CompositeD.    │
//! │ Isolation  │  │  Gem Base   │         │ (chains many)  │
//! └────────────┘  └─────────────┘         └────────────────┘
//! ```
//!
//! # Usage
//!
//! For standard Ruby projects:
//! ```text
//! use rb_core::gems::gem_path_detector::{
//!     CompositeGemPathDetector, CustomGemBaseDetector,
//!     BundlerIsolationDetector, UserGemsDetector,
//! };
//!
//! let detector = CompositeGemPathDetector::new(vec![
//!     Box::new(CustomGemBaseDetector),
//!     Box::new(BundlerIsolationDetector),
//!     Box::new(UserGemsDetector),
//! ]);
//! if let Some(gem_path) = detector.detect(context) {
//!     println!("Gem directories: {:?}", gem_path.gem_dirs());
//! }
//! ```
//!
//! # Adding New Detectors
//!
//! To add support for new gem path sources (e.g., `.gems/` local directory):
//!
//! 1. Implement the `GemPathDetector` trait
//! 2. Add to the detector chain in priority order

use log::debug;
use std::path::{Path, PathBuf};

use crate::ruby::RubyRuntime;

pub mod bundler_isolation;
pub mod custom_gem_base;
pub mod user_gems;

pub use bundler_isolation::BundlerIsolationDetector;
pub use custom_gem_base::CustomGemBaseDetector;
pub use user_gems::UserGemsDetector;

/// Represents a detected gem path configuration
#[derive(Debug, Clone, PartialEq)]
pub struct GemPathConfig {
    /// Directories to include in GEM_PATH (and GEM_HOME set to first)
    pub gem_dirs: Vec<PathBuf>,
    /// Binary directories for executables
    pub gem_bin_dirs: Vec<PathBuf>,
}

impl GemPathConfig {
    /// Create a new gem path configuration
    pub fn new(gem_dirs: Vec<PathBuf>, gem_bin_dirs: Vec<PathBuf>) -> Self {
        Self {
            gem_dirs,
            gem_bin_dirs,
        }
    }

    /// Get gem directories
    pub fn gem_dirs(&self) -> &[PathBuf] {
        &self.gem_dirs
    }

    /// Get gem binary directories
    pub fn gem_bin_dirs(&self) -> &[PathBuf] {
        &self.gem_bin_dirs
    }

    /// Get the primary gem home (first gem dir)
    pub fn gem_home(&self) -> Option<&Path> {
        self.gem_dirs.first().map(|p| p.as_path())
    }
}

/// Context information for gem path detection
#[derive(Debug)]
pub struct GemPathContext<'a> {
    /// Current working directory
    pub current_dir: &'a Path,
    /// Ruby runtime being used
    pub ruby_runtime: &'a RubyRuntime,
    /// Custom gem base directory (from -G flag)
    pub custom_gem_base: Option<&'a Path>,
}

impl<'a> GemPathContext<'a> {
    /// Create a new gem path context
    pub fn new(
        current_dir: &'a Path,
        ruby_runtime: &'a RubyRuntime,
        custom_gem_base: Option<&'a Path>,
    ) -> Self {
        Self {
            current_dir,
            ruby_runtime,
            custom_gem_base,
        }
    }
}

/// Trait for gem path detection strategies
pub trait GemPathDetector {
    /// Attempt to detect gem path configuration
    ///
    /// Returns `Some(GemPathConfig)` if this detector should handle gem paths,
    /// or `None` if this detector does not apply.
    fn detect(&self, context: &GemPathContext) -> Option<GemPathConfig>;

    /// Human-readable name of this detector (for logging)
    fn name(&self) -> &'static str;
}

/// Composite detector that tries multiple strategies in priority order
pub struct CompositeGemPathDetector {
    detectors: Vec<Box<dyn GemPathDetector>>,
}

impl CompositeGemPathDetector {
    /// Create a new composite detector with the given strategies
    pub fn new(detectors: Vec<Box<dyn GemPathDetector>>) -> Self {
        Self { detectors }
    }

    /// Detect gem path configuration using all configured detectors in priority order
    ///
    /// Returns the first configuration found, or falls back to user gems if no detector matches.
    pub fn detect(&self, context: &GemPathContext) -> GemPathConfig {
        for detector in &self.detectors {
            debug!(
                "Trying gem path detector '{}' in context: {}",
                detector.name(),
                context.current_dir.display()
            );
            if let Some(config) = detector.detect(context) {
                debug!(
                    "Detector '{}' detected gem path with {} dirs",
                    detector.name(),
                    config.gem_dirs.len()
                );
                return config;
            }
            debug!("Detector '{}' not applicable", detector.name());
        }

        // Should never reach here as UserGemsDetector always returns Some
        debug!("No detector matched, falling back to user gems");
        UserGemsDetector
            .detect(context)
            .expect("UserGemsDetector should always succeed")
    }

    /// Add a detector to the chain
    pub fn add_detector(&mut self, detector: Box<dyn GemPathDetector>) {
        self.detectors.push(detector);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruby::{RubyRuntime, RubyType};
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
    fn test_gem_path_config_creation() {
        let config = GemPathConfig::new(
            vec![PathBuf::from("/home/user/.gem/ruby/3.2.0")],
            vec![PathBuf::from("/home/user/.gem/ruby/3.2.0/bin")],
        );

        assert_eq!(config.gem_dirs().len(), 1);
        assert_eq!(config.gem_bin_dirs().len(), 1);
        assert_eq!(
            config.gem_home(),
            Some(Path::new("/home/user/.gem/ruby/3.2.0"))
        );
    }

    #[test]
    fn test_gem_path_config_no_dirs() {
        let config = GemPathConfig::new(vec![], vec![]);

        assert_eq!(config.gem_dirs().len(), 0);
        assert_eq!(config.gem_home(), None);
    }

    #[test]
    fn test_composite_detector_returns_first_match() {
        let ruby = create_test_ruby();
        let context = GemPathContext::new(
            Path::new("/project"),
            &ruby,
            Some(Path::new("/custom/gems")),
        );

        let detector = CompositeGemPathDetector::new(vec![
            Box::new(CustomGemBaseDetector),
            Box::new(BundlerIsolationDetector),
            Box::new(UserGemsDetector),
        ]);
        let config = detector.detect(&context);

        // Should get custom gem base (highest priority)
        assert!(
            config
                .gem_home()
                .unwrap()
                .to_string_lossy()
                .contains("custom")
        );
    }

    #[test]
    fn test_composite_detector_tries_in_order() {
        let ruby = create_test_ruby();
        let context = GemPathContext::new(Path::new("/project"), &ruby, None);

        // Test standard (non-bundler) composition
        let detector = CompositeGemPathDetector::new(vec![
            Box::new(CustomGemBaseDetector),
            Box::new(UserGemsDetector),
        ]);
        let config = detector.detect(&context);

        // Without custom gem base, should fall through to user gems
        assert!(!config.gem_dirs().is_empty());
    }

    #[test]
    fn test_bundler_composition_returns_empty() {
        let ruby = create_test_ruby();
        let context = GemPathContext::new(Path::new("/project"), &ruby, None);

        // Test bundler composition (includes BundlerIsolationDetector)
        let detector = CompositeGemPathDetector::new(vec![
            Box::new(CustomGemBaseDetector),
            Box::new(BundlerIsolationDetector),
        ]);
        let config = detector.detect(&context);

        // BundlerIsolationDetector always returns empty config (bundler isolation)
        assert_eq!(config.gem_dirs().len(), 0);
    }
}
