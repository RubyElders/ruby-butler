use super::locator::locate_config_file;
use super::{ConfigError, RbConfig};
use log::{debug, info};
use std::fs;
use std::path::PathBuf;

/// Load configuration from file
/// Returns default config if no file is found
///
/// Supports both TOML and KDL formats (detected by file extension)
///
/// # Arguments
/// * `override_path` - Optional path to explicitly load config from (for testing)
pub fn load_config(override_path: Option<PathBuf>) -> Result<RbConfig, ConfigError> {
    if let Some(config_path) = locate_config_file(override_path.clone()) {
        info!("Loading configuration from: {}", config_path.display());

        let contents = fs::read_to_string(&config_path)?;

        // Determine format based on file extension
        let config: RbConfig = if config_path.extension().and_then(|s| s.to_str()) == Some("kdl") {
            debug!("Parsing configuration as KDL format");
            parse_kdl_config(&contents)?
        } else {
            debug!("Parsing configuration as TOML format");
            toml::from_str(&contents)?
        };

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

/// Parse KDL configuration into RbConfig
fn parse_kdl_config(content: &str) -> Result<RbConfig, ConfigError> {
    let doc: kdl::KdlDocument = content.parse().map_err(|e: kdl::KdlError| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse KDL: {}", e),
        )
    })?;

    let mut config = RbConfig::default();

    // Parse rubies-dir
    if let Some(node) = doc.get("rubies-dir")
        && let Some(entry) = node.entries().first()
        && let Some(value) = entry.value().as_string()
    {
        config.rubies_dir = Some(PathBuf::from(value));
    }

    // Parse ruby-version
    if let Some(node) = doc.get("ruby-version")
        && let Some(entry) = node.entries().first()
        && let Some(value) = entry.value().as_string()
    {
        config.ruby_version = Some(value.to_string());
    }

    // Parse gem-home
    if let Some(node) = doc.get("gem-home")
        && let Some(entry) = node.entries().first()
        && let Some(value) = entry.value().as_string()
    {
        config.gem_home = Some(PathBuf::from(value));
    }

    Ok(config)
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

    #[test]
    fn test_load_kdl_config() {
        use std::fs;

        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_rb_config.kdl");

        // Create a test KDL config file
        let kdl_content = r#"
rubies-dir "/opt/rubies"
ruby-version "3.3.0"
gem-home "/opt/gems"
"#;
        fs::write(&config_path, kdl_content).expect("Failed to write KDL config");

        // Load config from KDL path
        let result = load_config(Some(config_path.clone()));
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.rubies_dir, Some(PathBuf::from("/opt/rubies")));
        assert_eq!(config.ruby_version, Some("3.3.0".to_string()));
        assert_eq!(config.gem_home, Some(PathBuf::from("/opt/gems")));

        // Cleanup
        let _ = fs::remove_file(&config_path);
    }
}
