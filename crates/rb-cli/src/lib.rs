pub mod commands;

use clap::{Parser, Subcommand, ValueEnum};
use clap::builder::styling::{AnsiColor, Effects, Styles};

// Configures Clap v4-style help menu colors (same as cargo and uv)
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Clone, Debug, ValueEnum)]
pub enum LogLevel {
    /// No logging output (default)
    None,
    /// Show informational messages and errors
    Info,
    /// Show detailed debug information for troubleshooting
    Debug,
}

impl From<LogLevel> for log::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::None => log::LevelFilter::Off,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
        }
    }
}

#[derive(Parser)]
#[command(name = "rb")]
#[command(about = "🎩 Ruby Butler - An elegant Ruby version manager for distinguished developers")]
#[command(long_about = "🎩 Ruby Butler\n\nA refined tool for managing Ruby installations and gem environments,\ncrafted with the discerning taste of the Ruby community's most distinguished members.\n\nBrought to you by RubyElders.com")]
#[command(author = "by RubyElders.com")]
#[command(version = "0.1.0")]
#[command(propagate_version = true)]
#[command(styles = STYLES)]
pub struct Cli {
    /// Set the log level for output verbosity
    #[arg(long, value_enum, default_value = "none", global = true, help = "Set the log level for output verbosity")]
    pub log_level: LogLevel,

    /// Increase verbosity (-v for info, -vv for debug)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true, help = "Increase verbosity (-v for info, -vv for debug)")]
    pub verbose: u8,

    /// Directory to search for Ruby installations
    #[arg(short = 'R', long = "rubies-dir", global = true, help = "Directory to search for Ruby installations (default: ~/.rubies)")]
    pub rubies_dir: Option<std::path::PathBuf>,

    /// Select specific Ruby version (defaults to latest)
    #[arg(short = 'r', long = "ruby", global = true, help = "Select specific Ruby version (defaults to latest)")]
    pub ruby_version: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// Get the effective log level, considering both --log-level and -v/-vv flags
    /// The verbose flags take precedence over --log-level when specified
    pub fn effective_log_level(&self) -> LogLevel {
        match self.verbose {
            0 => self.log_level.clone(), // Use explicit log level
            1 => LogLevel::Info,         // -v
            _ => LogLevel::Debug,        // -vv or more
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// 🔍 Discover and list available Ruby runtimes with distinguished grace
    #[command(visible_alias = "rt")]
    Runtime,
}

// Re-export for convenience
pub use commands::runtime_command;

/// Initialize the logger with the specified log level
pub fn init_logger(log_level: LogLevel) {
    env_logger::Builder::from_default_env()
        .filter_level(log_level.into())
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();
}