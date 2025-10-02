use colored::*;
use log::{debug, info, warn};
use rb_core::butler::{ButlerRuntime, Command as ButlerCommand};
use rb_core::project::{ProjectRuntime, RbprojectDetector};
use std::path::PathBuf;

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

    // For complex command execution, use shell invocation
    // This properly handles quotes, pipes, redirects, etc.
    #[cfg(target_os = "windows")]
    let shell = "powershell.exe";

    #[cfg(not(target_os = "windows"))]
    let shell = "sh";

    // Build the full command with user arguments
    let full_command = if args.is_empty() {
        command_str.to_string()
    } else {
        format!("{} {}", command_str, args.join(" "))
    };

    debug!("Executing via shell: {} -Command '{}'", shell, full_command);

    // Create a ButlerCommand that runs the script via shell
    let mut butler_command = ButlerCommand::new(shell);

    #[cfg(target_os = "windows")]
    {
        butler_command.arg("-Command");
        butler_command.arg(&full_command);
    }

    #[cfg(not(target_os = "windows"))]
    {
        butler_command.arg("-c");
        butler_command.arg(&full_command);
    }

    // Set working directory to project root (use current_dir if root is empty)
    let work_dir = if project.root.as_os_str().is_empty() {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
        project.root.clone()
    };

    butler_command.current_dir(&work_dir);

    info!(
        "Executing command in project directory: {}",
        work_dir.display()
    );

    // Execute the command with butler context (no validation since we're using shell)
    match butler_command.status_with_context(&butler_runtime) {
        Ok(status) => {
            if let Some(code) = status.code() {
                debug!("Script concluded with exit code: {}", code);
                std::process::exit(code);
            } else {
                debug!("Script was terminated by system signal");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{}", "❌ Execution Failed".red().bold());
            eprintln!();
            eprintln!("Failed to execute script '{}':", script_name.cyan().bold());
            eprintln!("  Command: {}", full_command.bright_black());
            eprintln!("  Error: {}", e.to_string().bright_black());
            eprintln!();
            eprintln!("Please verify the command is correct and all dependencies are available.");
            std::process::exit(1);
        }
    }
}
