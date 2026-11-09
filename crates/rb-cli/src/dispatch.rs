use crate::Commands;
use crate::InfoCommands;
use crate::commands::info::info_config_command;
use crate::commands::{help_command, version_command};
use crate::runtime_helpers::CommandContext;
use rb_core::butler::ButlerError;

use crate::runtime_helpers::{
    bash_complete_in, export_gemfile_command_wrapper, new_command_wrapper,
    shell_integration_command_wrapper, with_command_runtime, with_package_runtime,
};

/// Dispatch a command using its configured package backend.
///
/// ```compile_fail
/// use rb_cli::commands::sync::sync_command;
/// ```
///
/// ```compile_fail
/// use rb_cli::packages::bundler::BundlerBackend;
/// ```
pub fn dispatch_command(
    command: Commands,
    context: &mut CommandContext,
) -> Result<(), ButlerError> {
    let completion = match &command {
        Commands::BashComplete { line, point } => Some(context.for_completion(line, point)?),
        _ => None,
    };
    let effective = completion.as_ref().unwrap_or(context);
    effective.validate_working_directory()?;
    match command {
        Commands::Version => version_command(),
        Commands::Help { command: help_cmd } => help_command(help_cmd),
        Commands::Install {
            command: crate::InstallCommands::Ruby { version },
        } => crate::ruby_install::install(context, version.as_deref()),
        Commands::New => new_command_wrapper(context),
        Commands::ExportGemfile {
            gemfile,
            output,
            format,
        } => export_gemfile_command_wrapper(context, gemfile, output, format),
        Commands::ShellIntegration { shell } => shell_integration_command_wrapper(shell),
        Commands::BashComplete { line, point } => {
            let context = completion.as_ref().unwrap();
            bash_complete_in(context, &line, &point)
        }

        Commands::Run { script, args } => {
            let project = context.script_project()?;
            if script.is_none() {
                return crate::commands::run::list_available_scripts(project);
            }
            with_command_runtime(
                context,
                script.is_some(),
                crate::reporting::Completion::Compact,
                |runtime, backend, report| {
                    crate::commands::run::run_command(
                        runtime.clone(),
                        script,
                        args,
                        project,
                        backend,
                        report,
                    )
                },
            )
        }
        Commands::Exec { args } => with_command_runtime(
            context,
            !args.is_empty(),
            crate::reporting::Completion::Compact,
            |runtime, backend, report| {
                crate::commands::exec::exec_command(runtime.clone(), args, backend, report)
            },
        ),
        Commands::Sync => with_command_runtime(
            context,
            true,
            crate::reporting::Completion::Tree,
            |runtime, backend, report| {
                crate::commands::sync::sync_command(runtime.clone(), backend, report)
            },
        ),

        Commands::Info { command } => match command {
            InfoCommands::Runtime { available: true } => crate::ruby_install::available(context),
            InfoCommands::Config => info_config_command(context.config()),
            InfoCommands::Project => crate::commands::info::project::project_command(
                context.config(),
                context.script_project().ok().flatten(),
            ),
            InfoCommands::Runtime { available: false } => {
                with_package_runtime(context, |runtime, _| {
                    crate::commands::info::runtime::runtime_command(runtime)
                })
            }
            InfoCommands::Env => {
                let project = context.script_project().ok().flatten();
                with_package_runtime(context, |runtime, backend| {
                    crate::commands::info::env::environment_command(runtime, project, backend)
                })
            }
        },
    }
}
