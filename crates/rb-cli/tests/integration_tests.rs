use rb_tests::RubySandbox;
use rb_core::ruby::RubyRuntimeDetector;
use std::process::Command;

#[test]
fn test_runtime_command_with_empty_directory() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "rb", "--", "-R"])
        .arg(sandbox.root())
        .arg("runtime")
        .current_dir("f:\\sync\\elders\\ruby-butler2\\crates\\rb-cli")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("Following rubies were found"));
    assert!(stdout.contains("No Ruby installations found"));
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
        .current_dir("f:\\sync\\elders\\ruby-butler2\\crates\\rb-cli")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("Following rubies were found"));
    assert!(stdout.contains("CRuby"));
    assert!(stdout.contains("(3.2.5)"));
    assert!(stdout.contains("(3.1.0)"));
    assert!(stdout.contains("(3.3.1)"));
    assert!(stdout.contains("Ruby detected: (latest)"));
    assert!(stdout.contains("(3.3.1)")); // Should be the latest
}

#[test]
fn test_ruby_detector_integration() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    
    // Create Ruby directories
    sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
    sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");

    let rubies = RubyRuntimeDetector::discover(sandbox.root())
        .expect("Failed to discover rubies");

    assert_eq!(rubies.len(), 2);
    
    // Should be sorted with latest first
    assert_eq!(rubies[0].version.to_string(), "3.2.5");
    assert_eq!(rubies[1].version.to_string(), "3.1.0");

    let latest = RubyRuntimeDetector::latest(&rubies);
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().version.to_string(), "3.2.5");
}

#[test]
fn test_runtime_command_alias() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

    let output = Command::new("cargo")
        .args(&["run", "--bin", "rb", "--", "-R"])
        .arg(sandbox.root())
        .arg("rt")
        .current_dir("f:\\sync\\elders\\ruby-butler2\\crates\\rb-cli")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("Following rubies were found"));
    assert!(stdout.contains("CRuby"));
    assert!(stdout.contains("(3.2.5)"));
}
