use crate::Commands;
use crate::InfoCommands;
use crate::commands::info::info_config_command;
use crate::commands::{help_command, info_command, version_command};
use crate::runtime_helpers::CommandContext;
use rb_core::butler::ButlerError;

use crate::runtime_helpers::{
    bash_complete_with_backend, export_gemfile_command_wrapper, new_command_wrapper,
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
    effective.enter_working_directory()?;
    let backend = crate::packages::PackageBackend::from_context(effective);
    match command {
        Commands::Version => version_command(),
        Commands::Help { command: help_cmd } => help_command(help_cmd),
        Commands::Install {
            command: crate::InstallCommands::Ruby { version },
        } => crate::ruby_install::install(context, version.as_deref()),
        Commands::New => new_command_wrapper(),
        Commands::ExportGemfile {
            gemfile,
            output,
            format,
        } => export_gemfile_command_wrapper(context, gemfile, output, format),
        Commands::ShellIntegration { shell } => shell_integration_command_wrapper(shell),
        Commands::BashComplete { line, point } => {
            bash_complete_with_backend(completion.as_ref().unwrap(), &line, &point, &backend)
        }

        Commands::Run { script, args } => {
            let project_file = context.project_file.clone();
            with_command_runtime(
                context,
                script.is_some(),
                crate::reporting::Completion::Compact,
                |runtime, report| {
                    crate::commands::run::run_command(
                        runtime.clone(),
                        script,
                        args,
                        project_file,
                        &backend,
                        report,
                    )
                },
            )
        }
        Commands::Exec { args } => with_command_runtime(
            context,
            !args.is_empty(),
            crate::reporting::Completion::Compact,
            |runtime, report| {
                crate::commands::exec::exec_command(runtime.clone(), args, &backend, report)
            },
        ),
        Commands::Sync => with_command_runtime(
            context,
            true,
            crate::reporting::Completion::Tree,
            |runtime, report| {
                crate::commands::sync::sync_command(runtime.clone(), &backend, report)
            },
        ),

        Commands::Info { command } => match command {
            InfoCommands::Runtime { available: true } => crate::ruby_install::available(context),
            InfoCommands::Config => info_config_command(&context.config),
            _ => {
                let project_file = context.project_file.clone();
                with_package_runtime(context, |runtime| {
                    info_command(&command, runtime, project_file, &backend)
                })
            }
        },
    }
}
