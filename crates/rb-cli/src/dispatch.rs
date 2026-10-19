use crate::Commands;
use crate::InfoCommands;
use crate::commands::info::info_config_command;
use crate::commands::{help_command, info_command, version_command};
use crate::runtime_helpers::CommandContext;
use rb_core::butler::ButlerError;

use crate::runtime_helpers::{
    bash_complete_command, export_gemfile_command_wrapper, new_command_wrapper,
    shell_integration_command_wrapper, with_butler_runtime,
};

/// Dispatch command to appropriate handler
pub fn dispatch_command(
    command: Commands,
    context: &mut CommandContext,
) -> Result<(), ButlerError> {
    let backend = crate::packages::select(
        *context.config.package_backend.get(),
        context.project_file.clone(),
        *context.config.no_bundler.get(),
    );
    match command {
        Commands::Version => version_command(),
        Commands::Help { command: help_cmd } => help_command(help_cmd),
        Commands::New => new_command_wrapper(),
        Commands::ExportGemfile {
            gemfile,
            output,
            format,
        } => export_gemfile_command_wrapper(context, gemfile, output, format),
        Commands::ShellIntegration { shell } => shell_integration_command_wrapper(shell),
        Commands::BashComplete { line, point } => bash_complete_command(context, &line, &point),

        Commands::Run { script, args } => {
            let project_file = context.project_file.clone();
            with_butler_runtime(context, |runtime| {
                crate::commands::run::run_with_backend(
                    runtime.clone(),
                    script,
                    args,
                    project_file,
                    backend.as_ref(),
                )
            })
        }
        Commands::Exec { args } => with_butler_runtime(context, |runtime| {
            crate::commands::exec::exec_with_backend(runtime.clone(), args, backend.as_ref())
        }),
        Commands::Sync => with_butler_runtime(context, |runtime| {
            crate::commands::sync::sync_with_backend(runtime.clone(), backend.as_ref())
        }),

        Commands::Info { command } => match command {
            InfoCommands::Config => info_config_command(&context.config),
            _ => {
                let project_file = context.project_file.clone();
                with_butler_runtime(context, |runtime| {
                    info_command(&command, runtime, project_file)
                })
            }
        },
    }
}
