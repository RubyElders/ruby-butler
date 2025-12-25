use crate::Commands;
use crate::commands::{
    config_command, environment_command, exec_command, help_command, run_command, runtime_command,
    sync_command, version_command,
};
use crate::runtime_helpers::CommandContext;
use rb_core::butler::ButlerError;

use crate::runtime_helpers::{
    bash_complete_command, init_command_wrapper, shell_integration_command_wrapper,
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
        Commands::Init => init_command_wrapper(),
        Commands::Config => config_command(&context.config),
        Commands::ShellIntegration { shell } => shell_integration_command_wrapper(shell),
        Commands::BashComplete { line, point } => bash_complete_command(context, &line, &point),

        // Runtime commands - create ButlerRuntime lazily
        Commands::Runtime => with_butler_runtime(context, runtime_command),
        Commands::Environment => {
            let project_file = context.project_file.clone();
            with_butler_runtime(context, |runtime| {
                environment_command(runtime, project_file)
            })
        }
        Commands::Exec { args } => {
            with_butler_runtime(context, |runtime| exec_command(runtime.clone(), args))
        }
        Commands::Run { script, args } => {
            let project_file = context.project_file.clone();
            with_butler_runtime(context, |runtime| {
                run_command(runtime.clone(), script, args, project_file)
            })
        }
        Commands::Sync => with_butler_runtime(context, |runtime| sync_command(runtime.clone())),
    }
}
