pub mod loader;
pub mod locator;
pub mod value;

use clap::Args;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
pub use value::{ConfigSource, ConfigValue};

/// Shared configuration for both CLI and TOML
/// This struct serves both purposes:
/// - CLI argument parsing via clap::Args
/// - TOML deserialization via serde::Deserialize
#[derive(Debug, Clone, Args, Deserialize, Serialize, Default)]
pub struct RbConfig {
    /// Designate the directory containing your Ruby installations
    #[arg(
        short = 'R',
        long = "rubies-dir",
        global = true,
        help = "Designate the directory containing your Ruby installations",
        env = "RB_RUBIES_DIR"
    )]
    #[serde(rename = "rubies-dir", skip_serializing_if = "Option::is_none")]
    pub rubies_dir: Option<PathBuf>,

    /// Request a particular Ruby version for your environment
    #[arg(
        short = 'r',
        long = "ruby",
        global = true,
        help = "Request a particular Ruby version for your environment",
        env = "RB_RUBY_VERSION"
    )]
    #[serde(rename = "ruby-version", skip_serializing_if = "Option::is_none")]
    pub ruby_version: Option<String>,

    /// Specify custom gem base directory
    #[arg(
        short = 'G',
        long = "gem-home",
        global = true,
        help = "Specify custom gem base directory for gem installations",
        env = "RB_GEM_HOME"
    )]
    #[serde(rename = "gem-home", skip_serializing_if = "Option::is_none")]
    pub gem_home: Option<PathBuf>,

    /// Politely decline Bundler's company and operate independently
    #[arg(
        short = 'B',
        long = "no-bundler",
        global = true,
        action = clap::ArgAction::SetTrue,
        help = "Politely decline to activate bundler environment",
        env = "RB_NO_BUNDLER"
    )]
    #[serde(rename = "no-bundler", skip_serializing_if = "Option::is_none")]
    pub no_bundler: Option<bool>,

    /// Specify working directory (run as if started in this directory)
    #[arg(
        short = 'C',
        long = "work-dir",
        global = true,
        help = "Run as if started in the specified directory",
        env = "RB_WORK_DIR"
    )]
    #[serde(rename = "work-dir", skip_serializing_if = "Option::is_none")]
    pub work_dir: Option<PathBuf>,
}

impl RbConfig {
    /// Merge two configs, preferring values from self
    /// Used to merge CLI args (self) with file config (other)
    pub fn merge_with(&mut self, other: RbConfig) {
        use log::debug;

        debug!("Merging configuration (CLI arguments take precedence over config file)");

        if self.rubies_dir.is_none() {
            if let Some(ref dir) = other.rubies_dir {
                debug!("  Using rubies-dir from config file: {}", dir.display());
                self.rubies_dir = other.rubies_dir;
            }
        } else {
            debug!(
                "  Using rubies-dir from CLI arguments: {}",
                self.rubies_dir.as_ref().unwrap().display()
            );
        }

        if self.ruby_version.is_none() {
            if let Some(ref version) = other.ruby_version {
                debug!("  Using ruby-version from config file: {}", version);
                self.ruby_version = other.ruby_version;
            }
        } else {
            debug!(
                "  Using ruby-version from CLI arguments: {}",
                self.ruby_version.as_ref().unwrap()
            );
        }

        if self.gem_home.is_none() {
            if let Some(ref home) = other.gem_home {
                debug!("  Using gem-home from config file: {}", home.display());
                self.gem_home = other.gem_home;
            }
        } else {
            debug!(
                "  Using gem-home from CLI arguments: {}",
                self.gem_home.as_ref().unwrap().display()
            );
        }

        if self.no_bundler.is_none() {
            if let Some(no_bundler) = other.no_bundler {
                debug!("  Using no-bundler from config file: {}", no_bundler);
                self.no_bundler = Some(no_bundler);
            }
        } else {
            debug!(
                "  Using no-bundler from CLI arguments: {}",
                self.no_bundler.unwrap()
            );
        }

        if self.work_dir.is_none() {
            if let Some(ref dir) = other.work_dir {
                debug!("  Using work-dir from config file: {}", dir.display());
                self.work_dir = other.work_dir;
            }
        } else {
            debug!(
                "  Using work-dir from CLI arguments: {}",
                self.work_dir.as_ref().unwrap().display()
            );
        }
    }
}

/// Configuration with tracked sources for each value
/// This stores where each config value came from (CLI, env, file, or default)
#[derive(Debug, Clone)]
pub struct TrackedConfig {
    pub rubies_dir: ConfigValue<PathBuf>,
    pub ruby_version: Option<ConfigValue<String>>,
    pub gem_home: ConfigValue<PathBuf>,
    pub no_bundler: ConfigValue<bool>,
    pub work_dir: ConfigValue<PathBuf>,
}

impl TrackedConfig {
    /// Create a TrackedConfig from RbConfig, environment, and defaults
    /// Priority: CLI > Env > Config > Default
    pub fn from_merged(cli_config: &RbConfig, file_config: &RbConfig) -> Self {
        use log::debug;

        debug!("Building tracked configuration with sources");

        // Helper to determine source and value for PathBuf options
        let resolve_path_config = |cli: &Option<PathBuf>,
                                   file: &Option<PathBuf>,
                                   env_val: Option<PathBuf>,
                                   default: PathBuf|
         -> ConfigValue<PathBuf> {
            if let Some(path) = cli {
                debug!("  Using value from CLI: {}", path.display());
                ConfigValue::from_cli(path.clone())
            } else if let Some(path) = file {
                debug!("  Using value from config file: {}", path.display());
                ConfigValue::from_file(path.clone())
            } else if let Some(path) = env_val {
                debug!("  Using value from environment: {}", path.display());
                ConfigValue::from_env(path)
            } else {
                debug!("  Using default value: {}", default.display());
                ConfigValue::default_value(default)
            }
        };

        // Helper for optional String values
        let resolve_string_config = |cli: &Option<String>,
                                     file: &Option<String>,
                                     env_val: Option<String>|
         -> Option<ConfigValue<String>> {
            if let Some(val) = cli {
                debug!("  Using value from CLI: {}", val);
                Some(ConfigValue::from_cli(val.clone()))
            } else if let Some(val) = file {
                debug!("  Using value from config file: {}", val);
                Some(ConfigValue::from_file(val.clone()))
            } else if let Some(val) = env_val {
                debug!("  Using value from environment: {}", val);
                Some(ConfigValue::from_env(val))
            } else {
                None
            }
        };

        // Helper for bool values
        let resolve_bool_config = |cli: &Option<bool>,
                                   file: &Option<bool>,
                                   env_val: Option<bool>,
                                   default: bool|
         -> ConfigValue<bool> {
            if let Some(val) = cli {
                debug!("  Using value from CLI: {}", val);
                ConfigValue::from_cli(*val)
            } else if let Some(val) = file {
                debug!("  Using value from config file: {}", val);
                ConfigValue::from_file(*val)
            } else if let Some(val) = env_val {
                debug!("  Using value from environment: {}", val);
                ConfigValue::from_env(val)
            } else {
                debug!("  Using default value: {}", default);
                ConfigValue::default_value(default)
            }
        };

        // Read environment variables
        let env_rubies_dir = std::env::var("RB_RUBIES_DIR").ok().map(PathBuf::from);
        let env_ruby_version = std::env::var("RB_RUBY_VERSION").ok();
        let env_gem_home = std::env::var("RB_GEM_HOME").ok().map(PathBuf::from);
        let env_no_bundler = std::env::var("RB_NO_BUNDLER")
            .ok()
            .and_then(|v| v.parse::<bool>().ok());
        let env_work_dir = std::env::var("RB_WORK_DIR").ok().map(PathBuf::from);

        // Default values
        let default_rubies_dir = home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".rubies");
        let default_gem_home = home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".gem");
        let default_work_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        debug!("Resolving rubies_dir:");
        let rubies_dir = resolve_path_config(
            &cli_config.rubies_dir,
            &file_config.rubies_dir,
            env_rubies_dir,
            default_rubies_dir,
        );

        debug!("Resolving ruby_version:");
        let ruby_version = resolve_string_config(
            &cli_config.ruby_version,
            &file_config.ruby_version,
            env_ruby_version,
        );

        debug!("Resolving gem_home:");
        let gem_home = resolve_path_config(
            &cli_config.gem_home,
            &file_config.gem_home,
            env_gem_home,
            default_gem_home,
        );

        debug!("Resolving no_bundler:");
        let no_bundler = resolve_bool_config(
            &cli_config.no_bundler,
            &file_config.no_bundler,
            env_no_bundler,
            false,
        );

        debug!("Resolving work_dir:");
        let work_dir = resolve_path_config(
            &cli_config.work_dir,
            &file_config.work_dir,
            env_work_dir,
            default_work_dir,
        );

        Self {
            rubies_dir,
            ruby_version,
            gem_home,
            no_bundler,
            work_dir,
        }
    }

    /// Convert back to RbConfig for compatibility with existing code
    pub fn to_rb_config(&self) -> RbConfig {
        RbConfig {
            rubies_dir: Some(self.rubies_dir.value.clone()),
            ruby_version: self.ruby_version.as_ref().map(|v| v.value.clone()),
            gem_home: Some(self.gem_home.value.clone()),
            no_bundler: Some(self.no_bundler.value),
            work_dir: Some(self.work_dir.value.clone()),
        }
    }

    /// Get ruby_version for ButlerRuntime (returns None if unresolved)
    pub fn ruby_version_for_runtime(&self) -> Option<String> {
        self.ruby_version
            .as_ref()
            .filter(|v| !v.is_unresolved())
            .map(|v| v.value.clone())
    }

    /// Update ruby_version with resolved value from ButlerRuntime
    pub fn resolve_ruby_version(&mut self, resolved_version: String) {
        if let Some(ref mut version) = self.ruby_version {
            if version.is_unresolved() {
                version.resolve(resolved_version);
            }
        } else {
            self.ruby_version = Some(ConfigValue::resolved(resolved_version));
        }
    }

    /// Check if any values are unresolved
    pub fn has_unresolved(&self) -> bool {
        self.ruby_version
            .as_ref()
            .is_some_and(|v| v.is_unresolved())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "Failed to read configuration file: {}", e),
            ConfigError::ParseError(e) => write!(f, "Failed to parse configuration file: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::ParseError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_with_empty_cli_config() {
        let mut cli_config = RbConfig::default();
        let file_config = RbConfig {
            rubies_dir: Some(PathBuf::from("/test/rubies")),
            ruby_version: Some("3.3.0".to_string()),
            gem_home: Some(PathBuf::from("/test/gems")),
            no_bundler: None,
            work_dir: None,
        };

        cli_config.merge_with(file_config);

        assert_eq!(cli_config.rubies_dir, Some(PathBuf::from("/test/rubies")));
        assert_eq!(cli_config.ruby_version, Some("3.3.0".to_string()));
        assert_eq!(cli_config.gem_home, Some(PathBuf::from("/test/gems")));
        assert_eq!(cli_config.no_bundler, None);
    }

    #[test]
    fn test_merge_with_cli_takes_precedence() {
        let mut cli_config = RbConfig {
            rubies_dir: Some(PathBuf::from("/cli/rubies")),
            ruby_version: Some("3.2.0".to_string()),
            gem_home: None,
            no_bundler: None,
            work_dir: None,
        };
        let file_config = RbConfig {
            rubies_dir: Some(PathBuf::from("/file/rubies")),
            ruby_version: Some("3.3.0".to_string()),
            gem_home: Some(PathBuf::from("/file/gems")),
            no_bundler: Some(true),
            work_dir: None,
        };

        cli_config.merge_with(file_config);

        // CLI values should be preserved
        assert_eq!(cli_config.rubies_dir, Some(PathBuf::from("/cli/rubies")));
        assert_eq!(cli_config.ruby_version, Some("3.2.0".to_string()));
        // File value should fill in missing CLI value
        assert_eq!(cli_config.gem_home, Some(PathBuf::from("/file/gems")));
        assert_eq!(cli_config.no_bundler, Some(true));
    }

    #[test]
    fn test_merge_with_partial_file_config() {
        let mut cli_config = RbConfig {
            rubies_dir: None,
            ruby_version: Some("3.2.0".to_string()),
            gem_home: None,
            no_bundler: None,
            work_dir: None,
        };
        let file_config = RbConfig {
            rubies_dir: Some(PathBuf::from("/file/rubies")),
            ruby_version: None,
            gem_home: Some(PathBuf::from("/file/gems")),
            no_bundler: None,
            work_dir: None,
        };

        cli_config.merge_with(file_config);

        assert_eq!(cli_config.rubies_dir, Some(PathBuf::from("/file/rubies")));
        assert_eq!(cli_config.ruby_version, Some("3.2.0".to_string()));
        assert_eq!(cli_config.gem_home, Some(PathBuf::from("/file/gems")));
        assert_eq!(cli_config.no_bundler, None);
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_str = r#"
            rubies-dir = "/opt/rubies"
            ruby-version = "3.3.0"
            gem-home = "/opt/gems"
        "#;

        let config: RbConfig = toml::from_str(toml_str).expect("Failed to parse TOML");

        assert_eq!(config.rubies_dir, Some(PathBuf::from("/opt/rubies")));
        assert_eq!(config.ruby_version, Some("3.3.0".to_string()));
        assert_eq!(config.gem_home, Some(PathBuf::from("/opt/gems")));
    }

    #[test]
    fn test_toml_serialization() {
        let config = RbConfig {
            rubies_dir: Some(PathBuf::from("/opt/rubies")),
            ruby_version: Some("3.3.0".to_string()),
            gem_home: Some(PathBuf::from("/opt/gems")),
            no_bundler: None,
            work_dir: None,
        };

        let toml_str = toml::to_string(&config).expect("Failed to serialize to TOML");

        assert!(toml_str.contains("rubies-dir"));
        assert!(toml_str.contains("/opt/rubies"));
        assert!(toml_str.contains("ruby-version"));
        assert!(toml_str.contains("3.3.0"));
        assert!(toml_str.contains("gem-home"));
        assert!(toml_str.contains("/opt/gems"));
    }
}
