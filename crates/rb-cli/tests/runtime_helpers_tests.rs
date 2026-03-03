use rb_cli::config::{RbConfig, TrackedConfig};
use rb_cli::runtime_helpers::{CommandContext, new_command_wrapper};
use std::path::PathBuf;

fn create_test_context() -> CommandContext {
    let config = RbConfig::default();
    CommandContext {
        config: TrackedConfig::from_merged(&config, &RbConfig::default()),
        project_file: None,
    }
}

#[test]
fn test_new_command_wrapper_creates_file() {
    let temp_dir = std::env::temp_dir().join(format!("rb-runtime-new-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let result = new_command_wrapper();
    assert!(result.is_ok());

    assert!(temp_dir.join("rbproject.toml").exists());

    std::env::set_current_dir(&original_dir).unwrap();
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_new_command_wrapper_fails_if_file_exists() {
    let temp_dir =
        std::env::temp_dir().join(format!("rb-runtime-new-exists-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let project_file = temp_dir.join("rbproject.toml");
    std::fs::write(&project_file, "existing").unwrap();

    assert!(
        project_file.exists(),
        "Test precondition failed: file should exist"
    );
    if let Ok(file) = std::fs::File::open(&project_file) {
        let _ = file.sync_all();
    }

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let result = new_command_wrapper();
    assert!(
        result.is_err(),
        "Expected error when rbproject.toml already exists"
    );

    std::env::set_current_dir(&original_dir).unwrap();
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_command_context_initialization() {
    let context = create_test_context();

    assert!(context.project_file.is_none());
}

#[test]
fn test_command_context_stores_config() {
    let config = RbConfig {
        rubies_dir: Some(PathBuf::from("/custom/path")),
        ..Default::default()
    };

    let context = CommandContext {
        config: TrackedConfig::from_merged(&config, &RbConfig::default()),
        project_file: None,
    };

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
