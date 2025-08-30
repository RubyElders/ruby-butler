use clap::Parser;
use rb_cli::{Cli, Commands, runtime_command, exec_command, init_logger, create_ruby_context, resolve_search_dir};

fn main() {
    let cli = Cli::parse();
    
    // Initialize logger with the effective log level (considering -v/-vv flags)
    init_logger(cli.effective_log_level());

    // Determine if this command needs Ruby context (skip for help/version)
    let needs_ruby_context = matches!(cli.command, Commands::Exec { .. });
    
    // Create Ruby context only for commands that need it
    let butler_runtime = if needs_ruby_context {
        Some(create_ruby_context(cli.rubies_dir.clone(), cli.ruby_version.clone()))
    } else {
        None
    };

    // Resolve search directory for commands that need it
    let search_dir = resolve_search_dir(cli.rubies_dir);

    match cli.command {
        Commands::Runtime => {
            runtime_command(search_dir, cli.ruby_version);
        }
        Commands::Exec { args } => {
            exec_command(butler_runtime.expect("Exec command should have Ruby context"), args);
        }
    }
}
