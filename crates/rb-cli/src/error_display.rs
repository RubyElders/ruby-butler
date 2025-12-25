use colored::Colorize;
use rb_core::butler::ButlerError;
use std::path::Path;

/// Format error message for NoSuitableRuby error
pub fn format_no_suitable_ruby(
    rubies_dir: &Path,
    source: String,
    requested_version: Option<(String, String)>,
) -> String {
    let mut msg = String::new();

    msg.push_str("The designated Ruby estate directory appears to be absent from your system.\n");
    msg.push('\n');
    msg.push_str("Searched in:\n");
    msg.push_str(&format!("  • {} (from {})\n", rubies_dir.display(), source));

    if let Some((version, version_source)) = requested_version {
        msg.push('\n');
        msg.push_str(&format!(
            "Requested version: {} (from {})\n",
            version, version_source
        ));
    }

    msg.push('\n');
    msg.push_str(
        "May I suggest installing Ruby using ruby-install or a similar distinguished tool?",
    );

    msg
}

/// Format error message for CommandNotFound error
pub fn format_command_not_found(command: &str) -> String {
    format!(
        "🎩 My sincerest apologies, but the command '{}' appears to be
   entirely absent from your distinguished Ruby environment.

This humble Butler has meticulously searched through all
available paths and gem installations, yet the requested
command remains elusive.

Might I suggest:
  • Verifying the command name is spelled correctly
  • Installing the appropriate gem: {}
  • Checking if bundler management is required: {}",
        command.bright_yellow(),
        format!("gem install {}", command).cyan(),
        "bundle install".cyan()
    )
}

/// Format error message for RubiesDirectoryNotFound error
pub fn format_rubies_dir_not_found(path: &Path) -> String {
    format!(
        "Ruby installation directory not found: {}

Please verify the path exists or specify a different location
using the -R flag or RB_RUBIES_DIR environment variable.",
        path.display()
    )
}

/// Format general error message
pub fn format_general_error(msg: &str) -> String {
    format!("❌ {}", msg)
}

/// Get exit code for specific error type
pub fn error_exit_code(error: &ButlerError) -> i32 {
    match error {
        ButlerError::CommandNotFound(_) => 127,
        _ => 1,
    }
}
