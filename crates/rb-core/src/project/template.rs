use std::fs;
use std::path::Path;

/// Default template content for rbproject.toml
pub const DEFAULT_RBPROJECT_TOML: &str = r#"[project]
name = "Butler project template"
description = "Please fill in"

[scripts]
ruby-version = "ruby -v"
"#;

/// Create a new rbproject.toml file in the specified directory
///
/// This function creates a default rbproject.toml template. It will fail if the file
/// already exists, as overwriting existing configurations would be improper.
///
/// # Arguments
///
/// * `current_dir` - The directory where the rbproject.toml should be created
///
/// # Returns
///
/// * `Ok(())` - Successfully created the file
/// * `Err(String)` - Error message if creation fails (file exists or I/O error)
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use rb_core::project::create_default_project;
///
/// let result = create_default_project(Path::new("."));
/// assert!(result.is_ok());
/// ```
pub fn create_default_project(current_dir: &Path) -> Result<(), String> {
    let project_file = current_dir.join("rbproject.toml");

    // Check if file already exists
    if project_file.exists() {
        return Err("rbproject.toml file already exists in this directory.\n\
             Cannot overwrite existing project configurations without explicit instruction.\n\
             If you wish to recreate the file, please delete the existing one first."
            .to_string());
    }

    // Write the default template
    fs::write(&project_file, DEFAULT_RBPROJECT_TOML)
        .map_err(|e| format!("Failed to create rbproject.toml: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_default_project_creates_file() {
        let temp_dir =
            std::env::temp_dir().join(format!("rb-template-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let result = create_default_project(&temp_dir);

        assert!(result.is_ok());
        let project_file = temp_dir.join("rbproject.toml");
        assert!(project_file.exists());

        let content = fs::read_to_string(&project_file).unwrap();
        assert!(content.contains("[project]"));
        assert!(content.contains("name = \"Butler project template\""));
        assert!(content.contains("[scripts]"));
        assert!(content.contains("ruby-version = \"ruby -v\""));

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_create_default_project_fails_if_file_exists() {
        let temp_dir =
            std::env::temp_dir().join(format!("rb-template-test-exists-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let project_file = temp_dir.join("rbproject.toml");

        // Create existing file
        fs::write(&project_file, "existing content").unwrap();

        let result = create_default_project(&temp_dir);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("already exists"));

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_create_default_project_creates_valid_toml() {
        let temp_dir =
            std::env::temp_dir().join(format!("rb-template-test-valid-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let result = create_default_project(&temp_dir);

        assert!(result.is_ok());
        let project_file = temp_dir.join("rbproject.toml");
        let content = fs::read_to_string(&project_file).unwrap();

        // Verify it's valid TOML
        let parsed: Result<toml::Value, _> = toml::from_str(&content);
        assert!(parsed.is_ok(), "Generated TOML should be valid");

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }
}
