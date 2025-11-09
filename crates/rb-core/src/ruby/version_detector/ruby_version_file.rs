//! Detector for .ruby-version files

use super::RubyVersionDetector;
use log::{debug, warn};
use semver::Version;
use std::fs;
use std::path::Path;

/// Detects Ruby version from .ruby-version file
pub struct RubyVersionFileDetector;

impl RubyVersionDetector for RubyVersionFileDetector {
    fn detect(&self, context: &Path) -> Option<Version> {
        let ruby_version_path = context.join(".ruby-version");
        debug!(
            "Checking for .ruby-version file: {}",
            ruby_version_path.display()
        );

        match fs::read_to_string(&ruby_version_path) {
            Ok(content) => {
                let version_str = content.trim();
                debug!("Found .ruby-version content: '{}'", version_str);

                match Version::parse(version_str) {
                    Ok(version) => {
                        debug!(
                            "Successfully parsed Ruby version from .ruby-version: {}",
                            version
                        );
                        Some(version)
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse Ruby version '{}' from .ruby-version: {}",
                            version_str, e
                        );
                        None
                    }
                }
            }
            Err(_) => {
                debug!("No .ruby-version file found");
                None
            }
        }
    }

    fn name(&self) -> &'static str {
        ".ruby-version"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detects_valid_version() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join(".ruby-version"), "3.2.5\n").unwrap();

        let detector = RubyVersionFileDetector;
        let version = detector.detect(temp_dir.path()).unwrap();

        assert_eq!(version, Version::new(3, 2, 5));
    }

    #[test]
    fn test_handles_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join(".ruby-version"), "  3.1.0  \n").unwrap();

        let detector = RubyVersionFileDetector;
        let version = detector.detect(temp_dir.path()).unwrap();

        assert_eq!(version, Version::new(3, 1, 0));
    }

    #[test]
    fn test_returns_none_when_file_missing() {
        let temp_dir = TempDir::new().unwrap();

        let detector = RubyVersionFileDetector;
        assert!(detector.detect(temp_dir.path()).is_none());
    }

    #[test]
    fn test_returns_none_when_invalid_version() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join(".ruby-version"), "invalid\n").unwrap();

        let detector = RubyVersionFileDetector;
        assert!(detector.detect(temp_dir.path()).is_none());
    }

    #[test]
    fn test_name() {
        assert_eq!(RubyVersionFileDetector.name(), ".ruby-version");
    }
}
