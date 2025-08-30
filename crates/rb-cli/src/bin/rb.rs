use clap::Parser;
use rb_cli::{Cli, Commands, runtime_command, init_logger};

fn main() {
    let cli = Cli::parse();
    
    // Initialize logger with the effective log level (considering -v/-vv flags)
    init_logger(cli.effective_log_level());

    match cli.command {
        Commands::Runtime => {
            runtime_command(cli.rubies_dir, cli.ruby_version);
        }
    }
}
