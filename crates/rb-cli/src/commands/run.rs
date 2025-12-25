use colored::*;
use log::{debug, info, warn};
use rb_core::butler::{ButlerError, ButlerRuntime};
use rb_core::project::{ProjectRuntime, RbprojectDetector};
use std::path::PathBuf;

use super::exec::exec_command;

fn list_available_scripts(
    butler_runtime: ButlerRuntime,
    project_file: Option<PathBuf>,
) -> Result<(), ButlerError> {
    info!("Listing available project scripts");

    // Detect or load project runtime
    let current_dir = butler_runtime.current_dir();
    let project_runtime = if let Some(path) = project_file {
        // Use specified project file
        debug!(
            "Loading project config from specified path: {}",
            path.display()
        );
        match ProjectRuntime::from_file(&path) {
            Ok(project) => Some(project),
            Err(e) => {
                return Err(ButlerError::General(format!(
                    "The specified project configuration could not be loaded from {}:\n{}",
                    path.display(),
                    e
                )));
            }
        }
    } else {
        // Auto-detect project file
        match RbprojectDetector::discover(current_dir) {
            Ok(Some(project)) => {
                debug!(
                    "Discovered {} with {} scripts",
                    project.config_filename,
                    project.scripts.len()
                );
                Some(project)
            }
            Ok(None) => None,
            Err(e) => {
                warn!("Error detecting project config: {}", e);
                None
            }
        }
    };

    // Ensure we have a project configuration
    let project = match project_runtime {
        Some(p) => p,
        None => {
            return Err(ButlerError::General(
                "No project configuration detected in the current directory hierarchy.\n\nTo define project scripts, create one of these files (in priority order):\n  gem.kdl, gem.toml, rbproject.kdl, rbproject.toml\n\nOr specify a custom location: rb -P path/to/gem.kdl run".to_string()
            ));
        }
    };

    // Display help-style output
    println!("{}", "🎯 Run Project Scripts".green().bold());
    println!();

    // Show project metadata if available
    if let Some(name) = &project.metadata.name {
        println!("{}", name);
    }

    if let Some(description) = &project.metadata.description {
        println!("{}", description.bright_black());
    }

    if project.metadata.name.is_some() || project.metadata.description.is_some() {
        println!();
    }

    // Usage section
    println!("{}", "Usage:".green().bold());
    println!("  rb run <SCRIPT> [ARGS]...");
    println!();

    let available_scripts = project.script_names();
    let script_count = available_scripts.len();

    if available_scripts.is_empty() {
        println!("{}", "Scripts:".green().bold());
        println!("  {}", "No scripts defined.".bright_black());
        println!();
        println!(
            "To define scripts, add them to {}:",
            project.config_filename.cyan()
        );
        println!();
        println!("  {}", "[scripts]".bright_black());
        println!("  {} = {}", "test".cyan(), "\"rspec\"".bright_black());
        println!(
            "  {} = {{ command = {}, description = {} }}",
            "lint".cyan(),
            "\"rubocop\"".bright_black(),
            "\"Check code quality\"".bright_black()
        );
    } else {
        // Scripts section - formatted like Clap's Commands section
        println!("{}", "Scripts:".green().bold());

        // Calculate max width for alignment
        let max_name_width = available_scripts.iter().map(|s| s.len()).max().unwrap_or(0);

        for name in available_scripts {
            let script = project.get_script(name).unwrap();
            let command = script.command();

            if let Some(description) = script.description() {
                // Show: name  description
                println!(
                    "  {:<width$}  {}",
                    name.cyan().bold(),
                    description.bright_black(),
                    width = max_name_width
                );
            } else {
                // Show: name  command
                println!(
                    "  {:<width$}  {}",
                    name.cyan().bold(),
                    command.bright_black(),
                    width = max_name_width
                );
            }
        }

        println!();
        println!("{}", "Details:".green().bold());
        println!(
            "  {}: {}",
            "Project".bright_black(),
            project
                .rbproject_path()
                .display()
                .to_string()
                .bright_black()
        );
        println!(
            "  {}: {}",
            "Scripts".bright_black(),
            script_count.to_string().bright_black()
        );
    }

    Ok(())
}

pub fn run_command(
    butler_runtime: ButlerRuntime,
    script_name: Option<String>,
    args: Vec<String>,
    project_file: Option<PathBuf>,
) -> Result<(), ButlerError> {
    // If no script name provided, list available scripts
    if script_name.is_none() {
        return list_available_scripts(butler_runtime, project_file);
    }

    let script_name = script_name.unwrap();
    info!(
        "Executing project script '{}' with distinguished precision",
        script_name
    );

    // Detect or load project runtime
    let current_dir = butler_runtime.current_dir();
    let project_runtime = if let Some(path) = project_file {
        // Use specified project file
        debug!(
            "Loading project config from specified path: {}",
            path.display()
        );
        match ProjectRuntime::from_file(&path) {
            Ok(project) => Some(project),
            Err(e) => {
                return Err(ButlerError::General(format!(
                    "The specified project configuration could not be loaded from {}:\n{}",
                    path.display(),
                    e
                )));
            }
        }
    } else {
        // Auto-detect project file
        match RbprojectDetector::discover(current_dir) {
            Ok(Some(project)) => {
                debug!(
                    "Discovered {} with {} scripts",
                    project.config_filename,
                    project.scripts.len()
                );
                Some(project)
            }
            Ok(None) => None,
            Err(e) => {
                warn!("Error detecting project config: {}", e);
                None
            }
        }
    };

    // Ensure we have a project configuration
    let project = match project_runtime {
        Some(p) => p,
        None => {
            return Err(ButlerError::General(format!(
                "No project configuration detected in the current directory hierarchy.\n\nTo use project scripts, create one of these files: rbproject.toml, rb.toml, rb.kdl, gem.toml, gem.kdl\n\nOr specify a custom location with: rb -P path/to/rb.toml run {}",
                script_name
            )));
        }
    };

    // Look up the script
    if !project.has_script(&script_name) {
        return Err(ButlerError::General(format!(
            "The script '{}' is not defined in your project configuration",
            script_name
        )));
    }

    // Get the script command
    let command_str = project.get_script_command(&script_name).unwrap();

    info!("Executing script: {} → {}", script_name, command_str);

    // Parse the command string into program and arguments using shell word splitting
    let command_parts = parse_command(command_str);

    if command_parts.is_empty() {
        return Err(ButlerError::General(format!(
            "The script '{}' has an empty command",
            script_name
        )));
    }

    // Build the full argument list: parsed command parts + user-provided args
    let mut full_args = command_parts;
    full_args.extend(args);

    info!("Delegating to exec command with args: {:?}", full_args);

    // Delegate to exec_command - this ensures consistent behavior including:
    // - Automatic bundle exec detection
    // - Bundler environment synchronization
    // - Proper environment composition
    // - Command validation and error handling
    exec_command(butler_runtime, full_args)
}

/// Parse a command string into program and arguments
/// This is a simple whitespace-based parser that respects quotes
fn parse_command(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_double_quotes = false;
    let mut in_single_quotes = false;

    for ch in command.chars() {
        match ch {
            '"' if !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
            }
            '\'' if !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
            }
            ' ' if !in_double_quotes && !in_single_quotes => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_simple() {
        assert_eq!(
            parse_command("ruby -v"),
            vec!["ruby".to_string(), "-v".to_string()]
        );
    }

    #[test]
    fn test_parse_command_with_multiple_args() {
        assert_eq!(
            parse_command("gem install bundler --version 2.4.0"),
            vec![
                "gem".to_string(),
                "install".to_string(),
                "bundler".to_string(),
                "--version".to_string(),
                "2.4.0".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_command_with_quotes() {
        assert_eq!(
            parse_command("rails new \"my app\""),
            vec!["rails".to_string(), "new".to_string(), "my app".to_string()]
        );
    }

    #[test]
    fn test_parse_command_with_extra_spaces() {
        assert_eq!(
            parse_command("ruby  -e   \"puts 'hello'\""),
            vec![
                "ruby".to_string(),
                "-e".to_string(),
                "puts 'hello'".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_command_with_single_quotes() {
        assert_eq!(
            parse_command("ruby -e 'puts ARGV.join(\", \")'"),
            vec![
                "ruby".to_string(),
                "-e".to_string(),
                "puts ARGV.join(\", \")".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_command_empty() {
        assert_eq!(parse_command(""), Vec::<String>::new());
    }

    #[test]
    fn test_parse_command_only_spaces() {
        assert_eq!(parse_command("   "), Vec::<String>::new());
    }
}
