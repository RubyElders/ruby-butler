use rb_core::ruby::{RubyRuntime, RubyRuntimeDetector};
use rb_core::bundler::{BundlerRuntime, BundlerRuntimeDetector};
use rb_core::butler::ButlerRuntime;
use std::path::PathBuf;
use std::env;
use log::{debug, info};
use semver::Version;
use colored::*;

/// Centralized discovery context containing all environment information
/// that commands might need. Performs detection once and provides
/// different views of the environment to different commands.
#[derive(Debug)]
pub struct DiscoveryContext {
    pub rubies_dir: PathBuf,
    pub requested_ruby_version: Option<String>,
    pub current_dir: PathBuf,
    pub ruby_installations: Vec<RubyRuntime>,
    pub bundler_environment: Option<BundlerRuntime>,
    pub required_ruby_version: Option<Version>,
    pub selected_ruby: Option<RubyRuntime>,
    pub butler_runtime: Option<ButlerRuntime>,
}

impl DiscoveryContext {
    /// Perform comprehensive environment discovery
    pub fn discover(rubies_dir: PathBuf, requested_ruby_version: Option<String>) -> Result<Self, String> {
        let current_dir = env::current_dir()
            .map_err(|e| format!("Unable to determine current directory: {}", e))?;

        debug!("Starting comprehensive environment discovery");
        debug!("Search directory: {}", rubies_dir.display());
        debug!("Current directory: {}", current_dir.display());
        debug!("Requested Ruby version: {:?}", requested_ruby_version);

        // Step 1: Discover Ruby installations
        debug!("Discovering Ruby installations");
        let ruby_installations = RubyRuntimeDetector::discover(&rubies_dir)
            .map_err(|e| format!("Failed to discover Ruby installations: {}", e))?;
        
        info!("Found {} Ruby installations", ruby_installations.len());

        // Step 2: Detect bundler environment
        debug!("Detecting bundler environment");
        let bundler_environment = match BundlerRuntimeDetector::discover(&current_dir) {
            Ok(Some(bundler)) => {
                debug!("Bundler environment detected at: {}", bundler.root.display());
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
        };

        // Step 3: Determine required Ruby version
        let required_ruby_version = if let Some(bundler) = &bundler_environment {
            match bundler.ruby_version() {
                Some(version) => {
                    debug!("Bundler environment specifies Ruby version: {}", version);
                    Some(version)
                }
                None => {
                    debug!("Bundler environment found but no Ruby version specified");
                    None
                }
            }
        } else {
            None
        };

        // Step 4: Select appropriate Ruby runtime
        let selected_ruby = Self::select_ruby_runtime(
            &ruby_installations, 
            &requested_ruby_version, 
            &required_ruby_version
        );

        // Step 5: Create butler runtime if we have a selected Ruby
        let butler_runtime = if let Some(ruby) = &selected_ruby {
            match ruby.infer_gem_runtime() {
                Ok(gem_runtime) => {
                    debug!("Inferred gem runtime for Ruby {}: {}", ruby.version, gem_runtime.gem_home.display());
                    Some(ButlerRuntime::new(ruby.clone(), Some(gem_runtime)))
                }
                Err(e) => {
                    debug!("Failed to infer gem runtime for Ruby {}: {}", ruby.version, e);
                    Some(ButlerRuntime::new(ruby.clone(), None))
                }
            }
        } else {
            None
        };

        Ok(DiscoveryContext {
            rubies_dir,
            requested_ruby_version,
            current_dir,
            ruby_installations,
            bundler_environment,
            required_ruby_version,
            selected_ruby,
            butler_runtime,
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
                    let found = rubies.iter()
                        .find(|r| r.version == req_version)
                        .cloned();
                    
                    if found.is_none() {
                        println!("{}", format!("Requested Ruby version {} not found in available installations", requested).yellow());
                    }
                    return found;
                }
                Err(e) => {
                    println!("{}", format!("Invalid Ruby version format '{}': {}", requested, e).red());
                    return None;
                }
            }
        } else if let Some(required_version) = required_version {
            // Use version from bundler environment
            let found = rubies.iter()
                .find(|r| r.version == *required_version)
                .cloned();
            
            if let Some(ruby) = found {
                return Some(ruby);
            } else {
                println!("{}", format!("Required Ruby version {} (from bundler environment) not found in available installations", required_version).yellow());
                println!("{}", "   Falling back to latest available Ruby installation".bright_black());
                // Fall through to latest selection
            }
        }

        // Use latest available Ruby
        rubies.iter().max_by_key(|r| &r.version).cloned()
    }

    /// Check if we have a usable Ruby environment
    pub fn has_ruby_environment(&self) -> bool {
        self.selected_ruby.is_some()
    }

    /// Get the butler runtime, creating basic one if needed for exec command
    pub fn get_or_create_butler_runtime(&self) -> Result<ButlerRuntime, String> {
        if let Some(butler) = &self.butler_runtime {
            Ok(butler.clone())
        } else if let Some(ruby) = &self.selected_ruby {
            // Create basic butler runtime without gem runtime for exec
            Ok(ButlerRuntime::new(ruby.clone(), None))
        } else {
            Err("No Ruby runtime available".to_string())
        }
    }

    /// Display error if no Ruby installations found
    pub fn display_no_ruby_error(&self) {
        println!("{}", "No Ruby installations discovered in the designated quarters.".yellow());
        println!("{}", "   Perhaps consider installing Ruby environments to properly establish your estate.".bright_black());
    }

    /// Display error if no suitable Ruby found
    pub fn display_no_suitable_ruby_error(&self) {
        println!("{}", "No suitable Ruby installation could be selected".red());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rb_tests::RubySandbox;
    use semver::Version;
    use std::env;

    #[test]
    fn test_discovery_context_with_no_rubies() {
        let temp_dir = env::temp_dir();
        let empty_dir = temp_dir.join("empty_ruby_search");
        std::fs::create_dir_all(&empty_dir).expect("Failed to create empty directory");

        let context = DiscoveryContext::discover(empty_dir, None)
            .expect("Discovery should succeed even with no rubies");

        assert_eq!(context.ruby_installations.len(), 0);
        assert!(!context.has_ruby_environment());
        assert!(context.selected_ruby.is_none());
        assert!(context.butler_runtime.is_none());
        assert!(context.get_or_create_butler_runtime().is_err());
    }

    #[test]
    fn test_discovery_context_with_single_ruby() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        let context = DiscoveryContext::discover(sandbox.root().to_path_buf(), None)
            .expect("Discovery should succeed");

        assert_eq!(context.ruby_installations.len(), 1);
        assert!(context.has_ruby_environment());
        assert!(context.selected_ruby.is_some());
        assert!(context.butler_runtime.is_some());

        let selected = context.selected_ruby.as_ref().unwrap();
        assert_eq!(selected.version, Version::parse("3.2.5").unwrap());

        let butler = context.get_or_create_butler_runtime().expect("Should have butler runtime");
        assert!(!butler.bin_dirs().is_empty());
    }

    #[test]
    fn test_discovery_context_with_multiple_rubies_selects_latest() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
        sandbox.add_ruby_dir("3.3.1").expect("Failed to create ruby-3.3.1");

        let context = DiscoveryContext::discover(sandbox.root().to_path_buf(), None)
            .expect("Discovery should succeed");

        assert_eq!(context.ruby_installations.len(), 3);
        assert!(context.has_ruby_environment());
        
        let selected = context.selected_ruby.as_ref().unwrap();
        assert_eq!(selected.version, Version::parse("3.3.1").unwrap());
    }

    #[test]
    fn test_discovery_context_with_requested_version() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
        sandbox.add_ruby_dir("3.3.1").expect("Failed to create ruby-3.3.1");

        let context = DiscoveryContext::discover(
            sandbox.root().to_path_buf(), 
            Some("3.2.5".to_string())
        ).expect("Discovery should succeed");

        assert_eq!(context.ruby_installations.len(), 3);
        assert!(context.has_ruby_environment());
        assert_eq!(context.requested_ruby_version, Some("3.2.5".to_string()));
        
        let selected = context.selected_ruby.as_ref().unwrap();
        assert_eq!(selected.version, Version::parse("3.2.5").unwrap());
    }

    #[test]
    fn test_discovery_context_with_invalid_requested_version() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        let context = DiscoveryContext::discover(
            sandbox.root().to_path_buf(), 
            Some("invalid_version".to_string())
        ).expect("Discovery should succeed");

        assert_eq!(context.ruby_installations.len(), 1);
        assert!(!context.has_ruby_environment()); // Should fail with invalid version
        assert!(context.selected_ruby.is_none());
    }

    #[test]
    fn test_discovery_context_with_missing_requested_version() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        let context = DiscoveryContext::discover(
            sandbox.root().to_path_buf(), 
            Some("3.1.0".to_string())
        ).expect("Discovery should succeed");

        assert_eq!(context.ruby_installations.len(), 1);
        assert!(!context.has_ruby_environment()); // Should fail with missing version
        assert!(context.selected_ruby.is_none());
    }

    #[test]
    fn test_select_ruby_runtime_with_empty_list() {
        let result = DiscoveryContext::select_ruby_runtime(&[], &None, &None);
        assert!(result.is_none());
    }

    #[test]
    fn test_select_ruby_runtime_picks_latest() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        let ruby1_dir = sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");
        let ruby2_dir = sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
        let ruby3_dir = sandbox.add_ruby_dir("3.3.1").expect("Failed to create ruby-3.3.1");

        let rubies = vec![
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.1.0").unwrap(), &ruby1_dir),
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.2.5").unwrap(), &ruby2_dir),
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.3.1").unwrap(), &ruby3_dir),
        ];

        let result = DiscoveryContext::select_ruby_runtime(&rubies, &None, &None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().version, Version::parse("3.3.1").unwrap());
    }

    #[test]
    fn test_select_ruby_runtime_with_requested_version() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        let ruby1_dir = sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");
        let ruby2_dir = sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        let rubies = vec![
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.1.0").unwrap(), &ruby1_dir),
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.2.5").unwrap(), &ruby2_dir),
        ];

        let result = DiscoveryContext::select_ruby_runtime(
            &rubies, 
            &Some("3.1.0".to_string()), 
            &None
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().version, Version::parse("3.1.0").unwrap());
    }

    #[test]
    fn test_select_ruby_runtime_with_required_version() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        let ruby1_dir = sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");
        let ruby2_dir = sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        let rubies = vec![
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.1.0").unwrap(), &ruby1_dir),
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.2.5").unwrap(), &ruby2_dir),
        ];

        let result = DiscoveryContext::select_ruby_runtime(
            &rubies, 
            &None, 
            &Some(Version::parse("3.2.5").unwrap())
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().version, Version::parse("3.2.5").unwrap());
    }

    #[test]
    fn test_get_or_create_butler_runtime_with_existing() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        let context = DiscoveryContext::discover(sandbox.root().to_path_buf(), None)
            .expect("Discovery should succeed");

        let butler = context.get_or_create_butler_runtime().expect("Should have butler runtime");
        assert!(!butler.bin_dirs().is_empty());
    }

    #[test]
    fn test_get_or_create_butler_runtime_without_ruby() {
        let temp_dir = env::temp_dir();
        let empty_dir = temp_dir.join("empty_ruby_search_2");
        std::fs::create_dir_all(&empty_dir).expect("Failed to create empty directory");

        let context = DiscoveryContext::discover(empty_dir, None)
            .expect("Discovery should succeed");

        let result = context.get_or_create_butler_runtime();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No Ruby runtime available");
    }

    #[test]
    fn test_display_methods() {
        let temp_dir = env::temp_dir();
        let empty_dir = temp_dir.join("empty_ruby_display_test");
        std::fs::create_dir_all(&empty_dir).expect("Failed to create empty directory");

        let context = DiscoveryContext::discover(empty_dir, None)
            .expect("Discovery should succeed");

        // These methods print to stdout, so we just test they don't panic
        context.display_no_ruby_error();
        context.display_no_suitable_ruby_error();
    }

    #[test]
    fn test_select_ruby_runtime_requested_version_precedence() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        let ruby1_dir = sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");
        let ruby2_dir = sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        let rubies = vec![
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.1.0").unwrap(), &ruby1_dir),
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.2.5").unwrap(), &ruby2_dir),
        ];

        // Requested version should take precedence over required version
        let result = DiscoveryContext::select_ruby_runtime(
            &rubies, 
            &Some("3.1.0".to_string()), 
            &Some(Version::parse("3.2.5").unwrap())
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().version, Version::parse("3.1.0").unwrap());
    }

    #[test]
    fn test_select_ruby_runtime_required_version_fallback() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        let ruby1_dir = sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");
        let ruby2_dir = sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        let rubies = vec![
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.1.0").unwrap(), &ruby1_dir),
            RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.2.5").unwrap(), &ruby2_dir),
        ];

        // When required version not found, should fallback to latest
        let result = DiscoveryContext::select_ruby_runtime(
            &rubies, 
            &None, 
            &Some(Version::parse("3.0.0").unwrap()) // Not available
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().version, Version::parse("3.2.5").unwrap()); // Latest
    }

    #[test]
    fn test_discovery_context_current_directory_detection() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        // Change to sandbox directory to ensure consistent environment
        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(sandbox.root()).expect("Failed to change to sandbox dir");

        let butler_runtime = ButlerRuntime::discover_and_compose(sandbox.root().to_path_buf(), None)
            .expect("Discovery should succeed");

        // Restore directory (ignore errors in case directory was deleted)
        let _ = std::env::set_current_dir(original_dir);

        // Should capture current directory
        assert!(butler_runtime.current_dir().is_absolute());
        assert!(butler_runtime.current_dir().exists());
    }

    #[test]
    fn test_discovery_context_butler_runtime_creation_without_gem_runtime() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        // Create a context manually to test butler runtime creation edge cases
        let ruby_dir = sandbox.root().join("ruby-3.2.5");
        let ruby = RubyRuntime::new(rb_core::ruby::RubyType::CRuby, Version::parse("3.2.5").unwrap(), &ruby_dir);

        let context = DiscoveryContext {
            rubies_dir: sandbox.root().to_path_buf(),
            requested_ruby_version: None,
            current_dir: env::current_dir().unwrap(),
            ruby_installations: vec![ruby.clone()],
            bundler_environment: None,
            required_ruby_version: None,
            selected_ruby: Some(ruby),
            butler_runtime: None, // No pre-created butler runtime
        };

        let butler = context.get_or_create_butler_runtime().expect("Should create butler runtime");
        assert!(!butler.bin_dirs().is_empty());
    }
}
