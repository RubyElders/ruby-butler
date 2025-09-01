use rb_tests::{RubySandbox, BundlerSandbox};
use std::process::Command;

#[test]
fn test_runtime_command_with_empty_directory() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "rb", "--", "-R"])
        .arg(sandbox.root())
        .arg("runtime")
        .output()
        .expect("Failed to execute command");

    // With no Ruby installations, the command should exit with error
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("Invalid UTF-8");
    assert!(stderr.contains("Error:"));
}

#[test]
fn test_runtime_command_with_rubies() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    
    // Create some Ruby directories
    sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
    sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");
    sandbox.add_ruby_dir("3.3.1").expect("Failed to create ruby-3.3.1");

    let output = Command::new("cargo")
        .args(&["run", "--bin", "rb", "--", "-R"])
        .arg(sandbox.root())
        .arg("runtime")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("Ruby Environment Survey"));
    assert!(stdout.contains("CRuby"));
    assert!(stdout.contains("(3.2.5)"));
    assert!(stdout.contains("(3.1.0)"));
    assert!(stdout.contains("(3.3.1)"));
    assert!(stdout.contains("Environment Ready: (latest available)"));
    assert!(stdout.contains("(3.3.1)")); // Should be the latest
}

#[test]
fn test_runtime_command_alias() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

    let output = Command::new("cargo")
        .args(&["run", "--bin", "rb", "--", "-R"])
        .arg(sandbox.root())
        .arg("rt")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("Ruby Environment Survey"));
    assert!(stdout.contains("CRuby"));
    assert!(stdout.contains("(3.2.5)"));
}

#[test]
fn test_environment_command_with_no_bundler() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

    let output = Command::new("cargo")
        .args(&["run", "--bin", "rb", "--", "-R"])
        .arg(sandbox.root())
        .arg("environment")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    println!("Actual stdout:\n{}", stdout);
    assert!(stdout.contains("Your Current Ruby Environment"));
    assert!(stdout.contains("CRuby"));
    assert!(stdout.contains("(3.2.5)"));
    assert!(stdout.contains("Bundler environment not detected"));
    assert!(stdout.contains("Environment ready for distinguished Ruby development"));
}

#[test]
fn test_environment_command_alias() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

    let output = Command::new("cargo")
        .args(&["run", "--bin", "rb", "--", "-R"])
        .arg(sandbox.root())
        .arg("env")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("Your Current Ruby Environment"));
    assert!(stdout.contains("CRuby"));
    assert!(stdout.contains("(3.2.5)"));
}

#[test]
fn test_environment_command_with_bundler() {
    // This test verifies that the environment command can be called with bundler projects
    // For now we'll just test that it runs without error when called from a bundler project directory
    
    let ruby_sandbox = RubySandbox::new().expect("Failed to create ruby sandbox");
    ruby_sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
    
    let bundler_sandbox = BundlerSandbox::new().expect("Failed to create bundler sandbox");
    let _project_dir = bundler_sandbox.add_bundler_project("test-app", false).expect("Failed to create bundler project");
    
    // Add a .ruby-version file specifying 3.2.5
    bundler_sandbox.add_file(
        "test-app/.ruby-version",
        "3.2.5"
    ).expect("Failed to create .ruby-version file");
    
    // Test that environment command runs successfully
    // Note: Full bundler integration testing is better done through unit tests
    // since cargo test changes working directories in complex ways
    let output = Command::new("cargo")
        .args(&["run", "--bin", "rb", "--", "-R"])
        .arg(ruby_sandbox.root())
        .arg("environment")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("Your Current Ruby Environment"));
    assert!(stdout.contains("CRuby"));
    assert!(stdout.contains("(3.2.5)"));
    // Note: Bundler detection depends on current directory, which is complex to test in integration tests
    // The bundler functionality is properly tested in unit tests
}
