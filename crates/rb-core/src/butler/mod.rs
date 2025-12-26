use crate::bundler::{BundlerRuntime, BundlerRuntimeDetector};
use crate::gems::GemRuntime;
use crate::ruby::{RubyDiscoveryError, RubyRuntime, RubyRuntimeDetector};
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

/// Helper to compose detectors based on environment context during early discovery phase.
///
/// This helper delegates to RuntimeProvider implementations to ensure detector composition
/// logic remains centralized. It creates temporary runtime instances solely to extract
/// their detector composition strategies.
pub struct DetectorComposer;

impl DetectorComposer {
    /// Compose version detector for bundler environment by delegating to BundlerRuntime
    pub fn version_detector_for_bundler() -> crate::ruby::CompositeDetector {
        use crate::bundler::BundlerRuntime;
        use semver::Version;
        use std::path::PathBuf;

        // Create temporary bundler runtime to extract its detector composition
        let temp_runtime = BundlerRuntime::new(PathBuf::new(), Version::new(0, 0, 0));
        temp_runtime.compose_version_detector()
    }

    /// Compose gem path detector for bundler environment by delegating to BundlerRuntime
    ///
    /// Use this when bundler is detected - excludes user gems to maintain bundle isolation
    pub fn gem_path_detector_for_bundler()
    -> crate::gems::gem_path_detector::CompositeGemPathDetector {
        use crate::bundler::BundlerRuntime;
        use semver::Version;
        use std::path::PathBuf;

        // Create temporary bundler runtime to extract its detector composition
        let temp_runtime = BundlerRuntime::new(PathBuf::new(), Version::new(0, 0, 0));
        temp_runtime.compose_gem_path_detector()
    }

    /// Compose gem path detector for standard (non-bundler) environment by delegating to GemRuntime
    ///
    /// Use this when bundler is NOT detected - includes user gems
    pub fn gem_path_detector_standard() -> crate::gems::gem_path_detector::CompositeGemPathDetector
    {
        use crate::gems::GemRuntime;
        use semver::Version;
        use std::path::PathBuf;

        // Create temporary gem runtime to extract its detector composition
        let temp_runtime = GemRuntime::for_base_dir(&PathBuf::new(), &Version::new(0, 0, 0));
        temp_runtime.compose_gem_path_detector()
    }
}

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
    // Core runtime components - all optional now
    ruby_runtime: Option<RubyRuntime>,
    gem_runtime: Option<GemRuntime>,
    bundler_runtime: Option<BundlerRuntime>,

    // Discovery context
    rubies_dir: PathBuf,
    current_dir: PathBuf,
    ruby_installations: Vec<RubyRuntime>,
    requested_ruby_version: Option<String>,
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
            ruby_runtime: Some(ruby_runtime),
            gem_runtime,
            bundler_runtime: None,
            rubies_dir,
            current_dir,
            ruby_installations: vec![],
            requested_ruby_version: None,
            gem_base_dir: None,
        }
    }

    /// Create an empty ButlerRuntime when no Ruby installations are found
    /// This allows the runtime to exist even without Ruby, with methods failing when features are accessed
    pub fn empty(rubies_dir: PathBuf, current_dir: PathBuf) -> Self {
        debug!("Creating empty ButlerRuntime (no Ruby installations found)");

        Self {
            ruby_runtime: None,
            gem_runtime: None,
            bundler_runtime: None,
            rubies_dir,
            current_dir,
            ruby_installations: vec![],
            requested_ruby_version: None,
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

        Self::discover_and_compose_with_current_dir(
            rubies_dir,
            requested_ruby_version,
            gem_base_dir,
            skip_bundler,
            current_dir,
        )
    }

    /// Internal method: Perform comprehensive environment discovery with explicit current directory
    ///
    /// This method accepts the current directory as a parameter instead of reading it from
    /// the environment, which makes it suitable for testing without global state mutation.
    ///
    /// Note: This method is primarily intended for testing but is made public to allow
    /// flexible usage patterns where the current directory needs to be explicitly controlled.
    pub fn discover_and_compose_with_current_dir(
        rubies_dir: PathBuf,
        requested_ruby_version: Option<String>,
        gem_base_dir: Option<PathBuf>,
        skip_bundler: bool,
        current_dir: PathBuf,
    ) -> Result<Self, ButlerError> {
        debug!("Starting comprehensive environment discovery");
        debug!("Rubies directory: {}", rubies_dir.display());
        debug!("Current directory: {}", current_dir.display());
        debug!("Requested Ruby version: {:?}", requested_ruby_version);

        // Step 1: Discover Ruby installations
        debug!("Discovering Ruby installations");
        let ruby_installations = match RubyRuntimeDetector::discover(&rubies_dir) {
            Ok(installations) => installations,
            Err(RubyDiscoveryError::DirectoryNotFound(path)) => {
                // If the rubies directory doesn't exist, return a proper error
                return Err(ButlerError::RubiesDirectoryNotFound(path));
            }
            Err(e) => {
                // Other errors (like I/O errors) return empty list to gracefully degrade
                debug!("Ruby discovery failed: {:?}", e);
                vec![]
            }
        };

        info!("Found {} Ruby installations", ruby_installations.len());

        // If no Ruby installations found, return empty runtime
        if ruby_installations.is_empty() {
            debug!("No Ruby installations found, returning empty runtime");
            return Ok(Self::empty(rubies_dir, current_dir));
        }

        // Step 2: Detect bundler environment (skip if requested)
        let bundler_root = if skip_bundler {
            debug!("Bundler detection skipped (--no-bundler flag set)");
            None
        } else {
            debug!("Detecting bundler environment");
            match BundlerRuntimeDetector::discover(&current_dir) {
                Ok(Some(bundler_root)) => {
                    debug!(
                        "Bundler environment detected at: {}",
                        bundler_root.display()
                    );
                    Some(bundler_root)
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

        // Step 3: Extract version requirements from project directory
        let required_ruby_version = if bundler_root.is_some() {
            let detector = DetectorComposer::version_detector_for_bundler();
            detector.detect(&current_dir)
        } else {
            None
        };

        // Step 4: Select the most appropriate Ruby installation
        let selected_ruby = Self::select_ruby_runtime(
            &ruby_installations,
            &requested_ruby_version,
            &required_ruby_version,
        );

        // If no Ruby selected, handle appropriately
        let Some(selected_ruby) = selected_ruby else {
            // If a specific version was requested but not found, return error
            if let Some(requested) = &requested_ruby_version {
                return Err(ButlerError::NoSuitableRuby(format!(
                    "Requested Ruby version {} not found",
                    requested
                )));
            }
            // Otherwise return empty runtime
            debug!("No suitable Ruby selected, returning empty runtime");
            return Ok(Self::empty(rubies_dir, current_dir));
        };

        // Step 5: Create bundler runtime with selected Ruby version (if bundler detected)
        let bundler_runtime =
            bundler_root.map(|root| BundlerRuntime::new(root, selected_ruby.version.clone()));

        // Step 6: Detect and compose gem path configuration
        // Uses detector pattern to determine appropriate gem directories
        // Choose detector based on whether bundler is active
        use crate::gems::gem_path_detector::GemPathContext;

        let gem_detector = if bundler_runtime.is_some() {
            // Bundler detected: use bundler-specific composition (no user gems)
            DetectorComposer::gem_path_detector_for_bundler()
        } else {
            // No bundler: use standard composition (includes user gems)
            DetectorComposer::gem_path_detector_standard()
        };

        let gem_context =
            GemPathContext::new(&current_dir, &selected_ruby, gem_base_dir.as_deref());

        let gem_path_config = gem_detector.detect(&gem_context);
        debug!(
            "Detected gem path with {} directories",
            gem_path_config.gem_dirs().len()
        );

        // Create primary gem runtime from detected configuration
        let gem_runtime = gem_path_config.gem_home().map(|gem_home| {
            GemRuntime::for_base_dir(
                gem_home.parent().unwrap_or(gem_home),
                &selected_ruby.version,
            )
        });

        info!(
            "Environment composition complete: Ruby {}, Gem directories: {}, Bundler: {}",
            selected_ruby.version,
            gem_path_config.gem_dirs().len(),
            if bundler_runtime.is_some() {
                "detected"
            } else {
                "not detected"
            }
        );

        Ok(Self {
            ruby_runtime: Some(selected_ruby),
            gem_runtime,
            bundler_runtime,
            rubies_dir,
            current_dir,
            ruby_installations,
            requested_ruby_version,
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
                    return found;
                }
                Err(_e) => {
                    debug!("Invalid Ruby version format: {}", requested);
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
                debug!(
                    "Required Ruby version {} not found, falling back to latest",
                    required_version
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

    pub fn selected_ruby(&self) -> Result<&RubyRuntime, ButlerError> {
        self.ruby_runtime.as_ref().ok_or_else(|| {
            ButlerError::NoSuitableRuby(
                "No Ruby installation available. Please install Ruby first.".to_string(),
            )
        })
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

    /// Returns a list of bin directories from all active runtimes
    ///
    /// When in bundler context (bundler_runtime present):
    /// 1. Bundler bin directory (.rb/vendor/bundler/ruby/X.Y.Z/bin) - bundled gems only
    /// 2. Ruby bin directory (~/.rubies/ruby-X.Y.Z/bin) - core executables
    ///
    /// When NOT in bundler context:
    /// 1. Gem bin directory (~/.gem/ruby/X.Y.Z/bin) - user-installed gems
    /// 2. Ruby bin directory (~/.rubies/ruby-X.Y.Z/bin) - core executables
    ///
    /// NOTE: User gems are NOT available in bundler context for proper isolation.
    /// Use --no-bundler to opt out of bundler context and access user gems.
    pub fn bin_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // Bundler runtime bin dir first (if in bundler context)
        if let Some(ref bundler_runtime) = self.bundler_runtime
            && let Some(bundler_bin) = RuntimeProvider::bin_dir(bundler_runtime)
        {
            debug!(
                "Adding bundler bin directory to PATH: {}",
                bundler_bin.display()
            );
            dirs.push(bundler_bin);
        }

        // Gem runtime bin dir (only if NOT in bundler context for isolation)
        if self.bundler_runtime.is_none() {
            if let Some(ref gem_runtime) = self.gem_runtime {
                debug!(
                    "Adding gem bin directory to PATH: {}",
                    gem_runtime.gem_bin.display()
                );
                dirs.push(gem_runtime.gem_bin.clone());
            }
        } else {
            debug!("Skipping user gem bin directory (bundler isolation)");
        }

        // Ruby runtime bin dir always included (if Ruby available)
        if let Some(ref ruby_runtime) = self.ruby_runtime {
            let ruby_bin = ruby_runtime.bin_dir();
            debug!("Adding ruby bin directory to PATH: {}", ruby_bin.display());
            dirs.push(ruby_bin);
        } else {
            debug!("No Ruby runtime available, skipping ruby bin directory");
        }

        debug!("Total bin directories: {}", dirs.len());
        dirs
    }

    /// Returns a list of gem directories from all active runtimes
    ///
    /// When in bundler context (bundler_runtime present):
    /// 1. Bundler vendor directory (.rb/vendor/bundler/ruby/X.Y.Z) - bundled gems only
    /// 2. Ruby lib directory (~/.rubies/ruby-X.Y.Z/lib/ruby/gems/X.Y.0) - system gems
    ///
    /// When NOT in bundler context:
    /// 1. User gem home (~/.gem/ruby/X.Y.Z) - user-installed gems
    /// 2. Ruby lib directory (~/.rubies/ruby-X.Y.Z/lib/ruby/gems/X.Y.0) - system gems
    ///
    /// NOTE: User gems are NOT available in bundler context for proper isolation.
    /// Use --no-bundler to opt out of bundler context and access user gems.
    pub fn gem_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // Bundler runtime gem dir first (if in bundler context)
        if let Some(ref bundler_runtime) = self.bundler_runtime
            && let Some(bundler_gem) = RuntimeProvider::gem_dir(bundler_runtime)
        {
            debug!("Adding bundler gem directory: {}", bundler_gem.display());
            dirs.push(bundler_gem);
        }

        // User gem home (only if NOT in bundler context for isolation)
        if self.bundler_runtime.is_none() {
            if let Some(ref gem_runtime) = self.gem_runtime {
                debug!(
                    "Adding gem home directory: {}",
                    gem_runtime.gem_home.display()
                );
                dirs.push(gem_runtime.gem_home.clone());
            }
        } else {
            debug!("Skipping user gem home (bundler isolation)");
        }

        // Ruby runtime lib dir always included (if Ruby available)
        if let Some(ref ruby_runtime) = self.ruby_runtime {
            let ruby_lib = ruby_runtime.lib_dir();
            debug!("Adding ruby lib directory for gems: {}", ruby_lib.display());
            dirs.push(ruby_lib);
        } else {
            debug!("No Ruby runtime available, skipping ruby lib directory");
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

        // Test gem_dirs - should have gem_home first (user gems), then ruby lib (system gems)
        let gem_dirs = butler.gem_dirs();
        assert_eq!(gem_dirs.len(), 2);
        assert_eq!(gem_dirs[0], gem_runtime.gem_home); // User gem home first (higher priority)
        assert_eq!(gem_dirs[1], ruby.lib_dir()); // Ruby lib dir second (system gems)

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
