use crate::Shell;
use crate::commands::{new_command, shell_integration_command};
use rb_core::butler::{ButlerError, ButlerRuntime};
use std::path::PathBuf;

pub use crate::invocation::CommandContext;

pub(crate) fn with_command_runtime<F>(
    context: &mut CommandContext,
    requires_ruby: bool,
    completion: crate::reporting::Completion,
    f: F,
) -> Result<(), ButlerError>
where
    F: FnOnce(
        &crate::packages::PackageRuntime,
        &crate::packages::PackageBackend,
        &mut crate::reporting::PreparationReport,
    ) -> Result<(), ButlerError>,
{
    let mut report = crate::reporting::PreparationReport::new(completion);
    let result = (|| {
        if requires_ruby {
            context.ruby = Some(crate::ruby_install::prepare(context, &mut report)?);
        }
        with_package_runtime(context, |runtime, backend| f(runtime, backend, &mut report))
    })();
    match &result {
        Ok(()) => report.finish("Your environment is ready"),
        Err(error) => report.fail(error),
    }
    result
}

pub(crate) fn with_package_runtime<F>(context: &mut CommandContext, f: F) -> Result<(), ButlerError>
where
    F: FnOnce(
        &crate::packages::PackageRuntime,
        &crate::packages::PackageBackend,
    ) -> Result<(), ButlerError>,
{
    let runtime = crate::packages::PackageRuntime::discover(context)?;
    let backend = crate::packages::PackageBackend::from_context(context);
    f(&runtime, &backend)
}

fn with_ruby_runtime<F>(context: &mut CommandContext, f: F) -> Result<(), ButlerError>
where
    F: FnOnce(&ButlerRuntime) -> Result<(), ButlerError>,
{
    with_runtime(context, true, f)
}

fn with_runtime<F>(
    context: &mut CommandContext,
    skip_bundler: bool,
    f: F,
) -> Result<(), ButlerError>
where
    F: FnOnce(&ButlerRuntime) -> Result<(), ButlerError>,
{
    let butler_runtime = context.runtime(skip_bundler)?;

    f(&butler_runtime)
}

pub fn export_gemfile_command_wrapper(
    context: &mut CommandContext,
    gemfile: Option<PathBuf>,
    output: Option<PathBuf>,
    format: crate::ProjectFormat,
) -> Result<(), ButlerError> {
    let gemfile = crate::commands::export_gemfile::find_gemfile(
        context.process.directory(),
        gemfile.as_deref(),
        context.process.var("BUNDLE_GEMFILE"),
    )
    .map_err(|error| ButlerError::General(error.to_string()))?;
    let project_file = context.project_file().clone();
    let output = output.map(|file| context.process.directory().join(file));
    with_ruby_runtime(context, |runtime| {
        crate::commands::export_gemfile::export_gemfile_command(
            runtime,
            &gemfile,
            project_file.as_deref(),
            output.as_deref(),
            format.into(),
        )
    })
}

/// New command wrapper - no runtime needed
pub fn new_command_wrapper(context: &CommandContext) -> Result<(), ButlerError> {
    new_command(context.process.directory()).map_err(ButlerError::General)
}

/// Shell integration command wrapper - no runtime needed
pub fn shell_integration_command_wrapper(shell: Option<Shell>) -> Result<(), ButlerError> {
    match shell {
        Some(s) => shell_integration_command(s).map_err(|e| ButlerError::General(e.to_string())),
        None => {
            crate::commands::shell_integration::show_available_integrations();
            Ok(())
        }
    }
}

/// Bash completion command - tries to create runtime but gracefully handles failure
pub fn bash_complete_command(
    context: &CommandContext,
    line: &str,
    point: &str,
) -> Result<(), ButlerError> {
    let context = context.for_completion(line, point)?;
    context.validate_working_directory()?;
    bash_complete_in(&context, line, point)
}

pub(crate) fn bash_complete_in(
    context: &CommandContext,
    line: &str,
    point: &str,
) -> Result<(), ButlerError> {
    let matches = context
        .completion_matches
        .as_ref()
        .expect("completion context");
    let line_before = crate::completion::before_cursor(line, point);
    let mut words = shlex::split(line_before).unwrap_or_default();
    if !line_before.ends_with(char::is_whitespace) {
        words.pop();
    }
    let versions = words
        .last()
        .is_some_and(|word| matches!(word.as_str(), "-r" | "--ruby"));
    let ruby_versions = if versions {
        context
            .ruby_inventory()
            .map(|inventory| {
                inventory
                    .iter()
                    .map(|ruby| ruby.version.to_string())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let butler_runtime = if matches.subcommand_name() == Some("exec") {
        crate::packages::PackageRuntime::discover(context).ok()
    } else {
        None
    };
    let bin_dirs = butler_runtime.and_then(|runtime| {
        crate::packages::PackageBackend::from_context(context)
            .inspect(&runtime)
            .map(|environment| environment.bin_dirs())
            .map_err(|error| {
                log::debug!("Unable to discover completion environment: {error:#}");
                error
            })
            .ok()
    });
    let environment = crate::completion::CompletionEnvironment {
        bin_dirs: bin_dirs.unwrap_or_default(),
        project: if matches.subcommand_name() == Some("run") {
            context.script_project().ok().flatten()
        } else {
            None
        },
        directory: context.process.directory().into(),
        ruby_versions,
    };
    crate::completion::generate_completions(line, point, &environment, matches);
    Ok(())
}
