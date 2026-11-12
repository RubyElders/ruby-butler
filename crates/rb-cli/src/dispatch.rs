use crate::Commands;
use crate::InfoCommands;
use crate::runtime_helpers::CommandContext;
use rb_core::butler::ButlerError;

use crate::runtime_helpers::{
    bash_complete_in, export_gemfile_command_wrapper, new_command_wrapper, with_command_runtime,
    with_package_runtime,
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
    let output = context.output;
    let backend_name = context.config().package_backend.get().to_string();
    let completion = match &command {
        Commands::BashComplete { line, point } => Some(context.for_completion(line, point)?),
        _ => None,
    };
    let effective = completion.as_ref().unwrap_or(context);
    effective.validate_working_directory()?;
    match command {
        Commands::Version => crate::commands::version::version_with_output(&context.output),
        Commands::Help { command: help_cmd } => {
            crate::commands::help::help_with_output(help_cmd, &context.output)
        }
        Commands::Install {
            command: crate::InstallCommands::Ruby { version },
        } => crate::ruby_install::install(context, version.as_deref()),
        Commands::New => new_command_wrapper(context),
        Commands::ExportGemfile {
            gemfile,
            output,
            project_format,
        } => export_gemfile_command_wrapper(context, gemfile, output, project_format),
        Commands::ShellIntegration { shell } => match shell {
            Some(shell) => crate::commands::shell_integration::shell_with_output(shell, &output),
            None => crate::commands::shell_integration::available_with_output(&output),
        },
        Commands::BashComplete { line, point } => {
            let context = completion.as_ref().unwrap();
            bash_complete_in(context, &line, &point)
        }

        Commands::Run { script, args } => {
            let project = context.script_project()?;
            if script.is_none() {
                return crate::commands::run::list_available_scripts(project, &context.output);
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
                crate::commands::sync::sync_command(runtime.clone(), backend, report, &output)
            },
        ),

        Commands::Info { command } => match command {
            InfoCommands::Runtime { available: true } => crate::ruby_install::available(context),
            InfoCommands::Config => {
                crate::commands::info::config::config_command_with_output(context.config(), &output)
            }
            InfoCommands::Project => crate::commands::info::project::project_command(
                context.config(),
                context.script_project().ok().flatten(),
                &output,
            ),
            InfoCommands::Runtime { available: false } => {
                with_package_runtime(context, |runtime, _| {
                    crate::commands::info::runtime::runtime_command(runtime, &output)
                })
            }
            InfoCommands::Env {
                discover,
                probe,
                resolve_ruby,
            } if discover || probe => {
                let report =
                    crate::commands::info::discovery::discover(context, probe, resolve_ruby)
                        .map_err(|error| ButlerError::General(format!("{error:#}")))?;
                output.emit("info.env", &report, || {
                    crate::presentation::info::env::document(&report)
                })
            }
            InfoCommands::Env { .. } => {
                let project = context.script_project().ok().flatten();
                with_package_runtime(context, |runtime, backend| {
                    crate::commands::info::env::environment_command(
                        runtime,
                        project,
                        backend,
                        &backend_name,
                        &output,
                    )
                })
            }
        },
    }
}
