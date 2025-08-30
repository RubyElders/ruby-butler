use rb_tests::RubySandbox;
use rb_core::ruby::RubyRuntimeDetector;
use rb_cli::{create_ruby_context, resolve_search_dir};
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
fn test_create_ruby_context_integration() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
    sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");

    // Test creating Ruby context directly (without spawning cargo processes)
    let butler_runtime = create_ruby_context(Some(sandbox.root().to_path_buf()), None);
    
    let current_path = std::env::var("PATH").ok();
    let env_vars = butler_runtime.env_vars(current_path);
    
    // Verify environment variables are properly set
    assert!(env_vars.contains_key("PATH"));
    assert!(env_vars.contains_key("GEM_HOME"));
    assert!(env_vars.contains_key("GEM_PATH"));
    
    // Verify PATH contains Ruby bin directory
    let path = env_vars.get("PATH").unwrap();
    assert!(path.contains("ruby-3.2.5")); // Should pick the latest version
    
    // Verify GEM_PATH includes GEM_HOME (chruby pattern: GEM_HOME:GEM_ROOT)
    let gem_home = env_vars.get("GEM_HOME").unwrap();
    let gem_path = env_vars.get("GEM_PATH").unwrap();
    assert!(gem_path.contains(gem_home));
}

#[test]
fn test_resolve_search_dir_integration() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    
    // Test with explicit directory
    let result = resolve_search_dir(Some(sandbox.root().to_path_buf()));
    assert_eq!(result, sandbox.root());
    
    // Test with None (should use home/.rubies)
    let result = resolve_search_dir(None);
    assert!(result.ends_with(".rubies"));
    assert!(result.is_absolute());
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
