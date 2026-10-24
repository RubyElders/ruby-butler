use rb_cli::config::{RbConfig, TrackedConfig};
use rb_cli::runtime_helpers::CommandContext;
use std::{path::PathBuf, process::Command};

fn create_test_context() -> CommandContext {
    let config = RbConfig::default();
    CommandContext {
        config: TrackedConfig::from_merged(&config, &RbConfig::default()),
        project_file: None,
    }
}

#[test]
fn test_new_command_wrapper_creates_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rb"))
        .arg("new")
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(temp_dir.path().join("rbproject.toml").exists());
}

#[test]
fn test_new_command_wrapper_fails_if_file_exists() {
    let temp_dir = tempfile::tempdir().unwrap();
    let project_file = temp_dir.path().join("rbproject.toml");
    std::fs::write(&project_file, "existing").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_rb"))
        .arg("new")
        .current_dir(temp_dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("already graces this directory"),
        "{output:?}"
    );
    assert_eq!(std::fs::read_to_string(project_file).unwrap(), "existing");
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
fn test_bash_complete_context_safety() {
    let mut context = create_test_context();
    let directory = tempfile::tempdir().unwrap();
    context.config.rubies_dir =
        rb_cli::config::value::ConfigValue::from_cli(directory.path().join("missing"));
    assert!(rb_cli::runtime_helpers::bash_complete_command(&context, "rb sy", "5").is_ok());
}
