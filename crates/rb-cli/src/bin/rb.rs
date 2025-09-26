use clap::Parser;
use rb_cli::{
    Cli, Commands, environment_command, exec_command, init_logger, resolve_search_dir,
    runtime_command, sync_command,
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
    
    parts.push("\n\nA sophisticated Ruby environment manager with the refined precision".to_string());
    parts.push("of a proper gentleman's gentleman.\n".to_string());
    parts.push("At your distinguished service, RubyElders.com".to_string());
    
    parts.join(" ")
}

fn main() {
    // Handle version request with custom formatting before parsing
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("{}", build_version_info());
        return;
    }
    
    let cli = Cli::parse();

    // Initialize logger with the effective log level (considering -v/-vv flags)
    init_logger(cli.effective_log_level());

    // Handle sync command differently since it doesn't use ButlerRuntime in the same way
    if let Commands::Sync = cli.command {
        if let Err(e) = sync_command(
            cli.rubies_dir.clone(),
            cli.ruby_version.clone(),
            cli.gem_home.clone(),
        ) {
            eprintln!("Sync failed: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // Resolve search directory for Ruby installations
    let rubies_dir = resolve_search_dir(cli.rubies_dir);

    // Perform comprehensive environment discovery once
    let butler_runtime = match ButlerRuntime::discover_and_compose_with_gem_base(
        rubies_dir,
        cli.ruby_version,
        cli.gem_home,
    ) {
        Ok(runtime) => runtime,
        Err(e) => match e {
            ButlerError::RubiesDirectoryNotFound(path) => {
                eprintln!("🎩 My sincerest apologies, but the designated Ruby estate directory");
                eprintln!(
                    "   '{}' appears to be absent from your system.",
                    path.display()
                );
                eprintln!();
                eprintln!("Without access to a properly established Ruby estate, I'm afraid");
                eprintln!(
                    "there's precious little this humble Butler can accomplish on your behalf."
                );
                eprintln!();
                eprintln!("May I suggest installing Ruby using ruby-install or a similar");
                eprintln!("distinguished tool to establish your Ruby installations at the");
                eprintln!("expected location, then we shall proceed with appropriate ceremony.");
                std::process::exit(1);
            }
            _ => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
    };

    match cli.command {
        Commands::Runtime => {
            runtime_command(&butler_runtime);
        }
        Commands::Environment => {
            environment_command(&butler_runtime);
        }
        Commands::Exec { args } => {
            exec_command(butler_runtime, args);
        }
        Commands::Sync => {
            // Already handled above
            unreachable!()
        }
    }
}
