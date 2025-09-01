use clap::Parser;
use rb_cli::{Cli, Commands, runtime_command, environment_command, exec_command, sync_command, init_logger, resolve_search_dir};
use rb_core::butler::ButlerRuntime;

fn main() {
    let cli = Cli::parse();
    
    // Initialize logger with the effective log level (considering -v/-vv flags)
    init_logger(cli.effective_log_level());

    // Handle sync command differently since it doesn't use ButlerRuntime in the same way
    if let Commands::Sync = cli.command {
        if let Err(e) = sync_command(cli.rubies_dir.clone(), cli.ruby_version.clone(), cli.gem_home.clone()) {
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
        cli.gem_home
    ) {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
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
