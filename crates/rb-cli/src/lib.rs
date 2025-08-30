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
    
    /// ⚡ Execute a command in the Ruby environment with composed PATH and gem settings
    #[command(visible_alias = "x")]
    Exec {
        /// The program to execute and its arguments
        #[arg(trailing_var_arg = true, required = true)]
        args: Vec<String>,
    },
}

// Re-export for convenience
pub use commands::{runtime_command, exec_command};

use std::path::PathBuf;
use rb_core::butler::ButlerRuntime;
use home;
use log::debug;

const DEFAULT_RUBIES_DIR: &str = ".rubies";

/// Create Ruby context by discovering and setting up ButlerRuntime
pub fn create_ruby_context(rubies_dir: Option<PathBuf>, ruby_version: Option<String>) -> ButlerRuntime {
    let search_dir = resolve_search_dir(rubies_dir);

    match ButlerRuntime::discover_and_create(&search_dir, ruby_version.as_deref()) {
        Ok(butler) => butler,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Resolve the directory to search for Ruby installations
pub fn resolve_search_dir(rubies_dir: Option<PathBuf>) -> PathBuf {
    rubies_dir.unwrap_or_else(|| {
        let home_dir = home::home_dir()
            .expect("Could not determine home directory");
        debug!("Using home directory: {}", home_dir.display());
        let rubies_dir = home_dir.join(DEFAULT_RUBIES_DIR);
        debug!("No rubies directory specified, using default: {}", rubies_dir.display());
        rubies_dir
    })
}

/// Initialize the logger with the specified log level
pub fn init_logger(log_level: LogLevel) {
    env_logger::Builder::from_default_env()
        .filter_level(log_level.into())
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rb_tests::RubySandbox;

    #[test]
    fn test_resolve_search_dir_with_provided_path() {
        let test_path = PathBuf::from("/test/rubies");
        let result = resolve_search_dir(Some(test_path.clone()));
        assert_eq!(result, test_path);
    }

    #[test]
    fn test_resolve_search_dir_with_none() {
        let result = resolve_search_dir(None);
        // Should return home directory + .rubies
        assert!(result.ends_with(".rubies"));
        assert!(result.is_absolute());
    }

    #[test]
    fn test_create_ruby_context_with_sandbox() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
        
        let result = create_ruby_context(Some(sandbox.root().to_path_buf()), None);
        
        // Should successfully create a ButlerRuntime
        let current_path = std::env::var("PATH").ok();
        let env_vars = result.env_vars(current_path);
        assert!(env_vars.contains_key("PATH"));
        assert!(env_vars.contains_key("GEM_HOME"));
        assert!(env_vars.contains_key("GEM_PATH"));
    }

    #[test]
    fn test_effective_log_level_with_verbose_flags() {
        // Test with no verbose flags
        let cli = Cli {
            log_level: LogLevel::Info,
            verbose: 0,
            rubies_dir: None,
            ruby_version: None,
            command: Commands::Runtime,
        };
        assert!(matches!(cli.effective_log_level(), LogLevel::Info));

        // Test with -v flag (should override log_level to Info)
        let cli = Cli {
            log_level: LogLevel::None,
            verbose: 1,
            rubies_dir: None,
            ruby_version: None,
            command: Commands::Runtime,
        };
        assert!(matches!(cli.effective_log_level(), LogLevel::Info));

        // Test with -vv flag (should override log_level to Debug)
        let cli = Cli {
            log_level: LogLevel::None,
            verbose: 2,
            rubies_dir: None,
            ruby_version: None,
            command: Commands::Runtime,
        };
        assert!(matches!(cli.effective_log_level(), LogLevel::Debug));
    }
}