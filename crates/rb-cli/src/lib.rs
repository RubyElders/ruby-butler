pub mod commands;
pub mod completion;
pub mod config;
pub mod error_display;

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
#[command(disable_help_flag = true)]
#[command(disable_help_subcommand = true)]
#[command(disable_version_flag = true)]
#[command(styles = STYLES)]
#[command(next_help_heading = "Commands")]
pub struct Cli {
    /// Enable informational diagnostic output
    #[arg(
        short = 'v',
        long = "verbose",
        global = true,
        help = "Enable informational diagnostic output (same as --log-level=info)",
        env = "RB_VERBOSE",
        action = clap::ArgAction::SetTrue
    )]
    pub verbose: bool,

    /// Enable comprehensive diagnostic output
    #[arg(
        short = 'V',
        long = "very-verbose",
        global = true,
        help = "Enable comprehensive diagnostic output (same as --log-level=debug)",
        env = "RB_VERY_VERBOSE",
        action = clap::ArgAction::SetTrue
    )]
    pub very_verbose: bool,

    /// Specify verbosity for diagnostic output explicitly
    #[arg(
        short = 'L',
        long = "log-level",
        value_enum,
        global = true,
        help = "Specify verbosity for diagnostic output explicitly",
        env = "RB_LOG_LEVEL"
    )]
    pub log_level: Option<LogLevel>,

    /// Specify custom configuration file location
    #[arg(
        short = 'c',
        long = "config",
        global = true,
        help = "Specify custom configuration file location",
        env = "RB_CONFIG"
    )]
    pub config_file: Option<std::path::PathBuf>,

    /// Specify custom project file location
    #[arg(
        short = 'P',
        long = "project",
        global = true,
        help = "Specify custom rbproject.toml location (skips autodetection)",
        env = "RB_PROJECT"
    )]
    pub project_file: Option<std::path::PathBuf>,

    /// Flattened configuration options (works for both CLI and config file)
    #[command(flatten)]
    pub config: RbConfig,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Cli {
    /// Get the effective log level, considering -v/-V flags and --log-level
    /// Priority: -V > -v > --log-level > default (none)
    pub fn effective_log_level(&self) -> LogLevel {
        if self.very_verbose {
            LogLevel::Debug
        } else if self.verbose {
            LogLevel::Info
        } else {
            self.log_level.clone().unwrap_or(LogLevel::None)
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// 🔍 Survey your distinguished Ruby estate and present available environments
    #[command(visible_alias = "rt", next_help_heading = "Runtime Commands")]
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
    #[command(
        about = "📝 Initialize a new rbproject.toml in the current directory",
        next_help_heading = "Utility Commands"
    )]
    Init,
    /// ⚙️  Display current configuration with sources
    #[command(
        about = "⚙️  Display current configuration with sources",
        next_help_heading = "Utility Commands"
    )]
    Config,
    /// � Display Ruby Butler version information
    #[command(about = "📋 Display Ruby Butler version information")]
    Version,
    /// 📖 Display help information for Ruby Butler or specific commands
    #[command(about = "📖 Display help information for Ruby Butler or specific commands")]
    Help {
        /// The command to get help for
        #[arg(help = "Command to get help for (omit for general help)")]
        command: Option<String>,
    },
    /// �🔧 Generate shell integration (completions) for your distinguished shell
    #[command(about = "🔧 Generate shell integration (completions)")]
    ShellIntegration {
        /// The shell to generate completions for (omit to see available integrations)
        #[arg(value_enum, help = "Shell type (bash)")]
        shell: Option<Shell>,
    },

    /// Internal: Bash completion generator (hidden from help, used by shell integration)
    #[command(name = "__bash_complete", hide = true)]
    BashComplete {
        /// The complete command line being completed
        #[arg(help = "Complete command line (COMP_LINE)")]
        line: String,

        /// The cursor position in the line
        #[arg(help = "Cursor position (COMP_POINT)")]
        point: String,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum Shell {
    Bash,
}

// Re-export for convenience
pub use commands::{
    config_command, environment_command, exec_command, init_command, run_command, runtime_command,
    shell_integration_command, sync_command,
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
        // Check RB_RUBIES_DIR environment variable
        if let Ok(env_dir) = std::env::var("RB_RUBIES_DIR") {
            let path = PathBuf::from(env_dir);
            debug!(
                "Using rubies directory from RB_RUBIES_DIR: {}",
                path.display()
            );
            return path;
        }

        // Fall back to default ~/.rubies
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

    /// Merge CLI arguments with config file, returning both for tracked config
    /// Returns (cli_with_merged_config, file_config) for source tracking
    pub fn with_config_defaults_tracked(self) -> Result<(Self, config::RbConfig), ConfigError> {
        let file_config = config::loader::load_config(self.config_file.clone())?;
        Ok((self, file_config))
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
        // Temporarily unset environment variable for this test
        let original_env = std::env::var("RB_RUBIES_DIR").ok();
        unsafe {
            std::env::remove_var("RB_RUBIES_DIR");
        }

        let result = resolve_search_dir(None);

        // Restore original environment
        if let Some(val) = original_env {
            unsafe {
                std::env::set_var("RB_RUBIES_DIR", val);
            }
        }

        // Should return home directory + .rubies
        assert!(result.ends_with(".rubies"));
        assert!(result.is_absolute());
    }

    #[test]
    fn test_create_ruby_context_with_sandbox() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        let ruby_dir = sandbox
            .add_ruby_dir("3.2.5")
            .expect("Failed to create ruby-3.2.5");

        // Create Ruby executable so it can be discovered
        std::fs::create_dir_all(ruby_dir.join("bin")).expect("Failed to create bin dir");
        let ruby_exe = ruby_dir.join("bin").join("ruby");
        std::fs::write(&ruby_exe, "#!/bin/sh\necho ruby").expect("Failed to write ruby exe");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&ruby_exe, std::fs::Permissions::from_mode(0o755))
                .expect("Failed to set permissions");
        }

        // Create gem directories so gem runtime is inferred
        let gem_base = sandbox.gem_base_dir();
        let gem_dir = gem_base.join("3.2.5");
        std::fs::create_dir_all(&gem_dir).expect("Failed to create gem dir");

        // Use the internal method that accepts current_dir to avoid global state
        use rb_core::butler::ButlerRuntime;
        let result = ButlerRuntime::discover_and_compose_with_current_dir(
            sandbox.root().to_path_buf(),
            None,
            None,
            false,
            sandbox.root().to_path_buf(), // Current dir = sandbox root
        )
        .expect("Failed to create ButlerRuntime");

        // Should successfully create a ButlerRuntime
        let current_path = std::env::var("PATH").ok();
        let env_vars = result.env_vars(current_path);
        assert!(env_vars.contains_key("PATH"));
        assert!(env_vars.contains_key("GEM_HOME"));
        assert!(env_vars.contains_key("GEM_PATH"));
    }

    #[test]
    fn test_effective_log_level_with_verbose_flags() {
        // Test with log_level set
        let cli = Cli {
            log_level: Some(LogLevel::Info),
            verbose: false,
            very_verbose: false,
            config_file: None,
            project_file: None,
            config: RbConfig::default(),
            command: Some(Commands::Runtime),
        };
        assert!(matches!(cli.effective_log_level(), LogLevel::Info));

        // Test with -v flag (should override log_level to Info)
        let cli = Cli {
            log_level: Some(LogLevel::None),
            verbose: true,
            very_verbose: false,
            config_file: None,
            project_file: None,
            config: RbConfig::default(),
            command: Some(Commands::Runtime),
        };
        assert!(matches!(cli.effective_log_level(), LogLevel::Info));

        // Test with -V flag (should override log_level to Debug)
        let cli = Cli {
            log_level: Some(LogLevel::None),
            verbose: false,
            very_verbose: true,
            config_file: None,
            project_file: None,
            config: RbConfig::default(),
            command: Some(Commands::Runtime),
        };
        assert!(matches!(cli.effective_log_level(), LogLevel::Debug));
    }
}
