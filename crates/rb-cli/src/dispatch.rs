use crate::Commands;
use crate::InfoCommands;
use crate::commands::info::info_config_command;
use crate::commands::{
    exec_command, help_command, info_command, run_command, sync_command, version_command,
};
use crate::runtime_helpers::CommandContext;
use rb_core::butler::ButlerError;

use crate::runtime_helpers::{
    bash_complete_command, new_command_wrapper, shell_integration_command_wrapper,
    with_butler_runtime,
};

/// Dispatch command to appropriate handler
pub fn dispatch_command(
    command: Commands,
    context: &mut CommandContext,
) -> Result<(), ButlerError> {
    match command {
        // Utility commands - no runtime needed
        Commands::Version => version_command(),
        Commands::Help { command: help_cmd } => help_command(help_cmd),
        Commands::New => new_command_wrapper(),
        Commands::ShellIntegration { shell } => shell_integration_command_wrapper(shell),
        Commands::BashComplete { line, point } => bash_complete_command(context, &line, &point),

        // Workflow commands - create ButlerRuntime
        Commands::Run { script, args } => {
            let project_file = context.project_file.clone();
            with_butler_runtime(context, |runtime| {
                run_command(runtime.clone(), script, args, project_file)
            })
        }
        Commands::Exec { args } => {
            with_butler_runtime(context, |runtime| exec_command(runtime.clone(), args))
        }
        Commands::Sync => with_butler_runtime(context, |runtime| sync_command(runtime.clone())),

        // Diagnostic commands
        Commands::Info { command } => match command {
            InfoCommands::Config => {
                // Config doesn't need runtime, just the config
                info_config_command(&context.config)
            }
            _ => {
                // Other info commands need runtime
                let project_file = context.project_file.clone();
                with_butler_runtime(context, |runtime| {
                    info_command(&command, runtime, project_file)
                })
            }
        },
    }
}
