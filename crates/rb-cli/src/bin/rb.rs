use clap::Parser;
use colored::Colorize;
use rb_cli::config::TrackedConfig;
use rb_cli::{
    Cli, Commands, Shell, config_command, environment_command, exec_command, init_command,
    init_logger, run_command, runtime_command, shell_integration_command, sync_command,
};
use rb_core::butler::{ButlerError, ButlerRuntime};
use std::path::PathBuf;

/// Context information for command execution and error handling
struct CommandContext {
    config: TrackedConfig,
    project_file: Option<PathBuf>,
}

/// Centralized error handler that transforms technical errors into friendly messages
fn handle_command_error(error: ButlerError, context: &CommandContext) -> ! {
    match error {
        ButlerError::NoSuitableRuby(_) => {
            let rubies_dir = context.config.rubies_dir.get();
            eprintln!(
                "The designated Ruby estate directory appears to be absent from your system."
            );
            eprintln!();
            eprintln!("Searched in:");
            eprintln!(
                "  • {} (from {})",
                rubies_dir.display(),
                context.config.rubies_dir.source
            );

            if let Some(ref requested_version) = context.config.ruby_version {
                eprintln!();
                eprintln!(
                    "Requested version: {} (from {})",
                    requested_version.get(),
                    requested_version.source
                );
            }

            eprintln!();
            eprintln!(
                "May I suggest installing Ruby using ruby-install or a similar distinguished tool?"
            );
            std::process::exit(1);
        }
        ButlerError::CommandNotFound(command) => {
            eprintln!(
                "🎩 My sincerest apologies, but the command '{}' appears to be",
                command.bright_yellow()
            );
            eprintln!("   entirely absent from your distinguished Ruby environment.");
            eprintln!();
            eprintln!("This humble Butler has meticulously searched through all");
            eprintln!("available paths and gem installations, yet the requested");
            eprintln!("command remains elusive.");
            eprintln!();
            eprintln!("Might I suggest:");
            eprintln!("  • Verifying the command name is spelled correctly");
            eprintln!(
                "  • Installing the appropriate gem: {}",
                format!("gem install {}", command).cyan()
            );
            eprintln!(
                "  • Checking if bundler management is required: {}",
                "bundle install".cyan()
            );
            std::process::exit(127);
        }
        ButlerError::RubiesDirectoryNotFound(path) => {
            eprintln!("Ruby installation directory not found: {}", path.display());
            eprintln!();
            eprintln!("Please verify the path exists or specify a different location");
            eprintln!("using the -R flag or RB_RUBIES_DIR environment variable.");
            std::process::exit(1);
        }
        ButlerError::General(msg) => {
            eprintln!("❌ {}", msg);
            std::process::exit(1);
        }
    }
}

/// Create ButlerRuntime lazily and execute command with it
/// Also updates the context with resolved values (e.g., which Ruby was actually selected)
fn with_butler_runtime<F>(context: &mut CommandContext, f: F) -> Result<(), ButlerError>
where
    F: FnOnce(&ButlerRuntime) -> Result<(), ButlerError>,
{
    let rubies_dir = context.config.rubies_dir.get().clone();

    // Use runtime-compatible version (filters out unresolved values)
    let requested_version = context.config.ruby_version_for_runtime();

    let butler_runtime = ButlerRuntime::discover_and_compose_with_gem_base(
        rubies_dir,
        requested_version,
        Some(context.config.gem_home.get().clone()),
        *context.config.no_bundler.get(),
    )?;

    // Update context with resolved Ruby version if it was unresolved
    if context.config.has_unresolved()
        && let Ok(ruby_runtime) = butler_runtime.selected_ruby()
    {
        let resolved_version = ruby_runtime.version.to_string();
        context.config.resolve_ruby_version(resolved_version);
    }

    f(&butler_runtime)
}

/// Version command - no runtime needed
fn version_command() -> Result<(), ButlerError> {
    println!("{}", build_version_info());
    Ok(())
}

/// Help command - no runtime needed
fn help_command(subcommand: Option<String>) -> Result<(), ButlerError> {
    use clap::CommandFactory;
    let mut cmd = Cli::command();

    if let Some(subcommand_name) = subcommand {
        // Show help for specific subcommand
        if let Some(subcommand) = cmd.find_subcommand_mut(&subcommand_name) {
            let _ = subcommand.print_help();
        } else {
            eprintln!("Unknown command: {}", subcommand_name);
            eprintln!("Run 'rb help' to see available commands");
            std::process::exit(1);
        }
    } else {
        // Show custom grouped help
        print_custom_help(&cmd);
        return Ok(());
    }
    println!();
    Ok(())
}

/// Print custom help with command grouping
fn print_custom_help(cmd: &clap::Command) {
    // Print header
    if let Some(about) = cmd.get_about() {
        println!("{}", about);
    }
    println!();

    // Print usage
    let bin_name = cmd.get_name();
    println!(
        "{} {} {} {} {}",
        "Usage:".green().bold(),
        bin_name.cyan().bold(),
        "[OPTIONS]".cyan(),
        "COMMAND".cyan().bold(),
        "[COMMAND_OPTIONS]".cyan()
    );
    println!();

    // Group commands
    let runtime_commands = ["runtime", "environment", "exec", "sync", "run"];
    let utility_commands = ["init", "config", "version", "help", "shell-integration"];

    // Print runtime commands
    println!("{}", "Commands:".green().bold());
    for subcmd in cmd.get_subcommands() {
        let name = subcmd.get_name();
        if runtime_commands.contains(&name) {
            print_command_line(subcmd);
        }
    }
    println!();

    // Print utility commands
    println!("{}", "Utility Commands:".green().bold());
    for subcmd in cmd.get_subcommands() {
        let name = subcmd.get_name();
        if utility_commands.contains(&name) {
            print_command_line(subcmd);
        }
    }
    println!();

    // Print options
    println!("{}", "Options:".green().bold());
    for arg in cmd.get_arguments() {
        if arg.get_id() == "help" || arg.get_id() == "version" {
            continue;
        }
        print_argument_line(arg);
    }
}

/// Helper to print a command line
fn print_command_line(subcmd: &clap::Command) {
    let name = subcmd.get_name();
    let about = subcmd
        .get_about()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let aliases: Vec<_> = subcmd.get_all_aliases().collect();

    if aliases.is_empty() {
        println!("  {:18} {}", name.cyan().bold(), about);
    } else {
        let alias_str = format!("[aliases: {}]", aliases.join(", "));
        println!("  {:18} {} {}", name.cyan().bold(), about, alias_str.cyan());
    }
}

/// Helper to print an argument line
fn print_argument_line(arg: &clap::Arg) {
    let short = arg
        .get_short()
        .map(|c| format!("-{}", c))
        .unwrap_or_default();
    let long = arg
        .get_long()
        .map(|s| format!("--{}", s))
        .unwrap_or_default();

    let flag = if !short.is_empty() && !long.is_empty() {
        format!("{}, {}", short, long)
    } else if !short.is_empty() {
        short
    } else {
        long
    };

    // Only show value placeholder if it actually takes values (not boolean flags)
    let value_name = if arg.get_num_args().unwrap_or_default().takes_values()
        && arg.get_action().takes_values()
    {
        format!(
            " <{}>",
            arg.get_id().as_str().to_uppercase().replace('_', "-")
        )
    } else {
        String::new()
    };

    let help = arg.get_help().map(|s| s.to_string()).unwrap_or_default();

    // Show env var if available
    let env_var = if let Some(env) = arg.get_env() {
        format!(" [env: {}]", env.to_string_lossy())
    } else {
        String::new()
    };

    // Calculate visual width for alignment (without ANSI codes)
    let visual_width = flag.len() + value_name.len();
    let padding = if visual_width < 31 {
        31 - visual_width
    } else {
        1
    };

    // Color the flag and value name, but keep help text uncolored
    let colored_flag = flag.cyan().bold();
    let colored_value = if !value_name.is_empty() {
        value_name.cyan().to_string()
    } else {
        String::new()
    };
    let colored_env = if !env_var.is_empty() {
        format!(" {}", env_var.cyan())
    } else {
        String::new()
    };

    println!(
        "  {}{}{}{}{}",
        colored_flag,
        colored_value,
        " ".repeat(padding),
        help,
        colored_env
    );
}

/// Init command wrapper - no runtime needed
fn init_command_wrapper() -> Result<(), ButlerError> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    init_command(&current_dir).map_err(ButlerError::General)
}

/// Shell integration command wrapper - no runtime needed
fn shell_integration_command_wrapper(shell: Option<Shell>) -> Result<(), ButlerError> {
    match shell {
        Some(s) => shell_integration_command(s).map_err(|e| ButlerError::General(e.to_string())),
        None => {
            rb_cli::commands::shell_integration::show_available_integrations();
            Ok(())
        }
    }
}

/// Bash completion command - tries to create runtime but gracefully handles failure
fn bash_complete_command(
    context: &CommandContext,
    line: &str,
    point: &str,
) -> Result<(), ButlerError> {
    let rubies_dir = context.config.rubies_dir.get().clone();

    // Try to create runtime, but if it fails, continue with None
    // Completion still works for commands/flags even without Ruby
    let butler_runtime = ButlerRuntime::discover_and_compose_with_gem_base(
        rubies_dir,
        context
            .config
            .ruby_version
            .as_ref()
            .map(|v| v.get().clone()),
        Some(context.config.gem_home.get().clone()),
        *context.config.no_bundler.get(),
    )
    .ok();

    rb_cli::completion::generate_completions(line, point, butler_runtime.as_ref());
    Ok(())
}

fn build_version_info() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = option_env!("GIT_HASH").unwrap_or("unknown");
    let profile = option_env!("BUILD_PROFILE").unwrap_or("unknown");

    let mut parts = vec![format!("Ruby Butler v{}", version)];

    // Add tag if available, otherwise add git hash
    if let Some(tag) = option_env!("GIT_TAG") {
        if !tag.is_empty() && tag != format!("v{}", version) {
            parts.push(format!("({})", tag));
        }
    } else if git_hash != "unknown" {
        parts.push(format!("({})", git_hash));
    }

    // Add profile if debug
    if profile == "debug" {
        parts.push("[debug build]".to_string());
    }

    // Add dirty flag if present
    if option_env!("GIT_DIRTY").is_some() {
        parts.push("[modified]".to_string());
    }

    parts.push(
        "\n\nA sophisticated Ruby environment manager with the refined precision".to_string(),
    );
    parts.push("of a proper gentleman's gentleman.\n".to_string());
    parts.push("At your distinguished service, RubyElders.com".to_string());

    parts.join(" ")
}

fn main() {
    let cli = Cli::parse();

    // Skip logging for bash completion (must be silent)
    if !matches!(cli.command, Some(Commands::BashComplete { .. })) {
        init_logger(cli.effective_log_level());
    }

    // Merge config file defaults with CLI arguments (just data, no side effects)
    let (cli_parsed, file_config) = match cli.with_config_defaults_tracked() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    let Some(command) = cli_parsed.command else {
        use clap::CommandFactory;
        let cmd = Cli::command();
        print_custom_help(&cmd);
        std::process::exit(0);
    };

    // Create tracked config with sources
    let tracked_config = TrackedConfig::from_merged(&cli_parsed.config, &file_config);

    // Change working directory if specified
    if !tracked_config.work_dir.source.is_default() {
        let target_dir = tracked_config.work_dir.get();
        if let Err(e) = std::env::set_current_dir(target_dir) {
            eprintln!(
                "Failed to change to directory '{}': {}",
                target_dir.display(),
                e
            );
            std::process::exit(1);
        }
        use log::debug;
        debug!("Changed working directory to: {}", target_dir.display());
    }

    // Create command context (just config data, no runtime discovery yet)
    let mut context = CommandContext {
        config: tracked_config,
        project_file: cli_parsed.project_file.clone(),
    };

    // Dispatch to commands - each creates ButlerRuntime if needed
    let result = match command {
        Commands::Version => version_command(),
        Commands::Help { command: help_cmd } => help_command(help_cmd),
        Commands::Init => init_command_wrapper(),
        Commands::Config => config_command(&context.config),
        Commands::ShellIntegration { shell } => shell_integration_command_wrapper(shell),
        Commands::BashComplete { line, point } => bash_complete_command(&context, &line, &point),
        // These need ButlerRuntime - create it lazily and may update context
        Commands::Runtime => with_butler_runtime(&mut context, runtime_command),
        Commands::Environment => {
            let project_file = context.project_file.clone();
            with_butler_runtime(&mut context, |runtime| {
                environment_command(runtime, project_file)
            })
        }
        Commands::Exec { args } => {
            with_butler_runtime(&mut context, |runtime| exec_command(runtime.clone(), args))
        }
        Commands::Run { script, args } => {
            let project_file = context.project_file.clone();
            with_butler_runtime(&mut context, |runtime| {
                run_command(runtime.clone(), script, args, project_file)
            })
        }
        Commands::Sync => {
            with_butler_runtime(&mut context, |runtime| sync_command(runtime.clone()))
        }
    };

    // Handle any errors with consistent, friendly messages
    if let Err(e) = result {
        handle_command_error(e, &context);
    }
}
