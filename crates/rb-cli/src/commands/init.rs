use std::fs;
use std::path::Path;

const DEFAULT_RBPROJECT_TOML: &str = r#"[project]
name = "Butler project template"
description = "Please fill in"

[scripts]
ruby-version = "ruby -v"
"#;

/// Initialize a new rbproject.toml in the current directory
pub fn init_command(current_dir: &Path) -> Result<(), String> {
    let project_file = current_dir.join("rbproject.toml");

    // Check if file already exists
    if project_file.exists() {
        return Err(
            "🎩 My sincerest apologies, but an rbproject.toml file already graces\n\
             this directory with its presence.\n\n\
             This humble Butler cannot overwrite existing project configurations\n\
             without explicit instruction, as such an action would be most improper.\n\n\
             If you wish to recreate the file, kindly delete the existing one first."
                .to_string(),
        );
    }

    // Write the default template
    fs::write(&project_file, DEFAULT_RBPROJECT_TOML)
        .map_err(|e| format!("Failed to create rbproject.toml: {}", e))?;

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
        assert!(error.contains("already graces"));
        assert!(error.contains("this directory"));

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
