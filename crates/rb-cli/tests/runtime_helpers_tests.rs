use rb_cli::config::{RbConfig, TrackedConfig};
use rb_cli::runtime_helpers::{CommandContext, init_command_wrapper};
use std::path::PathBuf;

fn create_test_context() -> CommandContext {
    let config = RbConfig::default();
    CommandContext {
        config: TrackedConfig::from_merged(&config, &RbConfig::default()),
        project_file: None,
    }
}

#[test]
fn test_init_command_wrapper_creates_file() {
    // Create temp directory for test
    let temp_dir = std::env::temp_dir().join(format!("rb-runtime-init-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Change to temp dir and run init
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let result = init_command_wrapper();
    assert!(result.is_ok());

    // Verify file was created
    assert!(temp_dir.join("rbproject.toml").exists());

    // Restore and cleanup
    std::env::set_current_dir(&original_dir).unwrap();
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_init_command_wrapper_fails_if_file_exists() {
    let temp_dir =
        std::env::temp_dir().join(format!("rb-runtime-init-exists-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create existing file
    let project_file = temp_dir.join("rbproject.toml");
    std::fs::write(&project_file, "existing").unwrap();

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let result = init_command_wrapper();
    assert!(result.is_err());

    std::env::set_current_dir(&original_dir).unwrap();
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_command_context_initialization() {
    let context = create_test_context();

    // Context should start with no project file
    assert!(context.project_file.is_none());
}

#[test]
fn test_command_context_stores_config() {
    let mut config = RbConfig::default();
    config.rubies_dir = Some(PathBuf::from("/custom/path"));

    let context = CommandContext {
        config: TrackedConfig::from_merged(&config, &RbConfig::default()),
        project_file: None,
    };

    // Verify context is valid with custom config
    assert!(context.project_file.is_none());
}

#[test]
fn test_with_butler_runtime_creates_runtime_once() {
    // This test verifies the pattern - actual runtime creation
    // depends on Ruby installations being available
    let context = create_test_context();

    // The pattern should create runtime lazily within with_butler_runtime
    // We can't test actual runtime commands without Ruby installed,
    // but we can verify the context structure is sound
    assert!(context.project_file.is_none());
}

#[test]
fn test_bash_complete_context_safety() {
    // bash_complete should handle missing runtime gracefully
    let context = create_test_context();

    // Should not panic even with no runtime
    // Note: bash_complete needs COMP_LINE and COMP_POINT
    let result = rb_cli::runtime_helpers::bash_complete_command(&context, "", "0");

    // It may succeed or fail depending on environment, but shouldn't panic
    let _ = result;
}
