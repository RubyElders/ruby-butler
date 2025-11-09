use crate::butler::Command;
use crate::butler::runtime_provider::RuntimeProvider;
use crate::ruby::RubyVersionExt;
use log::debug;
use semver::Version;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundlerRuntime {
    /// Root directory containing the Gemfile
    pub root: PathBuf,
    /// Ruby version for this bundler context
    pub ruby_version: Version,
}

impl BundlerRuntime {
    pub fn new(root: impl AsRef<Path>, ruby_version: Version) -> Self {
        let root = root.as_ref().to_path_buf();

        debug!(
            "Creating BundlerRuntime for root: {} with Ruby {}",
            root.display(),
            ruby_version
        );

        Self { root, ruby_version }
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

    /// Returns the ruby-specific vendor directory (.rb/vendor/bundler/ruby/X.Y.0)
    /// Uses Ruby ABI version (major.minor.0) for compatibility grouping
    pub fn ruby_vendor_dir(&self, ruby_version: &Version) -> PathBuf {
        self.vendor_dir()
            .join("ruby")
            .join(ruby_version.ruby_abi_version())
    }

    /// Detect Ruby version from .ruby-version file or Gemfile ruby declaration
    pub fn ruby_version(&self) -> Option<Version> {
        use crate::ruby::CompositeDetector;
        let detector = CompositeDetector::bundler();
        detector.detect(&self.root)
    }

    /// Returns the bin directory where bundler-installed executables live
    /// Path: .rb/vendor/bundler/ruby/X.Y.0/bin
    pub fn bin_dir(&self) -> PathBuf {
        let bin_dir = self.ruby_vendor_dir(&self.ruby_version).join("bin");
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
    /// Also updates Gemfile.lock if check passes to handle removed gems
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

                // If check passes, update lockfile to handle removed gems
                if is_synced {
                    debug!("Bundle check passed, updating lockfile to match Gemfile");
                    self.update_lockfile_quietly(butler_runtime)?;
                }

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

    /// Update Gemfile.lock to match Gemfile quietly (no output)
    /// Used by check_sync to ensure lockfile is up to date
    fn update_lockfile_quietly(
        &self,
        butler_runtime: &crate::butler::ButlerRuntime,
    ) -> std::io::Result<()> {
        debug!("Quietly updating Gemfile.lock to match Gemfile");

        // Run bundle lock --local to regenerate lockfile based on Gemfile
        // Uses --local to avoid network access since bundle check already passed
        let output = Command::new("bundle")
            .arg("lock")
            .arg("--local")
            .current_dir(&self.root)
            .output_with_context(butler_runtime)?;

        if output.status.success() {
            debug!("Gemfile.lock updated successfully");
            Ok(())
        } else {
            // Silently ignore errors - lockfile update is best-effort
            // The bundle check already passed, so environment is functional
            debug!(
                "Bundle lock failed but continuing (exit code: {})",
                output.status.code().unwrap_or(-1)
            );
            Ok(())
        }
    }

    /// Update Gemfile.lock to match Gemfile (handles removed gems)
    /// Used by sync command with output streaming
    fn update_lockfile<F>(
        &self,
        butler_runtime: &crate::butler::ButlerRuntime,
        output_handler: &mut F,
    ) -> std::io::Result<()>
    where
        F: FnMut(&str),
    {
        debug!("Updating Gemfile.lock to match Gemfile");

        // Run bundle lock --local to regenerate lockfile based on Gemfile
        // Uses --local to avoid network access since bundle check already passed
        let output = Command::new("bundle")
            .arg("lock")
            .arg("--local")
            .current_dir(&self.root)
            .output_with_context(butler_runtime)?;

        // Stream output to handler
        if !output.stdout.is_empty() {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            for line in stdout_str.lines() {
                output_handler(line);
            }
        }

        if !output.stderr.is_empty() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            for line in stderr_str.lines() {
                eprintln!("{}", line);
            }
        }

        if output.status.success() {
            debug!("Gemfile.lock updated successfully");
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "Bundle lock failed (exit code: {})",
                output.status.code().unwrap_or(-1)
            )))
        }
    }

    /// Synchronize the bundler environment (configure path, check, and install if needed)
    pub fn synchronize<F>(
        &self,
        butler_runtime: &crate::butler::ButlerRuntime,
        mut output_handler: F,
    ) -> std::io::Result<SyncResult>
    where
        F: FnMut(&str),
    {
        debug!("Starting bundler synchronization");

        // Step 1: Check if already synchronized
        // Note: check_sync already updates lockfile quietly, but for sync command
        // we want to show output, so we call update_lockfile explicitly
        match self.check_sync(butler_runtime)? {
            true => {
                debug!("Bundler environment already synchronized");

                // For sync command, show the lockfile update output
                self.update_lockfile(butler_runtime, &mut output_handler)?;

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
            let bin = self.ruby_vendor_dir(&self.ruby_version).join("bin");
            debug!("BundlerRuntime bin directory: {}", bin.display());
            Some(bin)
        } else {
            debug!("BundlerRuntime not configured, no bin directory available");
            None
        }
    }

    fn gem_dir(&self) -> Option<PathBuf> {
        if self.is_configured() {
            let vendor = self.ruby_vendor_dir(&self.ruby_version);
            debug!("BundlerRuntime gem directory: {}", vendor.display());
            Some(vendor)
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

    // Helper to create BundlerRuntime with a default Ruby version for testing
    fn bundler_rt(root: impl AsRef<Path>) -> BundlerRuntime {
        BundlerRuntime::new(root, Version::new(3, 3, 7))
    }

    #[test]
    fn new_creates_proper_paths() {
        let root = Path::new("/home/user/my-app");
        let br = bundler_rt(root);

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
        // When no ruby/X.Y.Z structure exists, falls back to vendor/bundler/bin
        let br = bundler_rt("/home/user/project");
        // bin_dir should include Ruby minor version: .rb/vendor/bundler/ruby/3.3.0/bin
        let expected = Path::new("/home/user/project/.rb/vendor/bundler/ruby/3.3.0/bin");
        assert_eq!(br.bin_dir(), expected);
    }

    #[test]
    fn bin_dir_finds_versioned_ruby_directory() -> io::Result<()> {
        // When ruby/X.Y.Z/bin structure exists, uses that instead
        let sandbox = BundlerSandbox::new()?;
        let project_root = sandbox.root().join("versioned-project");
        fs::create_dir_all(&project_root)?;

        // Create Gemfile
        fs::write(
            project_root.join("Gemfile"),
            "source 'https://rubygems.org'\n",
        )?;

        // Create versioned ruby bin directory
        let ruby_bin = project_root
            .join(".rb")
            .join("vendor")
            .join("bundler")
            .join("ruby")
            .join("3.3.0")
            .join("bin");
        fs::create_dir_all(&ruby_bin)?;

        let br = BundlerRuntime::new(&project_root);
        assert_eq!(br.bin_dir(), ruby_bin);

        Ok(())
    }

    #[test]
    fn runtime_provider_returns_paths_when_configured() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_bundler_project("configured-app", true)?;
        let br = bundler_rt(&project_dir);

        // Should be configured since we created vendor structure
        assert!(br.is_configured());

        // bin_dir should include Ruby minor version path (X.Y.0)
        let expected_bin = br.vendor_dir().join("ruby").join("3.3.0").join("bin");
        // gem_dir should be the Ruby-minor-specific vendor directory
        let expected_gem = br.vendor_dir().join("ruby").join("3.3.0");

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
        let br = bundler_rt(&project_dir);

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

        let br = bundler_rt(&project_dir);
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

        let br = bundler_rt(&project_dir);
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

        let br = bundler_rt(&project_dir);
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

        let br = bundler_rt(&project_dir);
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

        let br = bundler_rt(&project_dir);
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

        let br = bundler_rt(&project_dir);
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

        let br = bundler_rt(&project_dir);
        assert_eq!(br.ruby_version(), Some(Version::parse("3.2.1").unwrap()));

        Ok(())
    }
}

pub mod detector;
pub use detector::BundlerRuntimeDetector;
