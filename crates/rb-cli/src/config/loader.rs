use super::{ConfigError, RbConfig};
use super::locator::locate_config_file;
use std::fs;
use std::path::PathBuf;
use log::{debug, info};

/// Load configuration from file
/// Returns default config if no file is found
/// 
/// # Arguments
/// * `override_path` - Optional path to explicitly load config from (for testing)
pub fn load_config(override_path: Option<PathBuf>) -> Result<RbConfig, ConfigError> {
    if let Some(config_path) = locate_config_file(override_path.clone()) {
        info!("Loading configuration from: {}", config_path.display());
        
        let contents = fs::read_to_string(&config_path)?;
        let config: RbConfig = toml::from_str(&contents)?;
        
        // Log what was loaded
        debug!("Configuration file contents parsed successfully");
        if let Some(ref dir) = config.rubies_dir {
            debug!("  rubies-dir: {}", dir.display());
        }
        if let Some(ref version) = config.ruby_version {
            debug!("  ruby-version: {}", version);
        }
        if let Some(ref home) = config.gem_home {
            debug!("  gem-home: {}", home.display());
        }
        
        Ok(config)
    } else {
        if override_path.is_some() {
            debug!("Specified configuration file not found, using defaults");
        } else {
            debug!("No configuration file found in default locations, using defaults");
        }
        Ok(RbConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_returns_default_when_no_file() {
        // Should return default config when no file exists
        let result = load_config(None);
        assert!(result.is_ok());
        
        let config = result.unwrap();
        assert!(config.rubies_dir.is_none());
        assert!(config.ruby_version.is_none());
        assert!(config.gem_home.is_none());
    }

    #[test]
    fn test_load_config_with_custom_path() {
        use std::fs;
        use std::io::Write;
        
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_rb_custom.toml");
        
        // Create a test config file
        let mut file = fs::File::create(&config_path).expect("Failed to create test config");
        writeln!(file, r#"ruby-version = "3.2.0""#).expect("Failed to write config");
        drop(file);
        
        // Load config from custom path
        let result = load_config(Some(config_path.clone()));
        assert!(result.is_ok());
        
        let config = result.unwrap();
        assert_eq!(config.ruby_version, Some("3.2.0".to_string()));
        
        // Cleanup
        let _ = fs::remove_file(&config_path);
    }
}
