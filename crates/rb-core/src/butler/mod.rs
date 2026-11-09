use crate::bundler::{BundlerRuntime, BundlerRuntimeDetector};
use crate::gems::GemRuntime;
use crate::process_context::ProcessContext;
use crate::ruby::{RubyDiscoveryError, RubyRuntime, RubyRuntimeDetector};
use home;
use log::{debug, info};
use semver::Version;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    sync::{Arc, OnceLock},
};

pub mod command;
mod execution_plan;
pub mod runtime_provider;

pub use command::Command;
pub use execution_plan::ExecutionPlan;
pub use runtime_provider::RuntimeProvider;

/// Helper to compose detectors based on environment context during early discovery phase.
///
/// Delegates to detector factories shared with runtime providers.
pub struct DetectorComposer;

impl DetectorComposer {
    /// Compose version detector for bundler environment by delegating to BundlerRuntime
    pub fn version_detector_for_bundler() -> crate::ruby::CompositeDetector {
        BundlerRuntime::version_detector()
    }

    /// Compose gem path detector for bundler environment by delegating to BundlerRuntime
    ///
    /// Use this when bundler is detected - excludes user gems to maintain bundle isolation
    pub fn gem_path_detector_for_bundler()
    -> crate::gems::gem_path_detector::CompositeGemPathDetector {
        BundlerRuntime::gem_path_detector()
    }

    /// Compose gem path detector for standard (non-bundler) environment by delegating to GemRuntime
    ///
    /// Use this when bundler is NOT detected - includes user gems
    pub fn gem_path_detector_standard() -> crate::gems::gem_path_detector::CompositeGemPathDetector
    {
        GemRuntime::gem_path_detector()
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
#[derive(Clone)]
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
    package_environment: BTreeMap<OsString, OsString>,
    package_bin: Option<PathBuf>,
    process: ProcessContext,
    environment: Arc<OnceLock<BTreeMap<OsString, OsString>>>,
}

impl std::fmt::Debug for ButlerRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ButlerRuntime")
            .field("ruby_runtime", &self.ruby_runtime)
            .field("current_dir", &self.current_dir)
            .finish_non_exhaustive()
    }
}

impl ButlerRuntime {
    pub fn with_package_environment(
        self,
        environment: HashMap<String, String>,
        bin: PathBuf,
    ) -> Self {
        self.with_package_environment_os(
            environment
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            bin,
        )
    }

    pub fn with_package_environment_os(
        mut self,
        environment: BTreeMap<OsString, OsString>,
        bin: PathBuf,
    ) -> Self {
        self.package_environment = environment;
        self.package_bin = Some(bin);
        self.environment = Arc::new(OnceLock::new());
        self
    }

    pub fn with_process_context(mut self, process: ProcessContext) -> Self {
        self.current_dir = process.directory().to_owned();
        self.process = process;
        self.environment = Arc::new(OnceLock::new());
        self
    }

    pub fn process_context(&self) -> &ProcessContext {
        &self.process
    }

    pub fn execution_environment(&self) -> &BTreeMap<OsString, OsString> {
        self.environment.get_or_init(|| {
            let overrides = self.compose_environment_overrides(self.process.var("PATH"));
            let mut environment = self.process.environment().clone();
            environment.extend(overrides);
            environment
        })
    }

    fn compose_environment_overrides(
        &self,
        inherited: Option<&OsStr>,
    ) -> BTreeMap<OsString, OsString> {
        debug!("Composing environment variables");
        let path = self.search_path(inherited);
        let mut overrides = self.environment_overrides();
        overrides.insert("PATH".into(), path);
        for key in [
            "GEM_HOME",
            "GEM_PATH",
            "BUNDLE_GEMFILE",
            "BUNDLE_APP_CONFIG",
            "BUNDLE_PATH",
        ] {
            if let Some(value) = overrides.get(OsStr::new(key)) {
                debug!("Setting {key}: {}", value.to_string_lossy());
            }
        }
        if !overrides.contains_key(OsStr::new("GEM_HOME")) {
            debug!("No GEM_HOME available - skipping GEM_HOME and GEM_PATH");
        }
        if !overrides.contains_key(OsStr::new("BUNDLE_PATH")) {
            debug!("No bundler runtime detected - skipping bundler environment variables");
        }
        debug!(
            "Environment composition complete: {} variables",
            overrides.len()
        );
        overrides
    }

    fn search_path(&self, inherited: Option<&OsStr>) -> OsString {
        debug!("Building PATH environment variable");
        let mut paths: Vec<OsString> = self
            .bin_dirs()
            .into_iter()
            .map(|path| {
                debug!("Adding to PATH: {}", path.display());
                path.into_os_string()
            })
            .collect();
        if let Some(inherited) = inherited {
            debug!("Appending existing PATH: {}", inherited.to_string_lossy());
            paths.push(inherited.to_owned());
        } else {
            debug!("No existing PATH provided");
        }
        let result = Self::join_paths(paths);
        debug!("Final PATH: {}", result.to_string_lossy());
        result
    }

    fn join_paths(paths: impl IntoIterator<Item = OsString>) -> OsString {
        let mut result = OsString::new();
        for (index, path) in paths.into_iter().enumerate() {
            if index > 0 {
                result.push(if cfg!(windows) { ";" } else { ":" });
            }
            result.push(path);
        }
        result
    }

    pub fn uses_bundler(&self) -> bool {
        self.bundler_runtime.is_some() || self.package_bin.is_some()
    }

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
            current_dir: current_dir.clone(),
            ruby_installations: vec![],
            requested_ruby_version: None,
            gem_base_dir: None,
            package_environment: BTreeMap::new(),
            package_bin: None,
            process: ProcessContext::new(current_dir.clone(), std::env::vars_os().collect()),
            environment: Arc::new(OnceLock::new()),
        }
    }

    /// Create an empty ButlerRuntime when no Ruby installations are found
    /// This allows the runtime to exist even without Ruby, with methods failing when features are accessed
    pub fn empty(rubies_dir: PathBuf, current_dir: PathBuf) -> Self {
        Self::empty_in(
            rubies_dir,
            ProcessContext::new(current_dir, std::env::vars_os().collect()),
        )
    }

    pub fn empty_in(rubies_dir: PathBuf, process: ProcessContext) -> Self {
        let current_dir = process.directory().to_owned();
        debug!("Creating empty ButlerRuntime (no Ruby installations found)");

        Self {
            ruby_runtime: None,
            gem_runtime: None,
            bundler_runtime: None,
            rubies_dir,
            current_dir: current_dir.clone(),
            ruby_installations: vec![],
            requested_ruby_version: None,
            gem_base_dir: None,
            package_environment: BTreeMap::new(),
            package_bin: None,
            process,
            environment: Arc::new(OnceLock::new()),
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
        Self::discover_in(
            ProcessContext::new(current_dir, std::env::vars_os().collect()),
            rubies_dir,
            requested_ruby_version,
            gem_base_dir,
            skip_bundler,
        )
    }

    pub fn discover_in(
        process: ProcessContext,
        rubies_dir: PathBuf,
        requested_ruby_version: Option<String>,
        gem_base_dir: Option<PathBuf>,
        skip_bundler: bool,
    ) -> Result<Self, ButlerError> {
        let current_dir = process.directory().to_owned();
        let requested_rubies_dir = rubies_dir;
        let rubies_dir = current_dir.join(&requested_rubies_dir);
        let gem_base_dir = gem_base_dir.map(|directory| current_dir.join(directory));
        debug!("Starting comprehensive environment discovery");
        debug!("Rubies directory: {}", rubies_dir.display());
        debug!("Current directory: {}", current_dir.display());
        debug!("Requested Ruby version: {:?}", requested_ruby_version);

        debug!("Discovering Ruby installations");
        let ruby_installations = match RubyRuntimeDetector::discover(&rubies_dir) {
            Ok(installations) => installations,
            Err(RubyDiscoveryError::DirectoryNotFound(_)) => {
                return Err(ButlerError::RubiesDirectoryNotFound(requested_rubies_dir));
            }
            Err(e) => {
                debug!("Ruby discovery failed: {:?}", e);
                vec![]
            }
        };

        info!("Found {} Ruby installations", ruby_installations.len());

        if ruby_installations.is_empty() {
            debug!("No Ruby installations found, returning empty runtime");
            return Ok(Self::empty_in(rubies_dir, process));
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

        // Extract version requirements from project directory
        let required_ruby_version = if bundler_root.is_some() {
            let detector = DetectorComposer::version_detector_for_bundler();
            detector.detect(&current_dir)
        } else {
            None
        };

        // Select the most appropriate Ruby installation
        let selected_ruby = Self::select_ruby_runtime(
            &ruby_installations,
            &requested_ruby_version,
            &required_ruby_version,
        );

        // If no Ruby selected, handle appropriately
        let Some(selected_ruby) = selected_ruby else {
            if let Some(requested) = &requested_ruby_version {
                return Err(ButlerError::NoSuitableRuby(format!(
                    "Requested Ruby version {} not found",
                    requested
                )));
            }
            // Otherwise return empty runtime
            debug!("No suitable Ruby selected, returning empty runtime");
            return Ok(Self::empty_in(rubies_dir, process));
        };

        let bundler_runtime =
            bundler_root.map(|root| BundlerRuntime::new(root, selected_ruby.version.clone()));

        Ok(Self::compose_selected(
            process,
            rubies_dir,
            requested_ruby_version,
            gem_base_dir,
            ruby_installations,
            selected_ruby,
            bundler_runtime,
        ))
    }

    pub fn compose_selected(
        process: ProcessContext,
        rubies_dir: PathBuf,
        requested_ruby_version: Option<String>,
        gem_base_dir: Option<PathBuf>,
        ruby_installations: Vec<RubyRuntime>,
        selected_ruby: RubyRuntime,
        bundler_runtime: Option<BundlerRuntime>,
    ) -> Self {
        let current_dir = process.directory().to_owned();
        let rubies_dir = current_dir.join(rubies_dir);
        let gem_base_dir = gem_base_dir.map(|directory| current_dir.join(directory));
        // Detect and compose gem path configuration
        use crate::gems::gem_path_detector::GemPathContext;

        let gem_detector = if bundler_runtime.is_some() {
            DetectorComposer::gem_path_detector_for_bundler()
        } else {
            DetectorComposer::gem_path_detector_standard()
        };

        let gem_context = GemPathContext::with_home(
            &current_dir,
            &selected_ruby,
            gem_base_dir.as_deref(),
            process.home_directory(),
        );

        let gem_path_config = gem_detector.detect(&gem_context);
        debug!(
            "Detected gem path with {} directories",
            gem_path_config.gem_dirs().len()
        );

        let gem_runtime = gem_path_config.gem_home().map(|gem_home| GemRuntime {
            gem_home: gem_home.to_owned(),
            gem_bin: gem_path_config
                .gem_bin_dirs()
                .first()
                .cloned()
                .unwrap_or_else(|| gem_home.join("bin")),
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

        Self {
            ruby_runtime: Some(selected_ruby),
            gem_runtime,
            bundler_runtime,
            rubies_dir,
            current_dir: current_dir.clone(),
            ruby_installations,
            requested_ruby_version,
            gem_base_dir,
            package_environment: BTreeMap::new(),
            package_bin: None,
            process,
            environment: Arc::new(OnceLock::new()),
        }
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
        let mut dirs = self.package_bin.iter().cloned().collect::<Vec<_>>();

        // Bundler runtime bin dir first (if in bundler context)
        if self.package_bin.is_none()
            && let Some(ref bundler_runtime) = self.bundler_runtime
            && let Some(bundler_bin) = RuntimeProvider::bin_dir(bundler_runtime)
        {
            debug!(
                "Adding bundler bin directory to PATH: {}",
                bundler_bin.display()
            );
            dirs.push(bundler_bin);
        }

        // Gem runtime bin dir (only if NOT in bundler context for isolation)
        if !self.uses_bundler() {
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
        let mut dirs = self
            .package_environment
            .get(OsStr::new("GEM_PATH"))
            .map(|paths| std::env::split_paths(paths).collect::<Vec<_>>())
            .unwrap_or_default();

        // Bundler runtime gem dir first (if in bundler context)
        if self.package_bin.is_none()
            && let Some(ref bundler_runtime) = self.bundler_runtime
            && let Some(bundler_gem) = RuntimeProvider::gem_dir(bundler_runtime)
        {
            debug!("Adding bundler gem directory: {}", bundler_gem.display());
            dirs.push(bundler_gem);
        }

        // User gem home (only if NOT in bundler context for isolation)
        if !self.uses_bundler() {
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

    /// Returns the active package gem home, falling back to the user gem home.
    pub fn gem_home(&self) -> Option<PathBuf> {
        let result = self
            .package_environment
            .get(OsStr::new("GEM_HOME"))
            .map(PathBuf::from)
            .or_else(|| {
                self.gem_runtime
                    .as_ref()
                    .map(|gem_runtime| gem_runtime.gem_home.clone())
            });

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
        self.compose_environment_overrides(existing_path.as_deref().map(OsStr::new))
            .into_iter()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
            .collect()
    }

    fn environment_overrides(&self) -> BTreeMap<OsString, OsString> {
        let mut environment = BTreeMap::new();
        if let Some(gem_home) = self.gem_home() {
            environment.insert("GEM_HOME".into(), gem_home.clone().into_os_string());
            if !self
                .package_environment
                .contains_key(OsStr::new("GEM_PATH"))
            {
                let mut paths = vec![gem_home];
                for directory in self.gem_dirs() {
                    if !paths.contains(&directory) {
                        paths.push(directory);
                    }
                }
                environment.insert(
                    "GEM_PATH".into(),
                    Self::join_paths(paths.into_iter().map(PathBuf::into_os_string)),
                );
            }
        }
        if let Some(bundler) = &self.bundler_runtime {
            environment.insert(
                "BUNDLE_GEMFILE".into(),
                bundler.gemfile_path().into_os_string(),
            );
            environment.insert(
                "BUNDLE_APP_CONFIG".into(),
                bundler.app_config_dir().into_os_string(),
            );
            environment.insert("BUNDLE_PATH".into(), bundler.vendor_dir().into_os_string());
        }
        environment.extend(self.package_environment.clone());
        environment
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

    #[test]
    fn prepared_package_environment_overrides_runtime_paths_without_global_mutation() {
        let ruby = create_ruby_runtime("3.4.10", "/opt/ruby");
        let original = ButlerRuntime::new(ruby, None);
        let bin = PathBuf::from("/prepared/bin");
        let prepared = original.clone().with_package_environment(
            HashMap::from([
                ("GEM_HOME".into(), "/prepared".into()),
                ("GEM_PATH".into(), "/prepared".into()),
                ("BUNDLE_GEMFILE".into(), "/project/.rb/Gemfile".into()),
            ]),
            bin.clone(),
        );
        let environment = prepared.env_vars(Some("/existing/bin".into()));
        assert_eq!(environment["GEM_HOME"], "/prepared");
        assert_eq!(environment["GEM_PATH"], "/prepared");
        assert_eq!(environment["BUNDLE_GEMFILE"], "/project/.rb/Gemfile");
        assert_eq!(prepared.bin_dirs()[0], bin);
        assert_eq!(prepared.gem_home(), Some(PathBuf::from("/prepared")));
        assert_eq!(prepared.gem_dirs()[0], PathBuf::from("/prepared"));
        assert_eq!(prepared.gem_dirs().len(), 2);
        assert!(environment["PATH"].starts_with("/prepared/bin"));
        assert!(prepared.uses_bundler());
        assert!(!original.uses_bundler());
        assert!(!original.env_vars(None).contains_key("BUNDLE_GEMFILE"));
    }
}
