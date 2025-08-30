use clap::Parser;
use rb_cli::{Cli, Commands, runtime_command};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Runtime { directory } => {
            runtime_command(directory);
        }
    }
}
