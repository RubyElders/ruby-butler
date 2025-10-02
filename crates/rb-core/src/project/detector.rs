use log::{debug, info};
use std::path::Path;

use super::ProjectRuntime;

pub struct RbprojectDetector;

impl RbprojectDetector {
    /// Discover a ProjectRuntime by searching for rbproject.toml in the current directory
    /// and walking up the directory tree until one is found or we reach the root.
    pub fn discover(start_dir: &Path) -> std::io::Result<Option<ProjectRuntime>> {
        debug!(
            "Searching for rbproject.toml starting from directory: {}",
            start_dir.display()
        );

        let mut current_dir = start_dir.to_path_buf();

        loop {
            debug!(
                "Checking directory for rbproject.toml: {}",
                current_dir.display()
            );
            let rbproject_path = current_dir.join("rbproject.toml");

            if rbproject_path.exists() && rbproject_path.is_file() {
                info!("Discovered rbproject.toml at: {}", rbproject_path.display());

                // Parse the file and create ProjectRuntime
                match ProjectRuntime::from_file(&rbproject_path) {
                    Ok(project_runtime) => {
                        debug!("Created ProjectRuntime for root: {}", current_dir.display());
                        return Ok(Some(project_runtime));
                    }
                    Err(e) => {
                        debug!(
                            "Failed to parse rbproject.toml at {}: {}",
                            rbproject_path.display(),
                            e
                        );
                        return Err(e);
                    }
                }
            } else {
                debug!("No rbproject.toml found in: {}", current_dir.display());
            }

            // Move up one directory
            match current_dir.parent() {
                Some(parent) => {
                    current_dir = parent.to_path_buf();
                    debug!("Moving up to parent directory: {}", current_dir.display());
                }
                None => {
                    debug!("Reached filesystem root, no rbproject.toml found");
                    break;
                }
            }
        }

        info!(
            "No rbproject.toml found starting from: {}",
            start_dir.display()
        );
        Ok(None)
    }

    /// Convenience method to discover from current working directory
    pub fn discover_from_cwd() -> std::io::Result<Option<ProjectRuntime>> {
        let cwd = std::env::current_dir()?;
        debug!(
            "Discovering project runtime from current working directory: {}",
            cwd.display()
        );
        Self::discover(&cwd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io;
    use tempfile::TempDir;

    fn create_rbproject_toml(dir: &Path, content: &str) -> io::Result<()> {
        let rbproject_path = dir.join("rbproject.toml");
        fs::write(rbproject_path, content)?;
        Ok(())
    }

    #[test]
    fn discover_finds_rbproject_in_current_directory() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path();

        let toml_content = r#"
[scripts]
test = "rspec"
lint = "rubocop"
"#;
        create_rbproject_toml(project_dir, toml_content)?;

        let result = RbprojectDetector::discover(project_dir)?;

        assert!(result.is_some());
        let project_runtime = result.unwrap();
        assert_eq!(project_runtime.root, project_dir);
        assert_eq!(project_runtime.scripts.len(), 2);
        assert_eq!(project_runtime.get_script_command("test"), Some("rspec"));
        assert_eq!(project_runtime.get_script_command("lint"), Some("rubocop"));

        Ok(())
    }

    #[test]
    fn discover_finds_rbproject_in_parent_directory() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path();

        let toml_content = r#"
[scripts]
server = "rails server"
"#;
        create_rbproject_toml(project_dir, toml_content)?;

        // Create nested directory structure
        let nested_dir = project_dir.join("app").join("controllers");
        fs::create_dir_all(&nested_dir)?;

        let result = RbprojectDetector::discover(&nested_dir)?;

        assert!(result.is_some());
        let project_runtime = result.unwrap();
        assert_eq!(project_runtime.root, project_dir);
        assert_eq!(project_runtime.scripts.len(), 1);
        assert_eq!(
            project_runtime.get_script_command("server"),
            Some("rails server")
        );

        Ok(())
    }

    #[test]
    fn discover_returns_none_when_no_rbproject_found() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path();

        let result = RbprojectDetector::discover(project_dir)?;

        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn discover_stops_at_first_rbproject_found() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let root_dir = temp_dir.path();

        // Create root rbproject.toml
        let root_toml = r#"
[scripts]
root = "root command"
"#;
        create_rbproject_toml(root_dir, root_toml)?;

        // Create nested project with its own rbproject.toml
        let nested_dir = root_dir.join("subproject");
        fs::create_dir_all(&nested_dir)?;
        let nested_toml = r#"
[scripts]
nested = "nested command"
"#;
        create_rbproject_toml(&nested_dir, nested_toml)?;

        // Create deep directory in nested project
        let deep_dir = nested_dir.join("lib").join("deep");
        fs::create_dir_all(&deep_dir)?;

        // Search from deep directory - should find subproject rbproject.toml, not root
        let result = RbprojectDetector::discover(&deep_dir)?;

        assert!(result.is_some());
        let project_runtime = result.unwrap();
        assert_eq!(project_runtime.root, nested_dir);
        assert_eq!(project_runtime.scripts.len(), 1);
        assert_eq!(
            project_runtime.get_script_command("nested"),
            Some("nested command")
        );
        assert_eq!(project_runtime.get_script_command("root"), None);

        Ok(())
    }

    #[test]
    fn discover_handles_empty_scripts_section() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path();

        let toml_content = r#"
[scripts]
"#;
        create_rbproject_toml(project_dir, toml_content)?;

        let result = RbprojectDetector::discover(project_dir)?;

        assert!(result.is_some());
        let project_runtime = result.unwrap();
        assert_eq!(project_runtime.scripts.len(), 0);

        Ok(())
    }

    #[test]
    fn discover_handles_missing_scripts_section() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path();

        let toml_content = r#"
[other_section]
key = "value"
"#;
        create_rbproject_toml(project_dir, toml_content)?;

        let result = RbprojectDetector::discover(project_dir)?;

        assert!(result.is_some());
        let project_runtime = result.unwrap();
        assert_eq!(project_runtime.scripts.len(), 0);

        Ok(())
    }

    #[test]
    fn discover_returns_error_for_invalid_toml() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path();

        let invalid_toml = r#"
[scripts
this is not valid toml
"#;
        create_rbproject_toml(project_dir, invalid_toml)?;

        let result = RbprojectDetector::discover(project_dir);

        assert!(result.is_err());

        Ok(())
    }
}
