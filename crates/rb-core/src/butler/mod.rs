use std::path::PathBuf;
use std::collections::HashMap;
use log::debug;
use semver;
use crate::ruby::{RubyRuntime, RubyRuntimeDetector};
use crate::gems::GemRuntime;
use home;

pub mod runtime_provider;

#[derive(Debug, Clone)]
pub struct ButlerRuntime {
    ruby_runtime: RubyRuntime,
    gem_runtime: Option<GemRuntime>,
}

impl ButlerRuntime {
    /// Create a new ButlerRuntime with a mandatory RubyRuntime and optional GemRuntime
    pub fn new(ruby_runtime: RubyRuntime, gem_runtime: Option<GemRuntime>) -> Self {
        debug!("Creating ButlerRuntime with Ruby: {} {}", ruby_runtime.kind.as_str(), ruby_runtime.version);
        
        if let Some(ref gem_runtime) = gem_runtime {
            debug!("Including GemRuntime with gem_home: {}", gem_runtime.gem_home.display());
        } else {
            debug!("No GemRuntime provided");
        }
        
        Self {
            ruby_runtime,
            gem_runtime,
        }
    }

    /// Returns a list of bin directories from both ruby and gem runtimes
    /// Gem bin directory comes first (higher priority) if present, then Ruby bin directory
    pub fn bin_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        
        // Gem runtime bin dir first (highest priority) - for user-installed tools
        if let Some(ref gem_runtime) = self.gem_runtime {
            debug!("Adding gem bin directory to PATH: {}", gem_runtime.gem_bin.display());
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
            debug!("Adding gem home directory: {}", gem_runtime.gem_home.display());
            dirs.push(gem_runtime.gem_home.clone());
        }
        
        debug!("Total gem directories: {}", dirs.len());
        dirs
    }

    /// Returns the gem_home from GemRuntime if present, otherwise returns None
    pub fn gem_home(&self) -> Option<PathBuf> {
        let result = self.gem_runtime.as_ref()
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
    /// Returns a HashMap with PATH, GEM_HOME, and GEM_PATH set appropriately
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
        
        debug!("Environment composition complete: {} variables", env.len());
        env
    }

    /// Convenience function to create a ButlerRuntime by discovering and selecting Ruby
    /// from a directory. Uses latest Ruby if no version is specified.
    pub fn discover_and_create(
        search_dir: &PathBuf, 
        requested_version: Option<&str>
    ) -> Result<Self, String> {
        debug!("Starting Ruby discovery process in: {}", search_dir.display());
        
        let rubies = RubyRuntimeDetector::discover(search_dir)
            .map_err(|e| format!("Failed to discover Ruby installations: {}", e))?;

        if rubies.is_empty() {
            return Err(format!("No Ruby installations found in {}", search_dir.display()));
        }

        debug!("Ruby discovery completed successfully, found {} installations", rubies.len());

        // Select Ruby based on requested version or use latest
        let selected_ruby = if let Some(version_str) = requested_version {
            debug!("Looking for requested Ruby version: {}", version_str);
            
            // Try to parse the version and find exact match
            let found = if let Ok(requested_version) = semver::Version::parse(version_str) {
                rubies.iter().find(|ruby| ruby.version == requested_version)
            } else {
                // If parsing fails, try string matching
                rubies.iter().find(|ruby| ruby.version.to_string() == version_str)
            };
            
            match found {
                Some(ruby) => {
                    debug!("Selected Ruby installation: {} {}", ruby.kind.as_str(), ruby.version);
                    ruby.clone()
                }
                None => {
                    return Err(format!(
                        "Ruby version {} not found. Available versions: {}", 
                        version_str,
                        rubies.iter()
                            .map(|r| r.version.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
        } else {
            // Use latest Ruby
            match RubyRuntimeDetector::latest(&rubies) {
                Some(latest) => {
                    debug!("Using latest Ruby installation: {} {}", latest.kind.as_str(), latest.version);
                    latest.clone()
                }
                None => {
                    return Err("No Ruby installations found".to_string());
                }
            }
        };

        // Try to infer gem runtime and compose ButlerRuntime
        let butler = match selected_ruby.infer_gem_runtime() {
            Ok(gem_runtime) => {
                debug!("Inferred gem runtime for Ruby {}: {}", selected_ruby.version, gem_runtime.gem_home.display());
                ButlerRuntime::new(selected_ruby, Some(gem_runtime))
            }
            Err(e) => {
                debug!("Failed to infer gem runtime for Ruby {}: {}", selected_ruby.version, e);
                ButlerRuntime::new(selected_ruby, None)
            }
        };

        Ok(butler)
    }

    /// Get the default rubies directory (~/.rubies)
    pub fn default_rubies_dir() -> Result<PathBuf, String> {
        let home_dir = home::home_dir()
            .ok_or_else(|| "Could not determine home directory".to_string())?;
        Ok(home_dir.join(".rubies"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruby::{RubyType, RubyRuntime};
    use crate::gems::GemRuntime;
    use semver::Version;
    use std::path::Path;

    fn create_ruby_runtime(version: &str, root: &str) -> RubyRuntime {
        RubyRuntime::new(
            RubyType::CRuby,
            Version::parse(version).unwrap(),
            root
        )
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
        assert_eq!(bin_dirs[0], gem_runtime.gem_bin);  // Gem bin dir first (higher priority)
        assert_eq!(bin_dirs[1], ruby.bin_dir());       // Ruby bin dir second

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
        let expected = format!("{}{}{}{}/usr/bin", 
            gem_runtime.gem_bin.display(),  // Gem bin first (highest priority)
            separator, 
            ruby.bin_dir().display(),       // Ruby bin second
            separator
        );
        assert_eq!(path, expected);
    }
}