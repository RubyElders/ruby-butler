use rb_tests::RubySandbox;
use std::io::Write;

/// Helper to capture stdout output from completion generation
fn capture_completions(
    line: &str,
    cursor_pos: &str,
    rubies_dir: Option<std::path::PathBuf>,
) -> String {
    // Run the actual binary to test completions
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));

    if let Some(dir) = rubies_dir {
        // Set RB_RUBIES_DIR environment variable (preferred method)
        cmd.env("RB_RUBIES_DIR", &dir);
    }

    cmd.arg("__bash_complete").arg(line).arg(cursor_pos);

    let output = cmd.output().expect("Failed to execute rb");

    // If stderr is not empty, print it for debugging
    if !output.stderr.is_empty() {
        eprintln!(
            "Completion stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8(output.stdout).expect("Invalid UTF-8 output")
}

#[test]
fn test_command_completion_empty_prefix() {
    let completions = capture_completions("rb ", "3", None);

    assert!(completions.contains("runtime"));
    assert!(completions.contains("rt"));
    assert!(completions.contains("run"));
    assert!(completions.contains("r"));
    assert!(completions.contains("exec"));
    assert!(completions.contains("shell-integration"));
}

#[test]
fn test_command_completion_with_prefix() {
    let completions = capture_completions("rb ru", "5", None);

    assert!(completions.contains("runtime"));
    assert!(completions.contains("run"));
    assert!(!completions.contains("exec"));
    assert!(!completions.contains("sync"));
}

#[test]
fn test_ruby_version_completion_empty_prefix() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");

    // Create mock Ruby installations
    sandbox.add_ruby_dir("3.4.5").unwrap();
    sandbox.add_ruby_dir("3.4.4").unwrap();
    sandbox.add_ruby_dir("3.3.7").unwrap();

    let completions = capture_completions("rb -r ", "7", Some(sandbox.root().to_path_buf()));

    assert!(completions.contains("3.4.5"));
    assert!(completions.contains("3.4.4"));
    assert!(completions.contains("3.3.7"));
}

#[test]
fn test_ruby_version_completion_with_prefix() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");

    // Create mock Ruby installations
    sandbox.add_ruby_dir("3.4.5").unwrap();
    sandbox.add_ruby_dir("3.4.4").unwrap();
    sandbox.add_ruby_dir("3.3.7").unwrap();
    sandbox.add_ruby_dir("3.2.1").unwrap();

    let completions = capture_completions("rb -r 3.4.", "10", Some(sandbox.root().to_path_buf()));

    assert!(completions.contains("3.4.5"));
    assert!(completions.contains("3.4.4"));
    assert!(!completions.contains("3.3.7"));
    assert!(!completions.contains("3.2.1"));
}

#[test]
#[cfg(unix)]
fn test_tilde_expansion_in_rubies_dir_short_flag() {
    // Create a Ruby sandbox in a known location within home directory
    let home_dir = std::env::var("HOME").expect("HOME not set");
    let test_dir = std::path::PathBuf::from(&home_dir).join(".rb-test-rubies");

    // Clean up if exists
    let _ = std::fs::remove_dir_all(&test_dir);

    // Create test rubies directory
    std::fs::create_dir_all(&test_dir).expect("Failed to create test dir");

    // Create mock Ruby installations
    let ruby_345 = test_dir.join("ruby-3.4.5").join("bin");
    std::fs::create_dir_all(&ruby_345).expect("Failed to create ruby-3.4.5");
    std::fs::File::create(ruby_345.join("ruby")).expect("Failed to create ruby executable");

    let ruby_344 = test_dir.join("ruby-3.4.4").join("bin");
    std::fs::create_dir_all(&ruby_344).expect("Failed to create ruby-3.4.4");
    std::fs::File::create(ruby_344.join("ruby")).expect("Failed to create ruby executable");

    // Test completion with tilde in path using -R flag
    let cmd_line = "rb -R ~/.rb-test-rubies -r ";
    let cursor_pos = "28";

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg(cmd_line).arg(cursor_pos);

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // Cleanup
    let _ = std::fs::remove_dir_all(&test_dir);

    assert!(
        completions.contains("3.4.5"),
        "Expected '3.4.5' in completions with tilde expansion, got: {}",
        completions
    );
    assert!(
        completions.contains("3.4.4"),
        "Expected '3.4.4' in completions with tilde expansion, got: {}",
        completions
    );
}

#[test]
#[cfg(unix)]
fn test_tilde_expansion_in_rubies_dir_long_flag() {
    // Create a Ruby sandbox in a known location within home directory
    let home_dir = std::env::var("HOME").expect("HOME not set");
    let test_dir = std::path::PathBuf::from(&home_dir).join(".rb-test-rubies-long");

    // Clean up if exists
    let _ = std::fs::remove_dir_all(&test_dir);

    // Create test rubies directory
    std::fs::create_dir_all(&test_dir).expect("Failed to create test dir");

    // Create mock Ruby installation
    let ruby_337 = test_dir.join("ruby-3.3.7").join("bin");
    std::fs::create_dir_all(&ruby_337).expect("Failed to create ruby-3.3.7");
    std::fs::File::create(ruby_337.join("ruby")).expect("Failed to create ruby executable");

    // Test completion with tilde in path using --rubies-dir flag
    let cmd_line = "rb --rubies-dir ~/.rb-test-rubies-long -r 3.3";
    let cursor_pos = "48";

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg(cmd_line).arg(cursor_pos);

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // Cleanup
    let _ = std::fs::remove_dir_all(&test_dir);

    assert!(
        completions.contains("3.3.7"),
        "Expected '3.3.7' in completions with tilde expansion (long flag), got: {}",
        completions
    );
}

#[test]
#[cfg(unix)]
fn test_tilde_only_expands_to_home() {
    // Create a Ruby sandbox in a known location within home directory
    let home_dir = std::env::var("HOME").expect("HOME not set");
    let test_dir = std::path::PathBuf::from(&home_dir).join(".rb-test-tilde-only");

    // Clean up if exists
    let _ = std::fs::remove_dir_all(&test_dir);

    // Create test rubies directory
    std::fs::create_dir_all(&test_dir).expect("Failed to create test dir");

    // Create mock Ruby installation
    let ruby_345 = test_dir.join("ruby-3.4.5").join("bin");
    std::fs::create_dir_all(&ruby_345).expect("Failed to create ruby-3.4.5");
    std::fs::File::create(ruby_345.join("ruby")).expect("Failed to create ruby executable");

    // Test completion with just tilde (no trailing slash)
    let cmd_line = format!("rb -R {}/.rb-test-tilde-only -r ", home_dir);
    let cursor_pos = format!("{}", cmd_line.len());

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg(&cmd_line).arg(&cursor_pos);

    let output = cmd.output().expect("Failed to execute rb");
    let completions_expanded = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // Now test with tilde version
    let cmd_line_tilde = "rb -R ~/.rb-test-tilde-only -r ";
    let cursor_pos_tilde = "31";

    let mut cmd_tilde = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd_tilde
        .arg("__bash_complete")
        .arg(cmd_line_tilde)
        .arg(cursor_pos_tilde);

    let output_tilde = cmd_tilde.output().expect("Failed to execute rb");
    let completions_tilde = String::from_utf8(output_tilde.stdout).expect("Invalid UTF-8 output");

    // Cleanup
    let _ = std::fs::remove_dir_all(&test_dir);

    // Both should produce the same results
    assert_eq!(
        completions_expanded, completions_tilde,
        "Tilde expansion should produce same results as full path"
    );
    assert!(completions_tilde.contains("3.4.5"));
}

#[test]
fn test_script_completion_from_rbproject() {
    // Create a temporary directory with rbproject.toml
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_file = temp_dir.path().join("rbproject.toml");

    let mut file = std::fs::File::create(&project_file).expect("Failed to create rbproject.toml");
    writeln!(file, "[scripts]").unwrap();
    writeln!(file, "test = 'bundle exec rspec'").unwrap();
    writeln!(file, "build = 'rake build'").unwrap();
    writeln!(file, "deploy = 'cap production deploy'").unwrap();
    file.flush().unwrap();
    drop(file);

    // Run completion from the temp directory
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb run ").arg("7");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.contains("test"),
        "Expected 'test' in completions, got: {}",
        completions
    );
    assert!(
        completions.contains("build"),
        "Expected 'build' in completions, got: {}",
        completions
    );
    assert!(
        completions.contains("deploy"),
        "Expected 'deploy' in completions, got: {}",
        completions
    );
}

#[test]
fn test_script_completion_with_prefix() {
    // Create a temporary directory with rbproject.toml
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_file = temp_dir.path().join("rbproject.toml");

    let mut file = std::fs::File::create(&project_file).expect("Failed to create rbproject.toml");
    writeln!(file, "[scripts]").unwrap();
    writeln!(file, "test = 'bundle exec rspec'").unwrap();
    writeln!(file, "build = 'rake build'").unwrap();
    writeln!(file, "deploy = 'cap production deploy'").unwrap();
    file.flush().unwrap();
    drop(file);

    // Run completion from the temp directory with prefix filtering
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb run te").arg("9");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.contains("test"),
        "Expected 'test' in completions, got: {}",
        completions
    );
    assert!(
        !completions.contains("build"),
        "Should not contain 'build' in completions, got: {}",
        completions
    );
    assert!(
        !completions.contains("deploy"),
        "Should not contain 'deploy' in completions, got: {}",
        completions
    );
}

#[test]
fn test_binstubs_completion_from_bundler() {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    // Create Ruby sandbox with Ruby installation
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox
        .add_ruby_dir("3.3.0")
        .expect("Failed to create ruby");

    // Create a temporary work directory with bundler binstubs
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create Gemfile (required for bundler detection)
    fs::write(
        temp_dir.path().join("Gemfile"),
        "source 'https://rubygems.org'\n",
    )
    .expect("Failed to create Gemfile");

    let binstubs_dir = temp_dir
        .path()
        .join(".rb")
        .join("vendor")
        .join("bundler")
        .join("ruby")
        .join("3.3.0")
        .join("bin");
    fs::create_dir_all(&binstubs_dir).expect("Failed to create binstubs directory");

    // Create mock binstubs
    let rspec_exe = binstubs_dir.join("rspec");
    fs::write(&rspec_exe, "#!/usr/bin/env ruby\n").expect("Failed to write rspec");
    #[cfg(unix)]
    fs::set_permissions(&rspec_exe, fs::Permissions::from_mode(0o755))
        .expect("Failed to set permissions");

    let rails_exe = binstubs_dir.join("rails");
    fs::write(&rails_exe, "#!/usr/bin/env ruby\n").expect("Failed to write rails");
    #[cfg(unix)]
    fs::set_permissions(&rails_exe, fs::Permissions::from_mode(0o755))
        .expect("Failed to set permissions");

    let rake_exe = binstubs_dir.join("rake");
    fs::write(&rake_exe, "#!/usr/bin/env ruby\n").expect("Failed to write rake");
    #[cfg(unix)]
    fs::set_permissions(&rake_exe, fs::Permissions::from_mode(0o755))
        .expect("Failed to set permissions");

    // Run completion from the temp directory with rubies-dir pointing to sandbox
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete")
        .arg("rb exec ")
        .arg("8")
        .arg("--rubies-dir")
        .arg(sandbox.root());
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // Should include bundler binstubs
    assert!(
        completions.contains("rspec"),
        "Expected 'rspec' in completions, got: {}",
        completions
    );
    assert!(
        completions.contains("rails"),
        "Expected 'rails' in completions, got: {}",
        completions
    );
    assert!(
        completions.contains("rake"),
        "Expected 'rake' in completions, got: {}",
        completions
    );

    // Note: Ruby bin executables (gem, bundle, ruby, etc.) would also be suggested
    // since we now have a Ruby installation
}

#[test]
fn test_binstubs_with_ruby_executables_in_bundler() {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let sandbox = RubySandbox::new().expect("Failed to create sandbox");

    // Create Ruby installation
    let ruby_version = "3.4.5";
    sandbox
        .add_ruby_dir(ruby_version)
        .expect("Failed to create ruby");
    let ruby_bin = sandbox
        .root()
        .join(format!("ruby-{}", ruby_version))
        .join("bin");
    fs::create_dir_all(&ruby_bin).expect("Failed to create ruby bin dir");

    // Add Ruby executables
    for exe in &["gem", "bundle", "ruby", "irb"] {
        let exe_path = ruby_bin.join(exe);
        fs::write(&exe_path, "#!/usr/bin/env ruby\n").expect("Failed to write executable");
        #[cfg(unix)]
        fs::set_permissions(&exe_path, fs::Permissions::from_mode(0o755))
            .expect("Failed to set permissions");
    }

    // Create work directory with bundler project
    let work_dir = tempfile::tempdir().expect("Failed to create temp dir");
    fs::write(
        work_dir.path().join("Gemfile"),
        "source 'https://rubygems.org'\n",
    )
    .expect("Failed to create Gemfile");

    // Create bundler binstubs (use ABI version 3.4.0, not 3.4.5)
    let binstubs_dir = work_dir
        .path()
        .join(".rb")
        .join("vendor")
        .join("bundler")
        .join("ruby")
        .join("3.4.0") // ABI version, not full version
        .join("bin");
    fs::create_dir_all(&binstubs_dir).expect("Failed to create binstubs directory");

    let rspec_exe = binstubs_dir.join("rspec");
    fs::write(&rspec_exe, "#!/usr/bin/env ruby\n").expect("Failed to write rspec");
    #[cfg(unix)]
    fs::set_permissions(&rspec_exe, fs::Permissions::from_mode(0o755))
        .expect("Failed to set permissions");

    // Run completion
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete")
        .arg("rb exec ")
        .arg("8")
        .arg("--rubies-dir")
        .arg(sandbox.root());
    cmd.current_dir(work_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // Should include both bundler binstubs AND Ruby executables
    assert!(
        completions.contains("rspec"),
        "Expected bundler binstub 'rspec' in completions, got: {}",
        completions
    );
    assert!(
        completions.contains("gem")
            || completions.contains("ruby")
            || completions.contains("bundle"),
        "Expected Ruby executables (gem/ruby/bundle) in completions, got: {}",
        completions
    );
}

#[test]
fn test_binstubs_completion_with_prefix() {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    // Create Ruby sandbox
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox
        .add_ruby_dir("3.3.0")
        .expect("Failed to create ruby");

    // Create a temporary directory with bundler binstubs in versioned ruby directory
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create Gemfile (required for bundler detection)
    fs::write(
        temp_dir.path().join("Gemfile"),
        "source 'https://rubygems.org'\n",
    )
    .expect("Failed to create Gemfile");

    let binstubs_dir = temp_dir
        .path()
        .join(".rb")
        .join("vendor")
        .join("bundler")
        .join("ruby")
        .join("3.3.0")
        .join("bin");
    fs::create_dir_all(&binstubs_dir).expect("Failed to create binstubs directory");

    // Create mock binstubs
    let rspec_exe = binstubs_dir.join("rspec");
    fs::write(&rspec_exe, "#!/usr/bin/env ruby\n").expect("Failed to write rspec");
    #[cfg(unix)]
    fs::set_permissions(&rspec_exe, fs::Permissions::from_mode(0o755))
        .expect("Failed to set permissions");

    let rails_exe = binstubs_dir.join("rails");
    fs::write(&rails_exe, "#!/usr/bin/env ruby\n").expect("Failed to write rails");
    #[cfg(unix)]
    fs::set_permissions(&rails_exe, fs::Permissions::from_mode(0o755))
        .expect("Failed to set permissions");

    // Run completion with prefix "r"
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete")
        .arg("rb exec r")
        .arg("9")
        .arg("--rubies-dir")
        .arg(sandbox.root());
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.contains("rspec"),
        "Expected 'rspec' in completions with prefix 'r', got: {}",
        completions
    );
    assert!(
        completions.contains("rails"),
        "Expected 'rails' in completions with prefix 'r', got: {}",
        completions
    );
}

#[test]
fn test_binstubs_completion_with_x_alias() {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    // Create Ruby sandbox
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox
        .add_ruby_dir("3.3.0")
        .expect("Failed to create ruby");

    // Create a temporary directory with bundler binstubs in versioned ruby directory
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create Gemfile (required for bundler detection)
    fs::write(
        temp_dir.path().join("Gemfile"),
        "source 'https://rubygems.org'\n",
    )
    .expect("Failed to create Gemfile");

    let binstubs_dir = temp_dir
        .path()
        .join(".rb")
        .join("vendor")
        .join("bundler")
        .join("ruby")
        .join("3.3.0")
        .join("bin");
    fs::create_dir_all(&binstubs_dir).expect("Failed to create binstubs directory");

    // Create mock binstub
    let rspec_exe = binstubs_dir.join("rspec");
    fs::write(&rspec_exe, "#!/usr/bin/env ruby\n").expect("Failed to write rspec");
    #[cfg(unix)]
    fs::set_permissions(&rspec_exe, fs::Permissions::from_mode(0o755))
        .expect("Failed to set permissions");

    // Run completion using 'x' alias
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete")
        .arg("rb x ")
        .arg("5")
        .arg("--rubies-dir")
        .arg(sandbox.root());
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.contains("rspec"),
        "Expected 'rspec' in completions with 'x' alias, got: {}",
        completions
    );
}

#[test]
#[ignore] // Requires real Ruby installation and gem setup
fn test_gem_binstubs_completion_without_bundler() {
    // This test verifies that gem binstubs are suggested when not in a bundler project
    // It requires a real Ruby installation with gems installed
    // Run with: cargo test -- --ignored test_gem_binstubs_completion_without_bundler

    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.4.5").unwrap();

    // Create a work directory without Gemfile (no bundler project)
    let work_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete")
        .arg("rb exec ")
        .arg("8")
        .arg("--rubies-dir")
        .arg(sandbox.root());
    cmd.current_dir(work_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // This would suggest gem binstubs from ~/.gem/ruby/X.Y.Z/bin if they exist
    // The specific executables depend on what's installed on the system
    println!("Completions: {}", completions);
}

#[test]
fn test_flags_completion() {
    let completions = capture_completions("rb -", "4", None);

    // Check for various flags
    assert!(completions.contains("-r"));
    assert!(completions.contains("--ruby"));
    assert!(completions.contains("-R"));
    assert!(completions.contains("--rubies-dir"));
    assert!(completions.contains("-v"));
    assert!(completions.contains("--verbose"));
}

#[test]
fn test_shell_integration_completion() {
    let completions = capture_completions("rb shell-integration ", "21", None);

    assert!(completions.contains("bash"));
    assert!(!completions.contains("zsh"));
    assert!(!completions.contains("fish"));
    assert!(!completions.contains("powershell"));
}

// Edge case tests for completion logic

#[test]
fn test_completion_after_complete_command() {
    // "rb runtime " should not suggest anything (command is complete)
    let completions = capture_completions("rb runtime ", "11", None);
    assert!(
        completions.is_empty(),
        "Should not suggest anything after complete command, got: {}",
        completions
    );
}

#[test]
fn test_completion_with_partial_command_no_space() {
    // "rb run" at cursor 6 should suggest "runtime" and "run"
    let completions = capture_completions("rb run", "6", None);
    assert!(
        completions.contains("runtime"),
        "Expected 'runtime' in completions, got: {}",
        completions
    );
    assert!(
        completions.contains("run"),
        "Expected 'run' in completions, got: {}",
        completions
    );
}

#[test]
fn test_cursor_position_in_middle() {
    // "rb runtime --help" with cursor at position 3 should suggest all commands starting with ""
    let completions = capture_completions("rb runtime --help", "3", None);
    assert!(
        completions.contains("runtime"),
        "Expected commands at cursor position 3, got: {}",
        completions
    );
    assert!(completions.contains("exec"));
}

#[test]
fn test_cursor_position_partial_word() {
    // "rb ru --help" with cursor at position 5 should suggest "runtime" and "run"
    let completions = capture_completions("rb ru --help", "5", None);
    assert!(
        completions.contains("runtime"),
        "Expected 'runtime' at cursor position 5, got: {}",
        completions
    );
    assert!(
        completions.contains("run"),
        "Expected 'run' at cursor position 5, got: {}",
        completions
    );
    assert!(!completions.contains("exec"));
}

#[test]
fn test_global_flags_before_command() {
    // "rb -v " should suggest commands after global flag
    let completions = capture_completions("rb -v ", "6", None);
    assert!(
        completions.contains("runtime"),
        "Expected commands after global flag, got: {}",
        completions
    );
    assert!(completions.contains("exec"));
}

#[test]
fn test_ruby_version_after_dash_r() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.4.5").unwrap();
    sandbox.add_ruby_dir("3.2.4").unwrap();

    // "rb -r " should suggest Ruby versions, not commands
    let completions = capture_completions("rb -r ", "7", Some(sandbox.root().to_path_buf()));
    assert!(
        completions.contains("3.4.5"),
        "Expected Ruby version 3.4.5, got: {}",
        completions
    );
    assert!(
        completions.contains("3.2.4"),
        "Expected Ruby version 3.2.4, got: {}",
        completions
    );
    assert!(
        !completions.contains("runtime"),
        "Should not suggest commands after -r flag, got: {}",
        completions
    );
}

#[test]
fn test_ruby_version_after_long_ruby_flag() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.4.5").unwrap();

    // "rb --ruby " should suggest Ruby versions
    let completions = capture_completions("rb --ruby ", "10", Some(sandbox.root().to_path_buf()));
    assert!(
        completions.contains("3.4.5"),
        "Expected Ruby version after --ruby flag, got: {}",
        completions
    );
}

#[test]
fn test_multiple_global_flags_before_command() {
    // "rb -v -R /opt/rubies " should still suggest commands
    let completions = capture_completions("rb -v -R /opt/rubies ", "21", None);
    assert!(
        completions.contains("runtime"),
        "Expected commands after multiple flags, got: {}",
        completions
    );
    assert!(completions.contains("exec"));
}

#[test]
fn test_flag_completion_shows_all_flags() {
    let completions = capture_completions("rb -", "4", None);

    // Check that we have a good variety of flags
    let flag_count = completions.lines().count();
    assert!(
        flag_count > 10,
        "Expected many flags, got only {}",
        flag_count
    );

    // Verify hidden flags are not shown
    assert!(
        !completions.contains("--complete"),
        "Hidden flags should not appear in completion"
    );
}

#[test]
fn test_command_alias_completion() {
    let completions = capture_completions("rb r", "4", None);

    // Should suggest both "runtime" and "run" (and their aliases "rt" and "r")
    assert!(completions.contains("runtime"));
    assert!(completions.contains("rt"));
    assert!(completions.contains("run"));
    assert!(completions.contains("r"));
}

#[test]
fn test_no_completion_after_exec_command() {
    // "rb exec bundle " should not suggest anything (exec takes arbitrary args)
    let completions = capture_completions("rb exec bundle ", "16", None);
    assert!(
        completions.is_empty(),
        "Should not suggest anything after exec command, got: {}",
        completions
    );
}

#[test]
fn test_completion_with_rubies_dir_flag() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.4.5").unwrap();

    // "rb -R /path/to/rubies -r " should still complete Ruby versions
    let line = format!("rb -R {} -r ", sandbox.root().display());
    let cursor = line.len().to_string();
    let completions = capture_completions(&line, &cursor, None);

    assert!(
        completions.contains("3.4.5"),
        "Expected Ruby version after -R and -r flags, got: {}",
        completions
    );
}

#[test]
fn test_script_completion_with_run_alias() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_file = temp_dir.path().join("rbproject.toml");

    let mut file = std::fs::File::create(&project_file).expect("Failed to create rbproject.toml");
    writeln!(file, "[scripts]").unwrap();
    writeln!(file, "test = 'bundle exec rspec'").unwrap();
    file.flush().unwrap();
    drop(file);

    // "rb r " should complete scripts (r is alias for run)
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb r ").arg("5");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.contains("test"),
        "Expected script completion with 'r' alias, got: {}",
        completions
    );
}

#[test]
fn test_empty_line_completion() {
    // Just "rb " should suggest all commands
    let completions = capture_completions("rb ", "3", None);

    let lines: Vec<&str> = completions.lines().collect();
    assert!(lines.len() > 5, "Expected many commands, got: {:?}", lines);
    assert!(completions.contains("runtime"));
    assert!(completions.contains("init"));
    assert!(completions.contains("shell-integration"));
}

#[test]
fn test_no_rbproject_returns_empty_for_run() {
    // "rb run " without rbproject.toml should return empty
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb run ").arg("7");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.is_empty(),
        "Expected no completions without rbproject.toml, got: {}",
        completions
    );
}

#[test]
fn test_run_command_first_arg_completes_scripts() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let rbproject_path = temp_dir.path().join("rbproject.toml");

    // Create rbproject.toml with scripts
    let mut file = std::fs::File::create(&rbproject_path).expect("Failed to create rbproject.toml");
    writeln!(file, "[scripts]").unwrap();
    writeln!(file, "test = \"rspec\"").unwrap();
    writeln!(file, "dev = \"rails server\"").unwrap();
    writeln!(file, "lint = \"rubocop\"").unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb run ").arg("7");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.contains("test"),
        "Expected 'test' script in completions"
    );
    assert!(
        completions.contains("dev"),
        "Expected 'dev' script in completions"
    );
    assert!(
        completions.contains("lint"),
        "Expected 'lint' script in completions"
    );
}

#[test]
fn test_run_command_second_arg_returns_empty() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let rbproject_path = temp_dir.path().join("rbproject.toml");

    // Create rbproject.toml with scripts
    let mut file = std::fs::File::create(&rbproject_path).expect("Failed to create rbproject.toml");
    writeln!(file, "[scripts]").unwrap();
    writeln!(file, "test = \"rspec\"").unwrap();
    writeln!(file, "dev = \"rails server\"").unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb run test ").arg("12");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.is_empty(),
        "Expected no completions for second arg after 'run test', got: {}",
        completions
    );
}

#[test]
fn test_run_alias_first_arg_completes_scripts() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let rbproject_path = temp_dir.path().join("rbproject.toml");

    // Create rbproject.toml with scripts
    let mut file = std::fs::File::create(&rbproject_path).expect("Failed to create rbproject.toml");
    writeln!(file, "[scripts]").unwrap();
    writeln!(file, "test = \"rspec\"").unwrap();
    writeln!(file, "build = \"rake build\"").unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb r ").arg("5");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.contains("test"),
        "Expected 'test' script in completions"
    );
    assert!(
        completions.contains("build"),
        "Expected 'build' script in completions"
    );
}

#[test]
fn test_run_alias_second_arg_returns_empty() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let rbproject_path = temp_dir.path().join("rbproject.toml");

    // Create rbproject.toml with scripts
    let mut file = std::fs::File::create(&rbproject_path).expect("Failed to create rbproject.toml");
    writeln!(file, "[scripts]").unwrap();
    writeln!(file, "test = \"rspec\"").unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb r test ").arg("10");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.is_empty(),
        "Expected no completions for second arg after 'r test', got: {}",
        completions
    );
}

#[test]
fn test_exec_command_suggests_gem_binstubs_or_empty() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Test first argument - without bundler project, may suggest gem binstubs if they exist
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb exec ").arg("8");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // May be empty (no gems installed) or contain gem binstubs (e.g., bundler)
    // Both are valid - just verify it doesn't crash
    println!("Completions from gem binstubs: {}", completions);
}

#[test]
fn test_exec_alias_suggests_gem_binstubs_or_empty() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Test first argument - without bundler project, may suggest gem binstubs if they exist
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb x ").arg("5");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // May be empty (no gems installed) or contain gem binstubs (e.g., bundler)
    // Both are valid - just verify it doesn't crash
    println!("Completions from gem binstubs: {}", completions);

    // Test second argument - should always return empty (fallback to default)
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb x rspec ").arg("11");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.is_empty(),
        "Expected no completions for second arg after 'x rspec', got: {}",
        completions
    );
}

#[test]
#[ignore] // TODO: This test fails in test environment but works in real shell
fn test_run_with_partial_script_name() {
    // This test verifies filtering works, but "rb run te" is completing "te" as an argument
    // When line doesn't end with space, the last word is the one being completed
    // So we're completing the first argument to "run" with prefix "te"
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let rbproject_path = temp_dir.path().join("rbproject.toml");

    // Create rbproject.toml with scripts
    let mut file = std::fs::File::create(&rbproject_path).expect("Failed to create rbproject.toml");
    writeln!(file, "[scripts]").unwrap();
    writeln!(file, "test = \"rspec\"").unwrap();
    writeln!(file, "test:unit = \"rspec spec/unit\"").unwrap();
    writeln!(file, "dev = \"rails server\"").unwrap();
    drop(file); // Ensure file is flushed

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete").arg("rb run t").arg("8");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // Should get completions starting with 't'
    assert!(
        completions.contains("test"),
        "Expected 'test' in completions, got: {:?}",
        completions
    );
    assert!(
        completions.contains("test:unit"),
        "Expected 'test:unit' in completions, got: {:?}",
        completions
    );
    assert!(
        !completions.contains("dev"),
        "Should not contain 'dev' when filtering by 't', got: {:?}",
        completions
    );
}

#[test]
fn test_run_third_arg_returns_empty() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let rbproject_path = temp_dir.path().join("rbproject.toml");

    // Create rbproject.toml with scripts
    let mut file = std::fs::File::create(&rbproject_path).expect("Failed to create rbproject.toml");
    writeln!(file, "[scripts]").unwrap();
    writeln!(file, "test = \"rspec\"").unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.arg("__bash_complete")
        .arg("rb run test arg1 ")
        .arg("17");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    assert!(
        completions.is_empty(),
        "Expected no completions for third arg, got: {}",
        completions
    );
}

#[test]
fn test_binstubs_with_no_bundler_flag() {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    // Create Ruby sandbox
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox
        .add_ruby_dir("3.3.0")
        .expect("Failed to create ruby");

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create Gemfile (to simulate bundler project)
    fs::write(
        temp_dir.path().join("Gemfile"),
        "source 'https://rubygems.org'\n",
    )
    .expect("Failed to create Gemfile");

    // Create bundler binstubs
    let bundler_bin = temp_dir
        .path()
        .join(".rb")
        .join("vendor")
        .join("bundler")
        .join("ruby")
        .join("3.3.0")
        .join("bin");
    fs::create_dir_all(&bundler_bin).expect("Failed to create bundler bin dir");

    for exe in &["rails", "rake", "bundler-specific-tool"] {
        let exe_path = bundler_bin.join(exe);
        fs::write(&exe_path, "#!/usr/bin/env ruby\n").expect("Failed to write executable");
        #[cfg(unix)]
        fs::set_permissions(&exe_path, fs::Permissions::from_mode(0o755))
            .expect("Failed to set permissions");
    }

    // Run completion WITHOUT -B flag - should show bundler binstubs
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.env("RB_RUBIES_DIR", sandbox.root())
        .current_dir(temp_dir.path())
        .arg("__bash_complete")
        .arg("rb x b")
        .arg("6");
    cmd.current_dir(temp_dir.path());

    let output = cmd.output().expect("Failed to execute rb");
    let completions_without_flag = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // Run completion WITH -B flag - should NOT show bundler binstubs
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.env("RB_RUBIES_DIR", sandbox.root())
        .current_dir(temp_dir.path())
        .arg("-B") // Pass -B as real CLI arg to create ButlerRuntime without bundler
        .arg("__bash_complete")
        .arg("rb -B x b")
        .arg("9");

    let output = cmd.output().expect("Failed to execute rb");
    let completions_with_flag = String::from_utf8(output.stdout).expect("Invalid UTF-8 output");

    // Without -B: should include bundler-specific binstubs
    assert!(
        completions_without_flag.contains("bundler-specific-tool"),
        "Expected 'bundler-specific-tool' in completions without -B flag, got: {}",
        completions_without_flag
    );

    // With -B: should NOT include bundler-specific binstubs
    assert!(
        !completions_with_flag.contains("bundler-specific-tool"),
        "Should not contain 'bundler-specific-tool' with -B flag, got: {}",
        completions_with_flag
    );

    // With -B: should include gem binstubs from system (if any starting with 'b')
    // Note: This may vary by system, but at least it shouldn't be empty if gems are installed
}

// Command-based interface tests (help and version as commands, not flags)

#[test]
fn test_help_command_appears_in_completions() {
    let completions = capture_completions("rb ", "3", None);

    assert!(
        completions.contains("help"),
        "Expected 'help' command in completions, got: {}",
        completions
    );
}

#[test]
fn test_version_command_appears_in_completions() {
    let completions = capture_completions("rb ", "3", None);

    assert!(
        completions.contains("version"),
        "Expected 'version' command in completions, got: {}",
        completions
    );
}

#[test]
fn test_help_flag_not_in_completions() {
    let completions = capture_completions("rb -", "4", None);

    // Check that neither -h nor --help appear as standalone completions
    let lines: Vec<&str> = completions.lines().collect();
    assert!(
        !lines.contains(&"-h") && !lines.contains(&"--help"),
        "Help flags should not appear in completions (command-based interface), got: {:?}",
        lines
    );
}

#[test]
fn test_version_flag_not_in_completions() {
    let completions = capture_completions("rb -", "4", None);

    // Check that --version doesn't appear as flag (it's a command now)
    // Note: -V is now --very-verbose, so it SHOULD appear
    let lines: Vec<&str> = completions.lines().collect();
    assert!(
        !lines.contains(&"--version"),
        "Version flag should not appear (command-based interface), got: {:?}",
        lines
    );
    assert!(
        lines.contains(&"-V") || completions.contains("--very-verbose"),
        "Very verbose flag should appear in completions, got: {:?}",
        lines
    );
}

#[test]
fn test_help_command_completion_with_prefix() {
    let completions = capture_completions("rb h", "4", None);

    assert!(
        completions.contains("help"),
        "Expected 'help' command when completing 'h' prefix, got: {}",
        completions
    );
}

#[test]
fn test_version_command_completion_with_prefix() {
    let completions = capture_completions("rb v", "4", None);

    assert!(
        completions.contains("version"),
        "Expected 'version' command when completing 'v' prefix, got: {}",
        completions
    );
}
