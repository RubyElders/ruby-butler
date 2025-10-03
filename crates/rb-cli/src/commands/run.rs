use colored::*;
use log::{debug, info, warn};
use rb_core::butler::ButlerRuntime;
use rb_core::project::{ProjectRuntime, RbprojectDetector};
use std::path::PathBuf;

use super::exec::exec_command;

fn list_available_scripts(butler_runtime: ButlerRuntime, project_file: Option<PathBuf>) {
    info!("Listing available project scripts");

    // Detect or load project runtime
    let current_dir = butler_runtime.current_dir();
    let project_runtime = if let Some(path) = project_file {
        // Use specified project file
        debug!(
            "Loading rbproject.toml from specified path: {}",
            path.display()
        );
        match ProjectRuntime::from_file(&path) {
            Ok(project) => Some(project),
            Err(e) => {
                eprintln!("{}", "❌ Selection Failed".red().bold());
                eprintln!();
                eprintln!("The specified project configuration could not be loaded:");
                eprintln!("  File: {}", path.display().to_string().bright_black());
                eprintln!("  Error: {}", e.to_string().bright_black());
                eprintln!();
                eprintln!("Please verify the file exists and contains valid TOML configuration.");
                std::process::exit(1);
            }
        }
    } else {
        // Auto-detect project file
        match RbprojectDetector::discover(current_dir) {
            Ok(Some(project)) => {
                debug!(
                    "Discovered rbproject.toml with {} scripts",
                    project.scripts.len()
                );
                Some(project)
            }
            Ok(None) => None,
            Err(e) => {
                warn!("Error detecting rbproject.toml: {}", e);
                None
            }
        }
    };

    // Ensure we have a project configuration
    let project = match project_runtime {
        Some(p) => p,
        None => {
            eprintln!("{}", "❌ No Project Configuration".red().bold());
            eprintln!();
            eprintln!("No project configuration detected in the current directory hierarchy.");
            eprintln!();
            eprintln!(
                "To define project scripts, create an {} file:",
                "rbproject.toml".cyan()
            );
            eprintln!();
            eprintln!("  {}", "[scripts]".bright_black());
            eprintln!("  {} = {}", "test".cyan(), "\"rspec\"".bright_black());
            eprintln!(
                "  {} = {{ command = {}, description = {} }}",
                "lint".cyan(),
                "\"rubocop\"".bright_black(),
                "\"Check code quality\"".bright_black()
            );
            eprintln!();
            eprintln!(
                "Or specify a custom location: {} -P path/to/rbproject.toml run",
                "rb".green().bold()
            );
            std::process::exit(1);
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
            "rbproject.toml".cyan()
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
}

pub fn run_command(
    butler_runtime: ButlerRuntime,
    script_name: Option<String>,
    args: Vec<String>,
    project_file: Option<PathBuf>,
) {
    // If no script name provided, list available scripts
    if script_name.is_none() {
        list_available_scripts(butler_runtime, project_file);
        return;
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
            "Loading rbproject.toml from specified path: {}",
            path.display()
        );
        match ProjectRuntime::from_file(&path) {
            Ok(project) => Some(project),
            Err(e) => {
                eprintln!("{}", "❌ Selection Failed".red().bold());
                eprintln!();
                eprintln!("The specified project configuration could not be loaded:");
                eprintln!("  File: {}", path.display().to_string().bright_black());
                eprintln!("  Error: {}", e.to_string().bright_black());
                eprintln!();
                eprintln!("Please verify the file exists and contains valid TOML configuration.");
                std::process::exit(1);
            }
        }
    } else {
        // Auto-detect project file
        match RbprojectDetector::discover(current_dir) {
            Ok(Some(project)) => {
                debug!(
                    "Discovered rbproject.toml with {} scripts",
                    project.scripts.len()
                );
                Some(project)
            }
            Ok(None) => None,
            Err(e) => {
                warn!("Error detecting rbproject.toml: {}", e);
                None
            }
        }
    };

    // Ensure we have a project configuration
    let project = match project_runtime {
        Some(p) => p,
        None => {
            eprintln!("{}", "❌ Selection Failed".red().bold());
            eprintln!();
            eprintln!("No project configuration detected in the current directory hierarchy.");
            eprintln!();
            eprintln!(
                "To use project scripts, please create an {} file with script definitions:",
                "rbproject.toml".cyan()
            );
            eprintln!();
            eprintln!("  {}", "[scripts]".bright_black());
            eprintln!("  {} = {}", "test".cyan(), "\"rspec\"".bright_black());
            eprintln!(
                "  {} = {{ command = {}, description = {} }}",
                "lint".cyan(),
                "\"rubocop\"".bright_black(),
                "\"Check code quality\"".bright_black()
            );
            eprintln!();
            eprintln!(
                "Or specify a custom location with: {} -P path/to/rbproject.toml run {}",
                "rb".green().bold(),
                script_name.cyan()
            );
            std::process::exit(1);
        }
    };

    // Look up the script
    if !project.has_script(&script_name) {
        eprintln!("{}", "❌ Script Not Found".red().bold());
        eprintln!();
        eprintln!(
            "The script '{}' is not defined in your project configuration.",
            script_name.cyan().bold()
        );
        eprintln!();

        let available_scripts = project.script_names();
        if available_scripts.is_empty() {
            eprintln!(
                "No scripts are currently defined in {}.",
                project
                    .rbproject_path()
                    .display()
                    .to_string()
                    .bright_black()
            );
        } else {
            eprintln!(
                "Available scripts from {}:",
                project
                    .rbproject_path()
                    .display()
                    .to_string()
                    .bright_black()
            );
            eprintln!();
            for name in available_scripts {
                let script = project.get_script(name).unwrap();
                let command = script.command();
                eprintln!(
                    "  {} {} {}",
                    name.cyan().bold(),
                    "→".bright_black(),
                    command.bright_black()
                );
                if let Some(description) = script.description() {
                    eprintln!("    {}", description.bright_black().italic());
                }
            }
        }
        eprintln!();
        eprintln!(
            "Run {} to see all available scripts.",
            "rb env".green().bold()
        );
        std::process::exit(1);
    }

    // Get the script command
    let command_str = project.get_script_command(&script_name).unwrap();

    info!("Executing script: {} → {}", script_name, command_str);

    // Parse the command string into program and arguments using shell word splitting
    let command_parts = parse_command(command_str);

    if command_parts.is_empty() {
        eprintln!("{}", "❌ Invalid Script".red().bold());
        eprintln!();
        eprintln!(
            "The script '{}' has an empty command.",
            script_name.cyan().bold()
        );
        std::process::exit(1);
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
    exec_command(butler_runtime, full_args);
}

/// Parse a command string into program and arguments
/// This is a simple whitespace-based parser that respects quotes
fn parse_command(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in command.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' if !in_quotes => {
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
    fn test_parse_command_empty() {
        assert_eq!(parse_command(""), Vec::<String>::new());
    }

    #[test]
    fn test_parse_command_only_spaces() {
        assert_eq!(parse_command("   "), Vec::<String>::new());
    }
}
