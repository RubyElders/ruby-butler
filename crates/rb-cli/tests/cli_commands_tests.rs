use std::process::Command;

/// Helper to execute rb binary with arguments
fn run_rb_command(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rb"));
    cmd.args(args);
    cmd.output().expect("Failed to execute rb")
}

/// Helper to convert output to string
fn output_to_string(output: &[u8]) -> String {
    String::from_utf8_lossy(output).to_string()
}

// Help command tests

#[test]
fn test_help_command_works() {
    let output = run_rb_command(&["help"]);
    let stdout = output_to_string(&output.stdout);

    assert!(output.status.success(), "help command should succeed");
    assert!(stdout.contains("Usage"), "Should show usage");
    assert!(stdout.contains("Commands"), "Should show commands");
    assert!(stdout.contains("Options"), "Should show options");
}

#[test]
fn test_help_command_shows_all_commands() {
    let output = run_rb_command(&["help"]);
    let stdout = output_to_string(&output.stdout);

    assert!(stdout.contains("runtime"), "Should list runtime command");
    assert!(
        stdout.contains("environment"),
        "Should list environment command"
    );
    assert!(stdout.contains("exec"), "Should list exec command");
    assert!(stdout.contains("sync"), "Should list sync command");
    assert!(stdout.contains("run"), "Should list run command");
    assert!(stdout.contains("init"), "Should list init command");
    assert!(stdout.contains("version"), "Should list version command");
    assert!(stdout.contains("help"), "Should list help command itself");
}

#[test]
fn test_help_for_specific_command() {
    let output = run_rb_command(&["help", "runtime"]);
    let stdout = output_to_string(&output.stdout);

    assert!(output.status.success(), "help runtime should succeed");
    assert!(
        stdout.contains("Survey your distinguished Ruby estate"),
        "Should show runtime command description"
    );
}

#[test]
fn test_help_for_nonexistent_command() {
    let output = run_rb_command(&["help", "nonexistent"]);
    let stderr = output_to_string(&output.stderr);

    assert!(
        !output.status.success(),
        "help for nonexistent command should fail"
    );
    assert!(
        stderr.contains("Unknown command"),
        "Should report unknown command"
    );
}

#[test]
fn test_help_flag_is_rejected() {
    let output = run_rb_command(&["--help"]);
    let stderr = output_to_string(&output.stderr);

    assert!(!output.status.success(), "--help flag should be rejected");
    assert!(
        stderr.contains("unexpected argument '--help'"),
        "Should report unexpected argument, got: {}",
        stderr
    );
}

#[test]
fn test_short_help_flag_is_rejected() {
    let output = run_rb_command(&["-h"]);
    let stderr = output_to_string(&output.stderr);

    assert!(!output.status.success(), "-h flag should be rejected");
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("found '-h'"),
        "Should report unexpected argument, got: {}",
        stderr
    );
}

// Version command tests

#[test]
fn test_version_command_works() {
    let output = run_rb_command(&["version"]);
    let stdout = output_to_string(&output.stdout);

    assert!(output.status.success(), "version command should succeed");
    assert!(
        stdout.contains("Ruby Butler"),
        "Should show Ruby Butler name"
    );
    assert!(
        stdout.contains("v") || stdout.contains("0."),
        "Should show version number"
    );
}

#[test]
fn test_version_command_shows_butler_identity() {
    let output = run_rb_command(&["version"]);
    let stdout = output_to_string(&output.stdout);

    assert!(
        stdout.contains("Ruby environment manager") || stdout.contains("gentleman"),
        "Should include butler identity/tagline"
    );
    assert!(stdout.contains("RubyElders"), "Should include attribution");
}

#[test]
fn test_version_flag_is_rejected() {
    let output = run_rb_command(&["--version"]);
    let stderr = output_to_string(&output.stderr);

    assert!(
        !output.status.success(),
        "--version flag should be rejected"
    );
    assert!(
        stderr.contains("unexpected argument '--version'"),
        "Should report unexpected argument, got: {}",
        stderr
    );
}

#[test]
fn test_short_version_flag_is_rejected() {
    // -V is now --very-verbose, so this test is obsolete
    // Test that -V works as very verbose flag
    let output = run_rb_command(&["-V", "help"]);
    assert!(
        output.status.success(),
        "-V flag should work as --very-verbose"
    );
}

// No arguments behavior

#[test]
fn test_no_arguments_shows_help() {
    let output = run_rb_command(&[]);
    let stdout = output_to_string(&output.stdout);

    assert!(output.status.success(), "no arguments should show help");
    assert!(stdout.contains("Usage"), "Should show usage when no args");
    assert!(
        stdout.contains("Commands"),
        "Should show commands when no args"
    );
}

// Command-based interface philosophy

#[test]
fn test_all_major_features_are_commands() {
    let output = run_rb_command(&["help"]);
    let stdout = output_to_string(&output.stdout);

    // Verify that help and version are listed as commands
    assert!(
        stdout.contains("version"),
        "version should be in help output"
    );
    assert!(stdout.contains("help"), "help should be in help output");

    // Extract options section (after both Commands sections)
    let options_section = stdout.split("Options:").nth(1).unwrap_or("");

    assert!(
        !options_section.contains("-h,") && !options_section.contains("--help"),
        "Options should not list -h or --help flags"
    );

    // Note: -V is now --very-verbose, not version flag
    assert!(
        options_section.contains("--very-verbose"),
        "Options should list --very-verbose flag"
    );
}
