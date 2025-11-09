//! Detector for Gemfile ruby declarations

use super::RubyVersionDetector;
use log::{debug, warn};
use semver::Version;
use std::fs;
use std::path::Path;

/// Detects Ruby version from Gemfile ruby declaration
pub struct GemfileDetector;

impl RubyVersionDetector for GemfileDetector {
    fn detect(&self, context: &Path) -> Option<Version> {
        let gemfile_path = context.join("Gemfile");
        debug!(
            "Checking for ruby declaration in Gemfile: {}",
            gemfile_path.display()
        );

        match fs::read_to_string(&gemfile_path) {
            Ok(content) => {
                debug!("Reading Gemfile for ruby declaration");

                for line in content.lines() {
                    let line = line.trim();

                    // Look for patterns like: ruby '3.2.5' or ruby "3.2.5"
                    if line.starts_with("ruby ") {
                        debug!("Found ruby line: '{}'", line);

                        // Extract version string between quotes
                        if let Some(version_str) = Self::extract_quoted_version(line) {
                            debug!("Extracted version string: '{}'", version_str);

                            match Version::parse(&version_str) {
                                Ok(version) => {
                                    debug!(
                                        "Successfully parsed Ruby version from Gemfile: {}",
                                        version
                                    );
                                    return Some(version);
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to parse Ruby version '{}' from Gemfile: {}",
                                        version_str, e
                                    );
                                }
                            }
                        }
                    }
                }

                debug!("No valid ruby declaration found in Gemfile");
                None
            }
            Err(_) => {
                debug!("No Gemfile found");
                None
            }
        }
    }

    fn name(&self) -> &'static str {
        "Gemfile"
    }
}

impl GemfileDetector {
    /// Extract version string from between quotes in a line
    /// Handles both single and double quotes
    fn extract_quoted_version(line: &str) -> Option<String> {
        // Remove "ruby " prefix and trim
        let rest = line.strip_prefix("ruby ")?.trim();

        // Handle both single and double quotes
        for quote in &['\'', '"'] {
            if rest.starts_with(*quote)
                && let Some(end_idx) = rest[1..].find(*quote)
            {
                return Some(rest[1..=end_idx].to_string());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_detects_single_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let gemfile_path = temp_dir.path().join("Gemfile");
        let mut file = std::fs::File::create(&gemfile_path).unwrap();
        writeln!(file, "source 'https://rubygems.org'").unwrap();
        writeln!(file, "ruby '3.1.4'").unwrap();
        writeln!(file, "gem 'rails'").unwrap();

        let detector = GemfileDetector;
        let version = detector.detect(temp_dir.path()).unwrap();

        assert_eq!(version, Version::new(3, 1, 4));
    }

    #[test]
    fn test_detects_double_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let gemfile_path = temp_dir.path().join("Gemfile");
        let mut file = std::fs::File::create(&gemfile_path).unwrap();
        writeln!(file, "source \"https://rubygems.org\"").unwrap();
        writeln!(file, "ruby \"3.3.0\"").unwrap();

        let detector = GemfileDetector;
        let version = detector.detect(temp_dir.path()).unwrap();

        assert_eq!(version, Version::new(3, 3, 0));
    }

    #[test]
    fn test_returns_none_when_no_gemfile() {
        let temp_dir = TempDir::new().unwrap();

        let detector = GemfileDetector;
        assert!(detector.detect(temp_dir.path()).is_none());
    }

    #[test]
    fn test_returns_none_when_no_ruby_declaration() {
        let temp_dir = TempDir::new().unwrap();
        let gemfile_path = temp_dir.path().join("Gemfile");
        let mut file = std::fs::File::create(&gemfile_path).unwrap();
        writeln!(file, "source 'https://rubygems.org'").unwrap();
        writeln!(file, "gem 'rails'").unwrap();

        let detector = GemfileDetector;
        assert!(detector.detect(temp_dir.path()).is_none());
    }

    #[test]
    fn test_extract_quoted_version() {
        assert_eq!(
            GemfileDetector::extract_quoted_version("ruby '3.2.5'"),
            Some("3.2.5".to_string())
        );
        assert_eq!(
            GemfileDetector::extract_quoted_version("ruby \"3.1.0\""),
            Some("3.1.0".to_string())
        );
        assert_eq!(
            GemfileDetector::extract_quoted_version("ruby '3.0.0' # comment"),
            Some("3.0.0".to_string())
        );
        assert_eq!(GemfileDetector::extract_quoted_version("ruby 3.2.5"), None);
        assert_eq!(GemfileDetector::extract_quoted_version("gem 'rails'"), None);
    }

    #[test]
    fn test_name() {
        assert_eq!(GemfileDetector.name(), "Gemfile");
    }
}
