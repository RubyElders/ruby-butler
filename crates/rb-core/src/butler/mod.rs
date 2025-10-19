use crate::bundler::{BundlerRuntime, BundlerRuntimeDetector};
use crate::gems::GemRuntime;
use crate::ruby::{RubyDiscoveryError, RubyRuntime, RubyRuntimeDetector};
use colored::*;
use home;
use log::{debug, info};
use semver::Version;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

pub mod command;
pub mod runtime_provider;

pub use command::Command;
pub use runtime_provider::RuntimeProvider;

/// Errors that can occur during ButlerRuntime operations
#[derive(Debug, Clone)]
pub enum ButlerError {
    /// The specified rubies directory does not exist
    RubiesDirectoryNotFound(PathBuf),
    /// No suitable Ruby installation found
    NoSuitableRuby(String),
    /// Specified command was not found in the environment
    CommandNotFound(String),
    /// General error with message
    General(String),
}

impl std::fmt::Display for ButlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ButlerError::RubiesDirectoryNotFound(path) => {
                write!(
                    f,
                    "Directory not found: {}. No Rubies were detected and there's nothing Butler can help with. Please install Ruby using ruby-install or similar tool to the expected path.",
                    path.display()
                )
            }
            ButlerError::NoSuitableRuby(msg) => {
                write!(f, "No suitable Ruby installation found: {}", msg)
            }
            ButlerError::CommandNotFound(command) => {
                write!(
                    f,
                    "Command not found: {}. The specified command is not available in the current environment.",
                    command
                )
            }
            ButlerError::General(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl std::error::Error for ButlerError {}

/// Enhanced ButlerRuntime that serves as the main orchestrator for Ruby environments.
/// Handles discovery, selection, and composition of Ruby installations, gem environments,
/// and bundler projects with distinguished precision.
#[derive(Debug, Clone)]
pub struct ButlerRuntime {
    // Core runtime components
    ruby_runtime: RubyRuntime,
    gem_runtime: Option<GemRuntime>,
    bundler_runtime: Option<BundlerRuntime>,

    // Discovery context
    rubies_dir: PathBuf,
    current_dir: PathBuf,
    ruby_installations: Vec<RubyRuntime>,
    requested_ruby_version: Option<String>,
    required_ruby_version: Option<Version>,
    gem_base_dir: Option<PathBuf>,
}

impl ButlerRuntime {
    /// Create a simple ButlerRuntime with just Ruby and Gem runtimes (for backward compatibility)
    pub fn new(ruby_runtime: RubyRuntime, gem_runtime: Option<GemRuntime>) -> Self {
        debug!(
            "Creating basic ButlerRuntime with Ruby: {} {}",
            ruby_runtime.kind.as_str(),
            ruby_runtime.version
        );

        if let Some(ref gem_runtime) = gem_runtime {
            debug!(
                "Including GemRuntime with gem_home: {}",
                gem_runtime.gem_home.display()
            );
        } else {
            debug!("No GemRuntime provided");
        }

        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let rubies_dir = PathBuf::from(".");

        Self {
            ruby_runtime,
            gem_runtime,
            bundler_runtime: None,
            rubies_dir,
            current_dir,
            ruby_installations: vec![],
            requested_ruby_version: None,
            required_ruby_version: None,
            gem_base_dir: None,
        }
    }

    /// Perform comprehensive environment discovery and create a fully composed ButlerRuntime
    pub fn discover_and_compose(
        rubies_dir: PathBuf,
        requested_ruby_version: Option<String>,
    ) -> Result<Self, ButlerError> {
        Self::discover_and_compose_with_gem_base(rubies_dir, requested_ruby_version, None, false)
    }

    /// Perform comprehensive environment discovery with optional custom gem base directory
    pub fn discover_and_compose_with_gem_base(
        rubies_dir: PathBuf,
        requested_ruby_version: Option<String>,
        gem_base_dir: Option<PathBuf>,
        skip_bundler: bool,
    ) -> Result<Self, ButlerError> {
        let current_dir = env::current_dir().map_err(|e| {
            ButlerError::General(format!("Unable to determine current directory: {}", e))
        })?;

        debug!("Starting comprehensive environment discovery");
        debug!("Rubies directory: {}", rubies_dir.display());
        debug!("Current directory: {}", current_dir.display());
        debug!("Requested Ruby version: {:?}", requested_ruby_version);

        // Step 1: Discover Ruby installations
        debug!("Discovering Ruby installations");
        let ruby_installations =
            RubyRuntimeDetector::discover(&rubies_dir).map_err(|e| match e {
                RubyDiscoveryError::DirectoryNotFound(path) => {
                    ButlerError::RubiesDirectoryNotFound(path)
                }
                RubyDiscoveryError::IoError(msg) => {
                    ButlerError::General(format!("Failed to discover Ruby installations: {}", msg))
                }
            })?;

        info!("Found {} Ruby installations", ruby_installations.len());

        // Step 2: Detect bundler environment (skip if requested)
        let bundler_runtime = if skip_bundler {
            debug!("Bundler detection skipped (--no-bundler flag set)");
            None
        } else {
            debug!("Detecting bundler environment");
            match BundlerRuntimeDetector::discover(&current_dir) {
                Ok(Some(bundler)) => {
                    debug!(
                        "Bundler environment detected at: {}",
                        bundler.root.display()
                    );
                    Some(bundler)
                }
                Ok(None) => {
                    debug!("No bundler environment detected");
                    None
                }
                Err(e) => {
                    debug!("Error detecting bundler environment: {}", e);
                    None
                }
            }
        };

        // Step 3: Extract version requirements from bundler
        let required_ruby_version = bundler_runtime
            .as_ref()
            .and_then(|bundler| bundler.ruby_version());

        // Step 4: Select the most appropriate Ruby installation
        let selected_ruby = Self::select_ruby_runtime(
            &ruby_installations,
            &requested_ruby_version,
            &required_ruby_version,
        )
        .ok_or_else(|| {
            ButlerError::NoSuitableRuby("No suitable Ruby installation found".to_string())
        })?;

        // Step 5: Create gem runtime (using custom base directory if provided)
        let gem_runtime = if let Some(ref custom_gem_base) = gem_base_dir {
            debug!(
                "Using custom gem base directory: {}",
                custom_gem_base.display()
            );
            Some(selected_ruby.gem_runtime_for_base(custom_gem_base))
        } else {
            match selected_ruby.infer_gem_runtime() {
                Ok(gem_runtime) => {
                    debug!(
                        "Successfully inferred gem runtime: {}",
                        gem_runtime.gem_home.display()
                    );
                    Some(gem_runtime)
                }
                Err(e) => {
                    debug!("Failed to infer gem runtime: {}", e);
                    None
                }
            }
        };

        info!(
            "Environment composition complete: Ruby {}, Gem runtime: {}, Bundler: {}",
            selected_ruby.version,
            if gem_runtime.is_some() {
                "available"
            } else {
                "unavailable"
            },
            if bundler_runtime.is_some() {
                "detected"
            } else {
                "not detected"
            }
        );

        Ok(Self {
            ruby_runtime: selected_ruby,
            gem_runtime,
            bundler_runtime,
            rubies_dir,
            current_dir,
            ruby_installations,
            requested_ruby_version,
            required_ruby_version,
            gem_base_dir,
        })
    }

    /// Select the most appropriate Ruby runtime based on requirements
    fn select_ruby_runtime(
        rubies: &[RubyRuntime],
        requested_version: &Option<String>,
        required_version: &Option<Version>,
    ) -> Option<RubyRuntime> {
        if rubies.is_empty() {
            return None;
        }

        if let Some(requested) = requested_version {
            // Use explicitly requested version
            match Version::parse(requested) {
                Ok(req_version) => {
                    let found = rubies.iter().find(|r| r.version == req_version).cloned();

                    if found.is_none() {
                        println!(
                            "{}",
                            format!(
                                "Requested Ruby version {} not found in available installations",
                                requested
                            )
                            .yellow()
                        );
                    }
                    return found;
                }
                Err(e) => {
                    println!(
                        "{}",
                        format!("Invalid Ruby version format '{}': {}", requested, e).red()
                    );
                    return None;
                }
            }
        } else if let Some(required_version) = required_version {
            // Use version from bundler environment
            let found = rubies
                .iter()
                .find(|r| r.version == *required_version)
                .cloned();

            if let Some(ruby) = found {
                return Some(ruby);
            } else {
                println!("{}", format!("Required Ruby version {} (from bundler environment) not found in available installations", required_version).yellow());
                println!(
                    "{}",
                    "   Falling back to latest available Ruby installation".bright_black()
                );
                // Fall through to latest selection
            }
        }

        // Use latest available Ruby
        rubies.iter().max_by_key(|r| &r.version).cloned()
    }

    /// Accessor methods for the discovery context
    pub fn rubies_dir(&self) -> &PathBuf {
        &self.rubies_dir
    }

    pub fn current_dir(&self) -> &PathBuf {
        &self.current_dir
    }

    pub fn ruby_installations(&self) -> &[RubyRuntime] {
        &self.ruby_installations
    }

    pub fn requested_ruby_version(&self) -> Option<&str> {
        self.requested_ruby_version.as_deref()
    }

    pub fn selected_ruby(&self) -> &RubyRuntime {
        &self.ruby_runtime
    }

    pub fn bundler_runtime(&self) -> Option<&BundlerRuntime> {
        self.bundler_runtime.as_ref()
    }

    pub fn gem_runtime(&self) -> Option<&GemRuntime> {
        self.gem_runtime.as_ref()
    }

    pub fn gem_base_dir(&self) -> Option<&PathBuf> {
        self.gem_base_dir.as_ref()
    }

    pub fn bundler_environment(&self) -> Option<&BundlerRuntime> {
        self.bundler_runtime.as_ref()
    }

    /// Check if we have a usable Ruby environment
    pub fn has_ruby_environment(&self) -> bool {
        true // We always have a selected ruby in ButlerRuntime
    }

    /// Display appropriate error messages for missing Ruby installations
    pub fn display_no_ruby_error(&self) {
        println!(
            "{}",
            "⚠️  No Ruby installations were found in your environment.".yellow()
        );
        println!();
        println!(
            "{}",
            "Please ensure you have Ruby installed and available in the search directory.".dimmed()
        );
    }

    pub fn display_no_suitable_ruby_error(&self) {
        println!(
            "{}",
            "⚠️  No suitable Ruby version found for the requested criteria.".yellow()
        );
        println!();
        if let Some(requested) = &self.requested_ruby_version {
            println!(
                "{}",
                format!("Requested version: {}", requested).bright_blue()
            );
        }
        if let Some(required) = &self.required_ruby_version {
            println!(
                "{}",
                format!("Required version (from bundler): {}", required).bright_blue()
            );
        }
        println!("{}", "Available versions:".bright_blue());
        for ruby in &self.ruby_installations {
            println!("  - {}", ruby.version.to_string().cyan());
        }
    }

    /// Returns a list of bin directories from both ruby and gem runtimes
    /// Gem bin directory comes first (higher priority) if present, then Ruby bin directory
    pub fn bin_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // Gem runtime bin dir first (highest priority) - for user-installed tools
        if let Some(ref gem_runtime) = self.gem_runtime {
            debug!(
                "Adding gem bin directory to PATH: {}",
                gem_runtime.gem_bin.display()
            );
            dirs.push(gem_runtime.gem_bin.clone());
        }

        // Ruby runtime bin dir second - for core Ruby executables
        let ruby_bin = self.ruby_runtime.bin_dir();
        debug!("Adding ruby bin directory to PATH: {}", ruby_bin.display());
        dirs.push(ruby_bin);

        debug!("Total bin directories: {}", dirs.len());
        dirs
    }

    /// Returns a list of gem directories from both ruby and gem runtimes
    pub fn gem_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // Ruby runtime always has a lib dir for gems
        let ruby_lib = self.ruby_runtime.lib_dir();
        debug!("Adding ruby lib directory for gems: {}", ruby_lib.display());
        dirs.push(ruby_lib);

        if let Some(ref gem_runtime) = self.gem_runtime {
            debug!(
                "Adding gem home directory: {}",
                gem_runtime.gem_home.display()
            );
            dirs.push(gem_runtime.gem_home.clone());
        }

        debug!("Total gem directories: {}", dirs.len());
        dirs
    }

    /// Returns the gem_home from GemRuntime if present, otherwise returns None
    pub fn gem_home(&self) -> Option<PathBuf> {
        let result = self
            .gem_runtime
            .as_ref()
            .map(|gem_runtime| gem_runtime.gem_home.clone());

        if let Some(ref gem_home) = result {
            debug!("Gem home directory: {}", gem_home.display());
        } else {
            debug!("No gem home directory (no GemRuntime)");
        }

        result
    }

    /// Build PATH string with bin directories prepended to the existing PATH
    pub fn build_path(&self, existing_path: Option<String>) -> String {
        debug!("Building PATH environment variable");

        let mut path_parts = Vec::new();

        // Add our bin directories first
        for bin_dir in self.bin_dirs() {
            let bin_str = bin_dir.display().to_string();
            debug!("Adding to PATH: {}", bin_str);
            path_parts.push(bin_str);
        }

        // Add existing PATH if provided
        if let Some(existing) = existing_path {
            debug!("Appending existing PATH: {}", existing);
            path_parts.push(existing);
        } else {
            debug!("No existing PATH provided");
        }

        // On Windows, use semicolon; on Unix, use colon
        let separator = if cfg!(windows) { ";" } else { ":" };
        let result = path_parts.join(separator);

        debug!("Final PATH: {}", result);
        result
    }

    /// Compose environment variables like chruby does
    /// Returns a HashMap with PATH, GEM_HOME, GEM_PATH, and bundler variables set appropriately
    pub fn env_vars(&self, existing_path: Option<String>) -> HashMap<String, String> {
        debug!("Composing environment variables");

        let mut env = HashMap::new();

        // Set PATH with our bin directories prepended
        let path = self.build_path(existing_path);
        env.insert("PATH".to_string(), path);

        // Set GEM_HOME and GEM_PATH if we have a gem runtime
        if let Some(gem_home) = self.gem_home() {
            let gem_home_str = gem_home.display().to_string();
            debug!("Setting GEM_HOME: {}", gem_home_str);
            env.insert("GEM_HOME".to_string(), gem_home_str.clone());

            // GEM_PATH follows chruby pattern: GEM_HOME:GEM_ROOT
            let mut gem_path_parts = vec![gem_home_str];

            // Add all gem directories (GEM_ROOT from Ruby runtime)
            for gem_dir in self.gem_dirs() {
                let gem_dir_str = gem_dir.display().to_string();
                if !gem_path_parts.contains(&gem_dir_str) {
                    gem_path_parts.push(gem_dir_str);
                }
            }

            let separator = if cfg!(windows) { ";" } else { ":" };
            let gem_path = gem_path_parts.join(separator);
            debug!("Setting GEM_PATH: {}", gem_path);
            env.insert("GEM_PATH".to_string(), gem_path);
        } else {
            debug!("No GEM_HOME available - skipping GEM_HOME and GEM_PATH");
        }

        // Set bundler-specific environment variables if bundler runtime is detected
        if let Some(bundler_runtime) = &self.bundler_runtime {
            let gemfile_path = bundler_runtime.gemfile_path();
            let app_config_dir = bundler_runtime.app_config_dir();

            debug!("Setting BUNDLE_GEMFILE: {}", gemfile_path.display());
            env.insert(
                "BUNDLE_GEMFILE".to_string(),
                gemfile_path.display().to_string(),
            );

            debug!("Setting BUNDLE_APP_CONFIG: {}", app_config_dir.display());
            env.insert(
                "BUNDLE_APP_CONFIG".to_string(),
                app_config_dir.display().to_string(),
            );
        } else {
            debug!("No bundler runtime detected - skipping bundler environment variables");
        }

        debug!("Environment composition complete: {} variables", env.len());
        env
    }

    /// Convenience function to create a ButlerRuntime by discovering and selecting Ruby
    /// from a directory. Uses latest Ruby if no version is specified.
    ///
    /// This is a backward compatibility method - prefer discover_and_compose for full context.
    pub fn discover_and_create(
        search_dir: &Path,
        requested_version: Option<&str>,
    ) -> Result<Self, ButlerError> {
        debug!(
            "Starting Ruby discovery process in: {}",
            search_dir.display()
        );

        let requested = requested_version.map(|s| s.to_string());
        Self::discover_and_compose(search_dir.to_path_buf(), requested)
    }

    /// Get the default rubies directory (~/.rubies)
    pub fn default_rubies_dir() -> Result<PathBuf, ButlerError> {
        let home_dir = home::home_dir().ok_or_else(|| {
            ButlerError::General("Could not determine home directory".to_string())
        })?;
        Ok(home_dir.join(".rubies"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gems::GemRuntime;
    use crate::ruby::{RubyRuntime, RubyType};
    use semver::Version;
    use std::path::Path;

    fn create_ruby_runtime(version: &str, root: &str) -> RubyRuntime {
        RubyRuntime::new(RubyType::CRuby, Version::parse(version).unwrap(), root)
    }

    #[test]
    fn test_butler_runtime_with_only_ruby() {
        let ruby = create_ruby_runtime("3.2.1", "/opt/ruby-3.2.1");
        let butler = ButlerRuntime::new(ruby.clone(), None);

        // Test bin_dirs - should have only ruby bin dir
        let bin_dirs = butler.bin_dirs();
        assert_eq!(bin_dirs.len(), 1);
        assert_eq!(bin_dirs[0], ruby.bin_dir());

        // Test gem_dirs - should have only ruby lib dir
        let gem_dirs = butler.gem_dirs();
        assert_eq!(gem_dirs.len(), 1);
        assert_eq!(gem_dirs[0], ruby.lib_dir());

        // Test gem_home should be None when no GemRuntime
        assert_eq!(butler.gem_home(), None);
    }

    #[test]
    fn test_butler_runtime_with_ruby_and_gem() {
        let ruby = create_ruby_runtime("3.2.1", "/opt/ruby-3.2.1");
        let gem_base = Path::new("/home/user/.gem");
        let gem_runtime = GemRuntime::for_base_dir(gem_base, &ruby.version);

        let butler = ButlerRuntime::new(ruby.clone(), Some(gem_runtime.clone()));

        // Test bin_dirs - should have gem first, then ruby bin dirs
        let bin_dirs = butler.bin_dirs();
        assert_eq!(bin_dirs.len(), 2);
        assert_eq!(bin_dirs[0], gem_runtime.gem_bin); // Gem bin dir first (higher priority)
        assert_eq!(bin_dirs[1], ruby.bin_dir()); // Ruby bin dir second

        // Test gem_dirs - should have both ruby and gem dirs
        let gem_dirs = butler.gem_dirs();
        assert_eq!(gem_dirs.len(), 2);
        assert_eq!(gem_dirs[0], ruby.lib_dir());
        assert_eq!(gem_dirs[1], gem_runtime.gem_home);

        // Test gem_home should return the gem runtime's gem_home
        assert_eq!(butler.gem_home(), Some(gem_runtime.gem_home));
    }

    #[test]
    fn test_build_path_without_existing() {
        let ruby = create_ruby_runtime("3.1.0", "/opt/ruby-3.1.0");
        let butler = ButlerRuntime::new(ruby.clone(), None);

        let path = butler.build_path(None);
        assert_eq!(path, ruby.bin_dir().display().to_string());
    }

    #[test]
    fn test_build_path_with_existing() {
        let ruby = create_ruby_runtime("3.1.0", "/opt/ruby-3.1.0");
        let butler = ButlerRuntime::new(ruby.clone(), None);

        let path = butler.build_path(Some("/usr/bin:/bin".to_string()));

        let separator = if cfg!(windows) { ";" } else { ":" };
        let expected = format!("{}{}/usr/bin:/bin", ruby.bin_dir().display(), separator);
        assert_eq!(path, expected);
    }

    #[test]
    fn test_build_path_with_multiple_bin_dirs() {
        let ruby = create_ruby_runtime("3.1.0", "/opt/ruby-3.1.0");
        let gem_base = Path::new("/home/user/.gem");
        let gem_runtime = GemRuntime::for_base_dir(gem_base, &ruby.version);

        let butler = ButlerRuntime::new(ruby.clone(), Some(gem_runtime.clone()));
        let path = butler.build_path(Some("/usr/bin".to_string()));

        let separator = if cfg!(windows) { ";" } else { ":" };
        let expected = format!(
            "{}{}{}{}/usr/bin",
            gem_runtime.gem_bin.display(), // Gem bin first (highest priority)
            separator,
            ruby.bin_dir().display(), // Ruby bin second
            separator
        );
        assert_eq!(path, expected);
    }
}
