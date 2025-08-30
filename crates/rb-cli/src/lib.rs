pub mod commands;

use clap::{Parser, Subcommand};
use clap::builder::styling::{AnsiColor, Effects, Styles};

// Configures Clap v4-style help menu colors (same as cargo and uv)
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(name = "rb")]
#[command(about = "🎩 Ruby Butler - An elegant Ruby version manager for distinguished developers")]
#[command(long_about = "🎩 Ruby Butler\n\nA refined tool for managing Ruby installations and gem environments,\ncrafted with the discerning taste of the Ruby community's most distinguished members.\n\nBrought to you by RubyElders.com")]
#[command(author = "by RubyElders.com")]
#[command(version = "0.1.0")]
#[command(propagate_version = true)]
#[command(styles = STYLES)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 🔍 Discover and list available Ruby runtimes with distinguished grace
    #[command(visible_alias = "rt")]
    Runtime {
        /// Directory to search for Ruby installations
        #[arg(short, long, help = "Directory to search for Ruby installations")]
        directory: Option<std::path::PathBuf>,
    },
}

// Re-export for convenience
pub use commands::runtime_command;