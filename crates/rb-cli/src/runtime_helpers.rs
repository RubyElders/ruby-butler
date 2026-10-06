use crate::Shell;
use crate::commands::{new_command, shell_integration_command};
use crate::config::TrackedConfig;
use rb_core::butler::{ButlerError, ButlerRuntime};
use std::path::PathBuf;

/// Context information for command execution and error handling
pub struct CommandContext {
    pub config: TrackedConfig,
    pub project_file: Option<PathBuf>,
}

/// Create ButlerRuntime lazily and execute command with it
/// Also updates the context with resolved values (e.g., which Ruby was actually selected)
pub fn with_butler_runtime<F>(context: &mut CommandContext, f: F) -> Result<(), ButlerError>
where
    F: FnOnce(&ButlerRuntime) -> Result<(), ButlerError>,
{
    with_runtime(context, *context.config.no_bundler.get(), f)
}

pub fn with_ruby_runtime<F>(context: &mut CommandContext, f: F) -> Result<(), ButlerError>
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
    let rubies_dir = context.config.rubies_dir.get().clone();

    let requested_version = context.config.ruby_version_for_runtime();

    let butler_runtime = ButlerRuntime::discover_and_compose_with_gem_base(
        rubies_dir,
        requested_version,
        Some(context.config.gem_home.get().clone()),
        skip_bundler,
    )?;

    if context.config.has_unresolved()
        && let Ok(ruby_runtime) = butler_runtime.selected_ruby()
    {
        let resolved_version = ruby_runtime.version.to_string();
        context.config.resolve_ruby_version(resolved_version);
    }

    f(&butler_runtime)
}

pub fn export_gemfile_command_wrapper(
    context: &mut CommandContext,
    gemfile: Option<PathBuf>,
    output: Option<PathBuf>,
    format: crate::ProjectFormat,
) -> Result<(), ButlerError> {
    let gemfile = crate::commands::export_gemfile::find_gemfile(
        &std::env::current_dir().map_err(|error| ButlerError::General(error.to_string()))?,
        gemfile.as_deref(),
        std::env::var_os("BUNDLE_GEMFILE").as_deref(),
    )
    .map_err(|error| ButlerError::General(error.to_string()))?;
    let project_file = context.project_file.clone();
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
pub fn new_command_wrapper() -> Result<(), ButlerError> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    new_command(&current_dir).map_err(ButlerError::General)
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
    let rubies_dir = context.config.rubies_dir.get().clone();

    // Completion works for commands/flags even without Ruby
    let butler_runtime = ButlerRuntime::discover_and_compose_with_gem_base(
        rubies_dir,
        context
            .config
            .ruby_version
            .as_ref()
            .map(|v| v.get().clone()),
        Some(context.config.gem_home.get().clone()),
        *context.config.no_bundler.get(),
    )
    .ok();

    crate::completion::generate_completions(line, point, butler_runtime.as_ref());
    Ok(())
}
