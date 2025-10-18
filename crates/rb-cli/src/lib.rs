pub mod commands;
pub mod config;
pub mod discovery;

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand, ValueEnum};
use config::{ConfigError, RbConfig};

// Configures Clap v4-style help menu colors (same as cargo and uv)
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Clone, Debug, ValueEnum)]
pub enum LogLevel {
    /// Maintain dignified silence (default)
    None,
    /// Provide informational updates with appropriate discretion
    Info,
    /// Furnish comprehensive diagnostic details for troubleshooting
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
#[command(about = "🎩 Ruby Butler - Your distinguished Ruby environment manager")]
#[command(
    long_about = "🎩 Ruby Butler\n\nA sophisticated Ruby environment manager that orchestrates your Ruby installations\nand gem collections with the refined precision of a proper gentleman's gentleman.\n\nNot merely a version switcher, but your devoted aide in curating Ruby environments\nwith the elegance and attention to detail befitting a distinguished developer.\n\n                        At your service,\n                        RubyElders.com"
)]
#[command(author = "RubyElders.com")]
#[command(version)]
#[command(propagate_version = true)]
#[command(styles = STYLES)]
pub struct Cli {
    /// Specify verbosity for diagnostic output (increases with each use)
    #[arg(
        long,
        value_enum,
        default_value = "none",
        global = true,
        help = "Specify verbosity for diagnostic output"
    )]
    pub log_level: LogLevel,

    /// Enhance verbosity gradually (-v for details, -vv for comprehensive diagnostics)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true, help = "Enhance verbosity gradually (-v for details, -vv for comprehensive diagnostics)")]
    pub verbose: u8,

    /// Specify custom configuration file location
    #[arg(
        short = 'c',
        long = "config",
        global = true,
        help = "Specify custom configuration file location (overrides RB_CONFIG env var and default locations)"
    )]
    pub config_file: Option<std::path::PathBuf>,

    /// Specify custom project file location
    #[arg(
        short = 'P',
        long = "project",
        global = true,
        help = "Specify custom rbproject.toml location (skips autodetection)"
    )]
    pub project_file: Option<std::path::PathBuf>,

    /// Flattened configuration options (works for both CLI and config file)
    #[command(flatten)]
    pub config: RbConfig,

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
    /// 🔍 Survey your distinguished Ruby estate and present available environments
    #[command(visible_alias = "rt")]
    Runtime,

    /// 🌍 Present your current Ruby environment with comprehensive details
    #[command(visible_alias = "env")]
    Environment,

    /// ⚡ Execute commands within your meticulously prepared Ruby environment
    #[command(visible_alias = "x")]
    Exec {
        /// The program and its arguments to execute with proper environmental preparation
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// 🔄 Synchronize your bundler environment with distinguished precision
    #[command(visible_alias = "s")]
    Sync,

    /// 🎯 Execute project scripts defined in rbproject.toml
    #[command(
        visible_alias = "r",
        about = "🎯 Execute project scripts defined in rbproject.toml",
        long_about = "🎯 Run Project Scripts\n\nExecute scripts defined in your project's rbproject.toml file with the\nmeticulously prepared Ruby environment appropriate to your distinguished project.\n\nProject scripts provide convenient shortcuts for common development tasks,\nconfigured with the same refined precision befitting a proper Ruby development workflow.\n\nRun without a script name to list all available scripts."
    )]
    Run {
        /// Name of the script to execute (from rbproject.toml), or omit to list available scripts
        #[arg(help = "Name of the script to execute (omit to list available scripts)")]
        script: Option<String>,

        /// Additional arguments to pass to the script
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Additional arguments to pass to the script"
        )]
        args: Vec<String>,
    },

    /// 📝 Initialize a new rbproject.toml in the current directory
    #[command(about = "📝 Initialize a new rbproject.toml in the current directory")]
    Init,
}

// Re-export for convenience
pub use commands::{
    environment_command, exec_command, init_command, run_command, runtime_command, sync_command,
};

use log::debug;
use rb_core::butler::ButlerRuntime;
use std::path::PathBuf;

const DEFAULT_RUBIES_DIR: &str = ".rubies";

/// Create Ruby context by discovering and setting up ButlerRuntime
pub fn create_ruby_context(
    rubies_dir: Option<PathBuf>,
    ruby_version: Option<String>,
) -> ButlerRuntime {
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
        let home_dir = home::home_dir().expect("Could not determine home directory");
        debug!("Using home directory: {}", home_dir.display());
        let rubies_dir = home_dir.join(DEFAULT_RUBIES_DIR);
        debug!(
            "No rubies directory specified, using default: {}",
            rubies_dir.display()
        );
        rubies_dir
    })
}

impl Cli {
    /// Merge CLI arguments with config file defaults
    /// CLI arguments always take precedence over config file values
    pub fn with_config_defaults(mut self) -> Result<Self, ConfigError> {
        let file_config = config::loader::load_config(self.config_file.clone())?;
        self.config.merge_with(file_config);
        Ok(self)
    }
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
        sandbox
            .add_ruby_dir("3.2.5")
            .expect("Failed to create ruby-3.2.5");

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
            config_file: None,
            project_file: None,
            config: RbConfig::default(),
            command: Commands::Runtime,
        };
        assert!(matches!(cli.effective_log_level(), LogLevel::Info));

        // Test with -v flag (should override log_level to Info)
        let cli = Cli {
            log_level: LogLevel::None,
            verbose: 1,
            config_file: None,
            project_file: None,
            config: RbConfig::default(),
            command: Commands::Runtime,
        };
        assert!(matches!(cli.effective_log_level(), LogLevel::Info));

        // Test with -vv flag (should override log_level to Debug)
        let cli = Cli {
            log_level: LogLevel::None,
            verbose: 2,
            config_file: None,
            project_file: None,
            config: RbConfig::default(),
            command: Commands::Runtime,
        };
        assert!(matches!(cli.effective_log_level(), LogLevel::Debug));
    }
}
