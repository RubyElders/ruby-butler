use clap::Parser;
use rb_cli::config::TrackedConfig;
use rb_cli::dispatch::dispatch_command;
use rb_cli::error_display::{
    error_exit_code, format_command_not_found, format_general_error, format_no_suitable_ruby,
    format_rubies_dir_not_found,
};
use rb_cli::help_formatter::print_custom_help;
use rb_cli::runtime_helpers::CommandContext;
use rb_cli::{Cli, Commands, init_logger};
use rb_core::butler::ButlerError;

/// Centralized error handler that transforms technical errors into friendly messages
fn handle_command_error(error: ButlerError, context: &CommandContext) -> ! {
    let message = match &error {
        ButlerError::NoSuitableRuby(_) => {
            let rubies_dir = context.config.rubies_dir.get();
            let source = context.config.rubies_dir.source.to_string();
            let version_info = context
                .config
                .ruby_version
                .as_ref()
                .map(|v| (v.get().clone(), v.source.to_string()));
            format_no_suitable_ruby(rubies_dir, source, version_info)
        }
        ButlerError::CommandNotFound(command) => format_command_not_found(command),
        ButlerError::RubiesDirectoryNotFound(path) => format_rubies_dir_not_found(path),
        ButlerError::General(msg) => format_general_error(msg),
    };

    eprintln!("{}", message);
    std::process::exit(error_exit_code(&error));
}

fn main() {
    let cli = Cli::parse();

    // Skip logging for bash completion (must be silent)
    if !matches!(cli.command, Some(Commands::BashComplete { .. })) {
        init_logger(cli.effective_log_level());
    }

    // Merge config file defaults with CLI arguments (just data, no side effects)
    let (cli_parsed, file_config) = match cli.with_config_defaults_tracked() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    let Some(command) = cli_parsed.command else {
        use clap::CommandFactory;
        let cmd = Cli::command();
        print_custom_help(&cmd);
        std::process::exit(0);
    };

    // Create tracked config with sources
    let tracked_config = TrackedConfig::from_merged(&cli_parsed.config, &file_config);

    // Change working directory if specified
    if !tracked_config.work_dir.source.is_default() {
        let target_dir = tracked_config.work_dir.get();
        if let Err(e) = std::env::set_current_dir(target_dir) {
            eprintln!(
                "Failed to change to directory '{}': {}",
                target_dir.display(),
                e
            );
            std::process::exit(1);
        }
        use log::debug;
        debug!("Changed working directory to: {}", target_dir.display());
    }

    // Create command context (just config data, no runtime discovery yet)
    let mut context = CommandContext {
        config: tracked_config,
        project_file: cli_parsed.project_file.clone(),
    };

    // Dispatch to appropriate command handler
    let result = dispatch_command(command, &mut context);

    // Handle any errors with consistent, friendly messages
    if let Err(e) = result {
        handle_command_error(e, &context);
    }
}
