//! Ruby version detection using various sources
//!
//! This module provides a **modular, extensible architecture** for detecting
//! required Ruby versions from various sources like .ruby-version files,
//! Gemfile declarations, and potentially .tool-versions (asdf/mise).
//!
//! # Architecture
//!
//! The system uses the **Strategy Pattern** with a trait-based design:
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │     RubyVersionDetector (trait)         │
//! │  - detect(&self, path) -> Option<V>     │
//! │  - name(&self) -> &str                  │
//! └────────────┬────────────────────────────┘
//!              │
//!     ┌────────┴─────────┬──────────┬────────────┐
//!     │                  │          │            │
//! ┌───▼────────┐  ┌──────▼──────┐  ▼      ┌─────▼──────────┐
//! │ .ruby-     │  │  Gemfile    │  ...    │ CompositeD.    │
//! │ version    │  │  Detector   │         │ (chains many)  │
//! └────────────┘  └─────────────┘         └────────────────┘
//! ```
//!
//! # Usage
//!
//! For standard Ruby projects:
//! ```text
//! use rb_core::ruby::version_detector::{CompositeDetector, GemfileDetector, RubyVersionFileDetector};
//!
//! let detector = CompositeDetector::new(vec![
//!     Box::new(RubyVersionFileDetector),
//!     Box::new(GemfileDetector),
//! ]);
//! if let Some(version) = detector.detect(project_root) {
//!     println!("Required Ruby: {}", version);
//! }
//! ```
//!
//! For bundler-managed projects:
//! ```text
//! let detector = CompositeDetector::new(vec![
//!     Box::new(RubyVersionFileDetector),
//!     Box::new(GemfileDetector),
//! ]);
//! let version = detector.detect(bundler_root);
//! ```
//!
//! # Adding New Detectors
//!
//! To add support for new version sources (e.g., `.tool-versions` for asdf):
//!
//! 1. Implement the `RubyVersionDetector` trait:
//!    ```text
//!    pub struct ToolVersionsDetector;
//!    impl RubyVersionDetector for ToolVersionsDetector {
//!        fn detect(&self, context: &Path) -> Option<Version> {
//!            // Read .tool-versions, parse "ruby X.Y.Z" line
//!        }
//!        fn name(&self) -> &'static str { ".tool-versions" }
//!    }
//!    ```
//!
//! 2. Add to the detector chain:
//!    ```text
//!    CompositeDetector {
//!        detectors: vec![
//!            Box::new(RubyVersionFileDetector),
//!            Box::new(GemfileDetector),
//!            Box::new(ToolVersionsDetector),  // <-- Add here
//!        ]
//!    }
//!    ```

use log::debug;
use semver::Version;
use std::path::Path;

pub mod gemfile;
pub mod ruby_version_file;

pub use gemfile::GemfileDetector;
pub use ruby_version_file::RubyVersionFileDetector;

/// Trait for Ruby version detection strategies
pub trait RubyVersionDetector {
    /// Attempt to detect a Ruby version requirement
    ///
    /// Returns `Some(Version)` if a version requirement is found,
    /// or `None` if this detector cannot determine a version.
    fn detect(&self, context: &Path) -> Option<Version>;

    /// Human-readable name of this detector (for logging)
    fn name(&self) -> &'static str;
}

/// Composite detector that tries multiple strategies in order
pub struct CompositeDetector {
    detectors: Vec<Box<dyn RubyVersionDetector>>,
}

impl CompositeDetector {
    /// Create a new composite detector with the given strategies
    pub fn new(detectors: Vec<Box<dyn RubyVersionDetector>>) -> Self {
        Self { detectors }
    }

    /// Detect Ruby version using all configured detectors in order
    ///
    /// Returns the first version found, or None if no detector succeeds.
    pub fn detect(&self, context: &Path) -> Option<Version> {
        for detector in &self.detectors {
            debug!(
                "Trying detector '{}' in context: {}",
                detector.name(),
                context.display()
            );
            if let Some(version) = detector.detect(context) {
                debug!("Detector '{}' found version: {}", detector.name(), version);
                return Some(version);
            }
            debug!("Detector '{}' found no version", detector.name());
        }
        debug!("No detector found a Ruby version requirement");
        None
    }

    /// Add a detector to the chain
    pub fn add_detector(&mut self, detector: Box<dyn RubyVersionDetector>) {
        self.detectors.push(detector);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_composite_detector_tries_in_order() {
        let temp_dir = TempDir::new().unwrap();

        // Create both .ruby-version and Gemfile
        std::fs::write(temp_dir.path().join(".ruby-version"), "3.2.5\n").unwrap();

        let gemfile_path = temp_dir.path().join("Gemfile");
        let mut file = std::fs::File::create(&gemfile_path).unwrap();
        writeln!(file, "ruby '3.1.0'").unwrap();

        let detector = CompositeDetector::new(vec![
            Box::new(ruby_version_file::RubyVersionFileDetector),
            Box::new(gemfile::GemfileDetector),
        ]);
        let version = detector.detect(temp_dir.path()).unwrap();

        // .ruby-version should take precedence (first in chain)
        assert_eq!(version, Version::new(3, 2, 5));
    }

    #[test]
    fn test_composite_detector_falls_back() {
        let temp_dir = TempDir::new().unwrap();

        // Only create Gemfile (no .ruby-version)
        let gemfile_path = temp_dir.path().join("Gemfile");
        let mut file = std::fs::File::create(&gemfile_path).unwrap();
        writeln!(file, "ruby '2.7.8'").unwrap();

        let detector = CompositeDetector::new(vec![
            Box::new(ruby_version_file::RubyVersionFileDetector),
            Box::new(gemfile::GemfileDetector),
        ]);
        let version = detector.detect(temp_dir.path()).unwrap();

        // Should fall back to Gemfile
        assert_eq!(version, Version::new(2, 7, 8));
    }

    #[test]
    fn test_composite_detector_returns_none_when_nothing_found() {
        let temp_dir = TempDir::new().unwrap();

        let detector = CompositeDetector::new(vec![
            Box::new(ruby_version_file::RubyVersionFileDetector),
            Box::new(gemfile::GemfileDetector),
        ]);
        assert!(detector.detect(temp_dir.path()).is_none());
    }
}
