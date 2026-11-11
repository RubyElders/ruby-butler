use clap::{CommandFactory, FromArgMatches};
use rb_cli::config::TrackedConfig;
use rb_cli::dispatch::dispatch_command;
use rb_cli::error_display::{
    error_exit_code, format_command_not_found, format_general_error, format_no_suitable_ruby,
    format_rubies_dir_not_found,
};
use rb_cli::runtime_helpers::CommandContext;
use rb_cli::{Cli, Commands, init_logger};
use rb_core::butler::ButlerError;

/// Centralized error handler that transforms technical errors into friendly messages
fn handle_command_error(error: ButlerError, context: &CommandContext) -> ! {
    let message = match &error {
        ButlerError::NoSuitableRuby(_) => {
            let rubies_dir = context.config().rubies_dir.get();
            let source = context.config().rubies_dir.source.to_string();
            let version_info = context
                .config()
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
    let inputs = rb_core::process_context::ProcessContext::capture().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1)
    });
    let matches = Cli::command().get_matches();
    let mut cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    if cli.help {
        eprintln!("Warning: use `rb help` instead of `rb --help`.");
        cli.command = Some(Commands::Help { command: None });
    }

    // Skip logging for bash completion (must be silent)
    if !matches!(cli.command, Some(Commands::BashComplete { .. })) {
        init_logger(cli.effective_log_level());
    }

    let (cli_parsed, file_config) = match cli.with_config_defaults_tracked_in(&inputs) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    let Some(command) = cli_parsed.command else {
        rb_cli::commands::help::help_with_output(
            None,
            &rb_cli::output::Output::new(cli_parsed.format),
        )
        .unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(1)
        });
        std::process::exit(0);
    };

    let tracked_config =
        TrackedConfig::from_matches(&cli_parsed.config, &file_config, &matches, &inputs);
    let mut context =
        CommandContext::from_inputs(tracked_config, cli_parsed.project_file.clone(), inputs)
            .with_output(cli_parsed.format);

    let result = dispatch_command(command, &mut context);

    if let Err(e) = result {
        handle_command_error(e, &context);
    }
}
