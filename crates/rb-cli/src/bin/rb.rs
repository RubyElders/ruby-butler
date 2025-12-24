use clap::Parser;
use rb_cli::{
    Cli, Commands, environment_command, exec_command, init_command, init_logger,
    resolve_search_dir, run_command, runtime_command, shell_integration_command, sync_command,
};
use rb_core::butler::{ButlerError, ButlerRuntime};

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
    // Handle version request with custom formatting before parsing
    // Only handle version if it's a direct flag, not part of exec command
    let args: Vec<String> = std::env::args().collect();
    let is_version_request = args.len() == 2 && (args[1] == "--version" || args[1] == "-V");

    if is_version_request {
        println!("{}", build_version_info());
        return;
    }

    let cli = Cli::parse();

    // Skip logging for bash completion (must be silent)
    if !matches!(cli.command, Some(Commands::BashComplete { .. })) {
        init_logger(cli.effective_log_level());
    }

    // Merge config file defaults with CLI arguments
    let cli = match cli.with_config_defaults() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    let Some(command) = cli.command else {
        use clap::CommandFactory;
        let mut cmd = Cli::command();
        let _ = cmd.print_help();
        println!();
        std::process::exit(0);
    };

    if let Commands::Init = command {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        if let Err(e) = init_command(&current_dir) {
            eprintln!("{}", e);
            std::process::exit(1);
        }
        return;
    }

    if let Commands::ShellIntegration { shell } = command {
        match shell {
            Some(s) => {
                if let Err(e) = shell_integration_command(s) {
                    eprintln!("Shell integration error: {}", e);
                    std::process::exit(1);
                }
            }
            None => {
                rb_cli::commands::shell_integration::show_available_integrations();
            }
        }
        return;
    }

    // Resolve search directory for Ruby installations
    let rubies_dir = resolve_search_dir(cli.config.rubies_dir.clone());

    // Perform comprehensive environment discovery once
    let is_completion = matches!(command, Commands::BashComplete { .. });

    let butler_runtime = match ButlerRuntime::discover_and_compose_with_gem_base(
        rubies_dir.clone(),
        cli.config.ruby_version.clone(),
        cli.config.gem_home.clone(),
        cli.config.no_bundler.unwrap_or(false),
    ) {
        Ok(runtime) => runtime,
        Err(e) => {
            if is_completion {
                // Completion: exit silently with no suggestions when no Ruby found
                std::process::exit(0);
            } else {
                // Interactive commands: show helpful error with search details
                match e {
                    ButlerError::RubiesDirectoryNotFound(path) => {
                        eprintln!(
                            "The designated Ruby estate directory appears to be absent from your system."
                        );
                        eprintln!();
                        eprintln!("Searched in:");
                        eprintln!("  • {}", path.display());

                        // Show why this path was used
                        if let Some(ref config_rubies) = cli.config.rubies_dir {
                            eprintln!("    (from config: {})", config_rubies.display());
                        } else {
                            eprintln!("    (default location)");
                        }

                        if let Some(ref requested_version) = cli.config.ruby_version {
                            eprintln!();
                            eprintln!("Requested version: {}", requested_version);
                        }

                        eprintln!();
                        eprintln!(
                            "May I suggest installing Ruby using ruby-install or a similar distinguished tool?"
                        );
                        std::process::exit(1);
                    }
                    ButlerError::NoSuitableRuby(msg) => {
                        eprintln!("No suitable Ruby installation found: {}", msg);
                        eprintln!();
                        eprintln!("Searched in: {}", rubies_dir.display());

                        if let Some(ref requested_version) = cli.config.ruby_version {
                            eprintln!("Requested version: {}", requested_version);
                        }

                        eprintln!();
                        eprintln!("May I suggest installing a suitable Ruby version?");
                        std::process::exit(1);
                    }
                    _ => {
                        eprintln!("Ruby detection encountered an unexpected difficulty: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    };

    match command {
        Commands::Runtime => {
            runtime_command(&butler_runtime);
        }
        Commands::Environment => {
            environment_command(&butler_runtime, cli.project_file);
        }
        Commands::Exec { args } => {
            exec_command(butler_runtime, args);
        }
        Commands::Run { script, args } => {
            run_command(butler_runtime, script, args, cli.project_file);
        }
        Commands::Init => {
            // Already handled above
            unreachable!()
        }
        Commands::Sync => {
            if let Err(e) = sync_command(butler_runtime) {
                eprintln!("Sync failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::ShellIntegration { .. } => unreachable!(),
        Commands::BashComplete { line, point } => {
            rb_cli::completion::generate_completions(&line, &point, &butler_runtime);
        }
    }
}
