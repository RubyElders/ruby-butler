use log::debug;
use std::path::PathBuf;

/// Trait for reading environment variables - allows mocking in tests
pub trait EnvReader {
    fn var(&self, key: &str) -> Result<String, std::env::VarError>;
}

/// Production implementation using std::env
pub struct StdEnvReader;

impl EnvReader for StdEnvReader {
    fn var(&self, key: &str) -> Result<String, std::env::VarError> {
        std::env::var(key)
    }
}

/// Locate the configuration file following XDG Base Directory specification
///
/// Supports both rb.kdl and rb.toml (preferring .kdl)
///
/// Priority order:
/// 1. Explicit override path (if provided)
/// 2. $RB_CONFIG environment variable
/// 3. $XDG_CONFIG_HOME/rb/rb.kdl or rb.toml (Unix/Linux)
/// 4. ~/.config/rb/rb.kdl or rb.toml (Unix/Linux fallback)
/// 5. %APPDATA%/rb/rb.kdl or rb.toml (Windows)
/// 6. ~/.rb.kdl or ~/.rb.toml (cross-platform fallback)
pub fn locate_config_file(override_path: Option<PathBuf>) -> Option<PathBuf> {
    locate_config_file_with_env(override_path, &StdEnvReader)
}

/// Internal function that accepts an environment reader for testing
fn locate_config_file_with_env(
    override_path: Option<PathBuf>,
    env: &dyn EnvReader,
) -> Option<PathBuf> {
    debug!("Searching for configuration file...");

    // 1. Check for explicit override first
    if let Some(path) = override_path {
        debug!("  Checking --config override: {}", path.display());
        if path.exists() {
            debug!("  Found configuration file via --config flag");
            return Some(path);
        }
    }

    // 2. Check RB_CONFIG environment variable
    if let Ok(rb_config) = env.var("RB_CONFIG") {
        let config_path = PathBuf::from(rb_config);
        debug!("  Checking RB_CONFIG env var: {}", config_path.display());
        if config_path.exists() {
            debug!("  Found configuration file via RB_CONFIG");
            return Some(config_path);
        }
    }

    // 3. Try XDG_CONFIG_HOME (Unix/Linux)
    if let Ok(xdg_config) = env.var("XDG_CONFIG_HOME") {
        let base_path = PathBuf::from(xdg_config).join("rb");
        // Try .kdl first, then .toml
        for ext in &["rb.kdl", "rb.toml"] {
            let config_path = base_path.join(ext);
            debug!("  Checking XDG_CONFIG_HOME: {}", config_path.display());
            if config_path.exists() {
                debug!("  Found configuration file in XDG_CONFIG_HOME");
                return Some(config_path);
            }
        }
    }

    // Try home directory based paths
    if let Some(home_dir) = home::home_dir() {
        // Unix/Linux: ~/.config/rb/rb.kdl or rb.toml
        #[cfg(not(target_os = "windows"))]
        {
            let base_path = home_dir.join(".config").join("rb");
            for ext in &["rb.kdl", "rb.toml"] {
                let config_path = base_path.join(ext);
                debug!("  Checking ~/.config/rb/{}: {}", ext, config_path.display());
                if config_path.exists() {
                    debug!("  Found configuration file in ~/.config/rb/");
                    return Some(config_path);
                }
            }
        }

        // Windows: %APPDATA%/rb/rb.kdl or rb.toml
        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let base_path = PathBuf::from(appdata).join("rb");
                for ext in &["rb.kdl", "rb.toml"] {
                    let config_path = base_path.join(ext);
                    debug!("  Checking %APPDATA%/rb/{}: {}", ext, config_path.display());
                    if config_path.exists() {
                        debug!("  Found configuration file in %APPDATA%/rb/");
                        return Some(config_path);
                    }
                }
            }
        }

        // Cross-platform fallback: ~/.rb.kdl or ~/.rb.toml
        for ext in &[".rb.kdl", ".rb.toml"] {
            let fallback_path = home_dir.join(ext);
            debug!("  Checking fallback ~/{}: {}", ext, fallback_path.display());
            if fallback_path.exists() {
                debug!("  Found configuration file at ~/{}", ext);
                return Some(fallback_path);
            }
        }
    }

    debug!("  No configuration file found in any location");
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Mock environment reader for testing without global state mutation
    struct MockEnvReader {
        vars: HashMap<String, String>,
    }

    impl MockEnvReader {
        fn new() -> Self {
            Self {
                vars: HashMap::new(),
            }
        }

        fn with_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
            self.vars.insert(key.into(), value.into());
            self
        }
    }

    impl EnvReader for MockEnvReader {
        fn var(&self, key: &str) -> Result<String, std::env::VarError> {
            self.vars
                .get(key)
                .cloned()
                .ok_or(std::env::VarError::NotPresent)
        }
    }

    #[test]
    fn test_locate_config_file_returns_option() {
        // Should not panic even if no config exists
        let result = locate_config_file(None);
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn test_locate_config_file_with_override() {
        use std::fs;
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_rb_override.toml");

        // Create a temporary config file
        fs::write(&config_path, "# test config").expect("Failed to write test config");

        // Should return the override path
        let result = locate_config_file(Some(config_path.clone()));
        assert_eq!(result, Some(config_path.clone()));

        // Cleanup
        let _ = fs::remove_file(&config_path);
    }

    #[test]
    fn test_locate_config_file_with_env_var() {
        use std::fs;
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_rb_env_mock.toml");

        // Create a temporary config file
        fs::write(&config_path, "# test config").expect("Failed to write test config");

        // Use mock environment - no global state mutation!
        let mock_env =
            MockEnvReader::new().with_var("RB_CONFIG", config_path.to_string_lossy().to_string());

        // Should return the env var path
        let result = locate_config_file_with_env(None, &mock_env);
        assert_eq!(result, Some(config_path.clone()));

        // Cleanup
        let _ = fs::remove_file(&config_path);
    }

    #[test]
    fn test_locate_config_file_with_xdg_config_home() {
        use std::fs;
        let temp_dir = std::env::temp_dir();
        let xdg_base = temp_dir.join("test_xdg_config");
        let rb_dir = xdg_base.join("rb");
        let config_path = rb_dir.join("rb.toml");

        // Create directory structure
        fs::create_dir_all(&rb_dir).expect("Failed to create test directory");
        fs::write(&config_path, "# test config").expect("Failed to write test config");

        // Use mock environment
        let mock_env = MockEnvReader::new()
            .with_var("XDG_CONFIG_HOME", xdg_base.to_string_lossy().to_string());

        // Should return the XDG config path
        let result = locate_config_file_with_env(None, &mock_env);
        assert_eq!(result, Some(config_path.clone()));

        // Cleanup
        let _ = fs::remove_dir_all(&xdg_base);
    }
}
