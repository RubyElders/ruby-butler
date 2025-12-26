use rb_core::project::create_default_project;
use std::path::Path;

/// Initialize a new rbproject.toml in the current directory
pub fn init_command(current_dir: &Path) -> Result<(), String> {
    // Delegate to rb-core for file creation
    create_default_project(current_dir)?;

    // Present success message with ceremony
    println!("✨ Splendid! A new rbproject.toml has been created with appropriate ceremony.");
    println!();
    println!("📝 This template includes:");
    println!("   • Project metadata (name and description)");
    println!("   • A sample script (ruby-version) to demonstrate usage");
    println!();
    println!("🎯 You may now:");
    println!("   • Edit rbproject.toml to add your own scripts");
    println!("   • Run 'rb run' to list available scripts");
    println!("   • Execute scripts with: rb run <script-name>");
    println!();
    println!("For comprehensive examples, please consult:");
    println!("   https://github.com/RubyElders/ruby-butler/blob/main/examples/rbproject.toml");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_init_creates_rbproject_toml() {
        let temp_dir = std::env::temp_dir().join(format!("rb-init-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let result = init_command(&temp_dir);

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
    fn test_init_fails_if_file_exists() {
        let temp_dir =
            std::env::temp_dir().join(format!("rb-init-test-exists-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let project_file = temp_dir.join("rbproject.toml");

        // Create existing file
        fs::write(&project_file, "existing content").unwrap();

        let result = init_command(&temp_dir);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("already exists"));

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_init_creates_valid_toml() {
        let temp_dir =
            std::env::temp_dir().join(format!("rb-init-test-valid-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let result = init_command(&temp_dir);

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
