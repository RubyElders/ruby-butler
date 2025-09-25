use crate::butler::Command;
use crate::butler::runtime_provider::RuntimeProvider;
use log::{debug, warn};
use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundlerRuntime {
    /// Root directory containing the Gemfile
    pub root: PathBuf,
}

impl BundlerRuntime {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();

        debug!("Creating BundlerRuntime for root: {}", root.display());

        Self { root }
    }

    /// Returns the full path to the Gemfile
    pub fn gemfile_path(&self) -> PathBuf {
        self.root.join("Gemfile")
    }

    /// Returns the application config directory (.rb)
    pub fn app_config_dir(&self) -> PathBuf {
        self.root.join(".rb")
    }

    /// Returns the vendor bundler directory (.rb/vendor/bundler)
    pub fn vendor_dir(&self) -> PathBuf {
        self.app_config_dir().join("vendor").join("bundler")
    }

    /// Detect Ruby version from .ruby-version file or Gemfile ruby declaration
    pub fn ruby_version(&self) -> Option<Version> {
        // First try .ruby-version file
        if let Some(version) = self.detect_from_ruby_version_file() {
            return Some(version);
        }

        // Then try Gemfile ruby declaration
        if let Some(version) = self.detect_from_gemfile() {
            return Some(version);
        }

        None
    }

    /// Detect Ruby version from .ruby-version file
    fn detect_from_ruby_version_file(&self) -> Option<Version> {
        let ruby_version_path = self.root.join(".ruby-version");
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

    /// Detect Ruby version from Gemfile ruby declaration
    fn detect_from_gemfile(&self) -> Option<Version> {
        let gemfile_path = self.gemfile_path();
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
                debug!("Could not read Gemfile");
                None
            }
        }
    }

    /// Extract version string from ruby declaration line
    fn extract_quoted_version(line: &str) -> Option<String> {
        // Handle both single and double quotes: ruby '3.2.5' or ruby "3.2.5"
        let after_ruby = line.strip_prefix("ruby ")?;
        let trimmed = after_ruby.trim();

        // Single quotes
        if let Some(single_quoted) = trimmed.strip_prefix('\'') {
            if let Some(end_quote) = single_quoted.find('\'') {
                return Some(single_quoted[..end_quote].to_string());
            }
        }

        // Double quotes
        if let Some(double_quoted) = trimmed.strip_prefix('"') {
            if let Some(end_quote) = double_quoted.find('"') {
                return Some(double_quoted[..end_quote].to_string());
            }
        }

        None
    }

    /// Returns the bin directory where bundler-installed executables live
    pub fn bin_dir(&self) -> PathBuf {
        let bin_dir = self.vendor_dir().join("bin");
        debug!("Bundler bin directory: {}", bin_dir.display());
        bin_dir
    }

    /// Returns whether this bundler runtime appears to be configured
    /// (i.e., has vendor directory structure)
    pub fn is_configured(&self) -> bool {
        let vendor_dir = self.vendor_dir();
        let configured = vendor_dir.exists();
        debug!(
            "Bundler runtime configured: {} (vendor dir exists: {})",
            configured,
            vendor_dir.display()
        );
        configured
    }

    /// Check if bundler environment is synchronized (dependencies satisfied)
    pub fn check_sync(
        &self,
        butler_runtime: &crate::butler::ButlerRuntime,
    ) -> std::io::Result<bool> {
        debug!("Checking bundle synchronization status");

        self.configure_local_path(butler_runtime)?;

        // Check if dependencies are satisfied
        let output = Command::new("bundle")
            .arg("check")
            .current_dir(&self.root)
            .output_with_context(butler_runtime);

        match output {
            Ok(output) => {
                let is_synced = output.status.success();
                debug!(
                    "Bundle check result: {} (exit code: {})",
                    is_synced,
                    output.status.code().unwrap_or(-1)
                );
                Ok(is_synced)
            }
            Err(e) => {
                // Check if bundler is not installed
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Bundler executable not found. Please install bundler with: gem install bundler",
                    ))
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Configure bundler to use local vendor directory
    pub fn configure_local_path(
        &self,
        butler_runtime: &crate::butler::ButlerRuntime,
    ) -> std::io::Result<()> {
        debug!(
            "Configuring bundle path to vendor directory: {}",
            self.vendor_dir().display()
        );

        let status = Command::new("bundle")
            .args(["config", "path", "--local"])
            .arg(self.vendor_dir().to_string_lossy().as_ref())
            .current_dir(&self.root)
            .status_with_context(butler_runtime);

        match status {
            Ok(status) => {
                if status.success() {
                    debug!("Successfully configured bundle path");
                    Ok(())
                } else {
                    Err(std::io::Error::other(format!(
                        "Failed to configure bundle path (exit code: {})",
                        status.code().unwrap_or(-1)
                    )))
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Bundler executable not found. Please install bundler with: gem install bundler",
                    ))
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Install bundler dependencies with streaming output
    pub fn install_dependencies<F>(
        &self,
        butler_runtime: &crate::butler::ButlerRuntime,
        mut output_handler: F,
    ) -> std::io::Result<()>
    where
        F: FnMut(&str),
    {
        use std::io::{BufRead, BufReader};
        use std::process::Stdio;

        debug!("Installing bundle dependencies");

        let child_result = Command::new("bundle")
            .arg("install")
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // Capture stderr to analyze errors
            .execute_with_context(butler_runtime);

        let mut child = match child_result {
            Ok(child) => child,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Bundler executable not found. Please install bundler with: gem install bundler",
                    ));
                } else {
                    return Err(e);
                }
            }
        };

        // Stream stdout to the output handler
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let line = line?;
                output_handler(&line);
            }
        }

        // Capture stderr for error analysis
        let mut stderr_content = String::new();
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                let line = line?;
                eprintln!("{}", line); // Still show stderr to user
                stderr_content.push_str(&line);
                stderr_content.push('\n');
            }
        }

        let status = child.wait()?;

        if status.success() {
            debug!("Bundle install completed successfully");
            Ok(())
        } else {
            // Enhance error message with stderr content for better error classification
            let base_error = format!(
                "Bundle install failed (exit code: {})",
                status.code().unwrap_or(-1)
            );

            let enhanced_error = if !stderr_content.trim().is_empty() {
                format!("{}. Error details: {}", base_error, stderr_content.trim())
            } else {
                base_error
            };

            Err(std::io::Error::other(enhanced_error))
        }
    }

    /// Synchronize the bundler environment (configure path, check, and install if needed)
    pub fn synchronize<F>(
        &self,
        butler_runtime: &crate::butler::ButlerRuntime,
        output_handler: F,
    ) -> std::io::Result<SyncResult>
    where
        F: FnMut(&str),
    {
        debug!("Starting bundler synchronization");

        // Step 1: Check if already synchronized
        match self.check_sync(butler_runtime)? {
            true => {
                debug!("Bundler environment already synchronized");
                Ok(SyncResult::AlreadySynced)
            }
            false => {
                debug!("Bundler environment requires synchronization");

                // Step 3: Install dependencies
                self.install_dependencies(butler_runtime, output_handler)?;

                Ok(SyncResult::Synchronized)
            }
        }
    }
}

/// Result of a bundler synchronization operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncResult {
    /// Environment was already synchronized
    AlreadySynced,
    /// Environment was successfully synchronized
    Synchronized,
}

impl RuntimeProvider for BundlerRuntime {
    fn bin_dir(&self) -> Option<PathBuf> {
        if self.is_configured() {
            Some(self.bin_dir())
        } else {
            debug!("BundlerRuntime not configured, no bin directory available");
            None
        }
    }

    fn gem_dir(&self) -> Option<PathBuf> {
        if self.is_configured() {
            Some(self.vendor_dir())
        } else {
            debug!("BundlerRuntime not configured, no gem directory available");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rb_tests::BundlerSandbox;
    use std::io;
    use std::path::Path;

    fn bundler_rt(root: &str) -> BundlerRuntime {
        BundlerRuntime::new(root)
    }

    #[test]
    fn new_creates_proper_paths() {
        let root = Path::new("/home/user/my-app");
        let br = BundlerRuntime::new(root);

        assert_eq!(br.root, root);
        assert_eq!(br.gemfile_path(), root.join("Gemfile"));
        assert_eq!(br.app_config_dir(), root.join(".rb"));
        assert_eq!(
            br.vendor_dir(),
            root.join(".rb").join("vendor").join("bundler")
        );
        assert_eq!(br.ruby_version(), None); // No filesystem access in this test
    }

    #[test]
    fn bin_dir_is_vendor_bin() {
        let br = bundler_rt("/home/user/project");
        let expected = Path::new("/home/user/project/.rb/vendor/bundler/bin");
        assert_eq!(br.bin_dir(), expected);
    }

    #[test]
    fn runtime_provider_returns_paths_when_configured() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_bundler_project("configured-app", true)?;
        let br = BundlerRuntime::new(&project_dir);

        // Should be configured since we created vendor structure
        assert!(br.is_configured());

        let expected_bin = br.vendor_dir().join("bin");
        let expected_gem = br.vendor_dir();

        assert_eq!(
            <BundlerRuntime as RuntimeProvider>::bin_dir(&br),
            Some(expected_bin)
        );
        assert_eq!(
            <BundlerRuntime as RuntimeProvider>::gem_dir(&br),
            Some(expected_gem)
        );

        Ok(())
    }

    #[test]
    fn runtime_provider_returns_none_when_not_configured() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_bundler_project("basic-app", false)?;
        let br = BundlerRuntime::new(&project_dir);

        // Should not be configured since no vendor structure exists
        assert!(!br.is_configured());
        assert_eq!(<BundlerRuntime as RuntimeProvider>::bin_dir(&br), None);
        assert_eq!(<BundlerRuntime as RuntimeProvider>::gem_dir(&br), None);

        Ok(())
    }

    #[test]
    fn detects_ruby_version_from_ruby_version_file() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_bundler_project("version-app", false)?;

        // Add .ruby-version file
        sandbox.add_file(
            format!(
                "{}/{}",
                project_dir.file_name().unwrap().to_str().unwrap(),
                ".ruby-version"
            ),
            "3.2.5",
        )?;

        let br = BundlerRuntime::new(&project_dir);
        assert_eq!(br.ruby_version(), Some(Version::parse("3.2.5").unwrap()));

        Ok(())
    }

    #[test]
    fn detects_ruby_version_from_gemfile_single_quotes() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_dir("gemfile-app")?;

        let gemfile_content = r#"source 'https://rubygems.org'

ruby '3.1.4'

gem 'rails', '~> 7.0'
gem 'pg', '~> 1.4'
"#;
        sandbox.add_file(
            format!(
                "{}/Gemfile",
                project_dir.file_name().unwrap().to_str().unwrap()
            ),
            gemfile_content,
        )?;

        let br = BundlerRuntime::new(&project_dir);
        assert_eq!(br.ruby_version(), Some(Version::parse("3.1.4").unwrap()));

        Ok(())
    }

    #[test]
    fn detects_ruby_version_from_gemfile_double_quotes() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_dir("gemfile-app")?;

        let gemfile_content = r#"source "https://rubygems.org"

ruby "3.3.0"

gem "rails", "~> 7.1"
"#;
        sandbox.add_file(
            format!(
                "{}/Gemfile",
                project_dir.file_name().unwrap().to_str().unwrap()
            ),
            gemfile_content,
        )?;

        let br = BundlerRuntime::new(&project_dir);
        assert_eq!(br.ruby_version(), Some(Version::parse("3.3.0").unwrap()));

        Ok(())
    }

    #[test]
    fn ruby_version_file_takes_precedence_over_gemfile() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_dir("precedence-app")?;

        // Add Gemfile with one version
        let gemfile_content = r#"source 'https://rubygems.org'
ruby '3.1.0'
gem 'rails'
"#;
        sandbox.add_file(
            format!(
                "{}/Gemfile",
                project_dir.file_name().unwrap().to_str().unwrap()
            ),
            gemfile_content,
        )?;

        // Add .ruby-version with different version
        sandbox.add_file(
            format!(
                "{}/{}",
                project_dir.file_name().unwrap().to_str().unwrap(),
                ".ruby-version"
            ),
            "3.2.5",
        )?;

        let br = BundlerRuntime::new(&project_dir);
        // Should prefer .ruby-version
        assert_eq!(br.ruby_version(), Some(Version::parse("3.2.5").unwrap()));

        Ok(())
    }

    #[test]
    fn returns_none_for_invalid_ruby_version() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_dir("invalid-app")?;

        // Add invalid .ruby-version file
        sandbox.add_file(
            format!(
                "{}/{}",
                project_dir.file_name().unwrap().to_str().unwrap(),
                ".ruby-version"
            ),
            "not-a-version",
        )?;

        let br = BundlerRuntime::new(&project_dir);
        assert_eq!(br.ruby_version(), None);

        Ok(())
    }

    #[test]
    fn returns_none_when_no_ruby_version_specified() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_dir("no-version-app")?;

        // Add Gemfile without ruby declaration
        let gemfile_content = r#"source 'https://rubygems.org'

gem 'rails'
gem 'pg'
"#;
        sandbox.add_file(
            format!(
                "{}/Gemfile",
                project_dir.file_name().unwrap().to_str().unwrap()
            ),
            gemfile_content,
        )?;

        let br = BundlerRuntime::new(&project_dir);
        assert_eq!(br.ruby_version(), None);

        Ok(())
    }

    #[test]
    fn handles_whitespace_in_ruby_version_file() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_bundler_project("whitespace-app", false)?;

        // Add .ruby-version file with whitespace
        sandbox.add_file(
            format!(
                "{}/{}",
                project_dir.file_name().unwrap().to_str().unwrap(),
                ".ruby-version"
            ),
            "  3.2.1  \n",
        )?;

        let br = BundlerRuntime::new(&project_dir);
        assert_eq!(br.ruby_version(), Some(Version::parse("3.2.1").unwrap()));

        Ok(())
    }

    #[test]
    fn extract_quoted_version_handles_various_formats() {
        assert_eq!(
            BundlerRuntime::extract_quoted_version("ruby '3.2.5'"),
            Some("3.2.5".to_string())
        );
        assert_eq!(
            BundlerRuntime::extract_quoted_version("ruby \"3.1.4\""),
            Some("3.1.4".to_string())
        );
        assert_eq!(
            BundlerRuntime::extract_quoted_version("ruby  '3.0.0'  "),
            Some("3.0.0".to_string())
        );
        assert_eq!(BundlerRuntime::extract_quoted_version("ruby 3.2.5"), None);
        assert_eq!(BundlerRuntime::extract_quoted_version("gem 'rails'"), None);
    }
}

pub mod detector;
pub use detector::BundlerRuntimeDetector;
