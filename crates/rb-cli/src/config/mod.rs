pub mod locator;
pub mod loader;

use clap::Args;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        help = "Designate the directory containing your Ruby installations (default: ~/.rubies)"
    )]
    #[serde(rename = "rubies-dir", skip_serializing_if = "Option::is_none")]
    pub rubies_dir: Option<PathBuf>,

    /// Request a particular Ruby version for your environment
    #[arg(
        short = 'r',
        long = "ruby",
        global = true,
        help = "Request a particular Ruby version for your environment (defaults to latest available)"
    )]
    #[serde(rename = "ruby-version", skip_serializing_if = "Option::is_none")]
    pub ruby_version: Option<String>,

    /// Specify custom gem base directory
    #[arg(
        short = 'G',
        long = "gem-home",
        global = true,
        help = "Specify custom gem base directory for gem installations (default: ~/.gem)"
    )]
    #[serde(rename = "gem-home", skip_serializing_if = "Option::is_none")]
    pub gem_home: Option<PathBuf>,
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
            debug!("  Using rubies-dir from CLI arguments: {}", self.rubies_dir.as_ref().unwrap().display());
        }
        
        if self.ruby_version.is_none() {
            if let Some(ref version) = other.ruby_version {
                debug!("  Using ruby-version from config file: {}", version);
                self.ruby_version = other.ruby_version;
            }
        } else {
            debug!("  Using ruby-version from CLI arguments: {}", self.ruby_version.as_ref().unwrap());
        }
        
        if self.gem_home.is_none() {
            if let Some(ref home) = other.gem_home {
                debug!("  Using gem-home from config file: {}", home.display());
                self.gem_home = other.gem_home;
            }
        } else {
            debug!("  Using gem-home from CLI arguments: {}", self.gem_home.as_ref().unwrap().display());
        }
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
        };

        cli_config.merge_with(file_config);

        assert_eq!(cli_config.rubies_dir, Some(PathBuf::from("/test/rubies")));
        assert_eq!(cli_config.ruby_version, Some("3.3.0".to_string()));
        assert_eq!(cli_config.gem_home, Some(PathBuf::from("/test/gems")));
    }

    #[test]
    fn test_merge_with_cli_takes_precedence() {
        let mut cli_config = RbConfig {
            rubies_dir: Some(PathBuf::from("/cli/rubies")),
            ruby_version: Some("3.2.0".to_string()),
            gem_home: None,
        };
        let file_config = RbConfig {
            rubies_dir: Some(PathBuf::from("/file/rubies")),
            ruby_version: Some("3.3.0".to_string()),
            gem_home: Some(PathBuf::from("/file/gems")),
        };

        cli_config.merge_with(file_config);

        // CLI values should be preserved
        assert_eq!(cli_config.rubies_dir, Some(PathBuf::from("/cli/rubies")));
        assert_eq!(cli_config.ruby_version, Some("3.2.0".to_string()));
        // File value should fill in missing CLI value
        assert_eq!(cli_config.gem_home, Some(PathBuf::from("/file/gems")));
    }

    #[test]
    fn test_merge_with_partial_file_config() {
        let mut cli_config = RbConfig {
            rubies_dir: None,
            ruby_version: Some("3.2.0".to_string()),
            gem_home: None,
        };
        let file_config = RbConfig {
            rubies_dir: Some(PathBuf::from("/file/rubies")),
            ruby_version: None,
            gem_home: Some(PathBuf::from("/file/gems")),
        };

        cli_config.merge_with(file_config);

        assert_eq!(cli_config.rubies_dir, Some(PathBuf::from("/file/rubies")));
        assert_eq!(cli_config.ruby_version, Some("3.2.0".to_string()));
        assert_eq!(cli_config.gem_home, Some(PathBuf::from("/file/gems")));
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

