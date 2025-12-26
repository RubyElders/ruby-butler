use rb_cli::error_display::{error_exit_code, format_command_not_found, format_no_suitable_ruby};
use rb_core::butler::ButlerError;
use std::path::PathBuf;

#[test]
fn test_format_no_suitable_ruby_contains_key_info() {
    let rubies_dir = PathBuf::from("/home/user/.rubies");
    let message = format_no_suitable_ruby(
        &rubies_dir,
        "config".to_string(),
        Some(("3.3.0".to_string(), "command-line".to_string())),
    );

    assert!(message.contains(".rubies"));
    assert!(message.contains("config"));
    assert!(message.contains("3.3.0"));
}

#[test]
fn test_format_no_suitable_ruby_without_version() {
    let rubies_dir = PathBuf::from("/usr/local/rubies");
    let message = format_no_suitable_ruby(&rubies_dir, "default".to_string(), None);

    assert!(message.contains("rubies"));
    assert!(message.contains("default"));
    assert!(message.contains("install"));
}

#[test]
fn test_format_command_not_found_contains_command_name() {
    let message = format_command_not_found("nonexistent_command");

    assert!(message.contains("nonexistent_command"));
    assert!(message.contains("absent"));
}

#[test]
fn test_format_command_not_found_provides_guidance() {
    let message = format_command_not_found("rake");

    assert!(message.contains("install") || message.contains("gem") || message.contains("bundle"));
}

#[test]
fn test_error_exit_code_returns_1_for_no_suitable_ruby() {
    let error = ButlerError::NoSuitableRuby("test".to_string());
    assert_eq!(error_exit_code(&error), 1);
}

#[test]
fn test_error_exit_code_returns_127_for_command_not_found() {
    let error = ButlerError::CommandNotFound("test".to_string());
    assert_eq!(error_exit_code(&error), 127);
}

#[test]
fn test_error_exit_code_returns_1_for_general_error() {
    let error = ButlerError::General("test error".to_string());
    assert_eq!(error_exit_code(&error), 1);
}

#[test]
fn test_error_exit_code_returns_1_for_rubies_directory_not_found() {
    let error = ButlerError::RubiesDirectoryNotFound(PathBuf::from("/test"));
    assert_eq!(error_exit_code(&error), 1);
}
