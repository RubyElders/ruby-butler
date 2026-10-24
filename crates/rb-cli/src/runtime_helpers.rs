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

impl CommandContext {
    pub(crate) fn for_completion(&self, line: &str, point: &str) -> Result<Self, ButlerError> {
        use clap::FromArgMatches;
        let matches =
            crate::completion::completed_matches(crate::completion::before_cursor(line, point));
        let mut config = crate::config::RbConfig::from_arg_matches(&matches)
            .map_err(|error| ButlerError::General(error.to_string()))?;
        let config_file = matches.get_one::<PathBuf>("config_file").cloned();
        let project_file = matches.get_one::<PathBuf>("project_file").cloned();
        let explicit = clap::parser::ValueSource::CommandLine;
        if matches.value_source("no_bundler") != Some(explicit) {
            config.no_bundler = None;
        }
        let selected_file = matches.value_source("config_file") == Some(explicit);
        let file = if selected_file {
            crate::config::loader::load_config(config_file)
                .map_err(|error| ButlerError::General(error.to_string()))?
        } else {
            self.config.to_rb_config()
        };
        let config = TrackedConfig::from_merged(&config, &file);
        Ok(Self {
            config,
            project_file: project_file.or_else(|| self.project_file.clone()),
        })
    }

    pub(crate) fn enter_working_directory(&self) -> Result<(), ButlerError> {
        if !self.config.work_dir.source.is_default() {
            let directory = self.config.work_dir.get();
            std::env::set_current_dir(directory).map_err(|error| {
                ButlerError::General(format!(
                    "Failed to change to directory '{}': {error}",
                    directory.display()
                ))
            })?;
            log::debug!("Changed working directory to: {}", directory.display());
        }
        Ok(())
    }

    pub(crate) fn runtime(&self, skip_bundler: bool) -> Result<ButlerRuntime, ButlerError> {
        ButlerRuntime::discover_and_compose_with_gem_base(
            self.config.rubies_dir.get().clone(),
            self.config.ruby_version_for_runtime(),
            Some(self.config.gem_home.get().clone()),
            skip_bundler,
        )
    }
}

pub(crate) fn with_package_runtime<F>(context: &mut CommandContext, f: F) -> Result<(), ButlerError>
where
    F: FnOnce(&crate::packages::PackageRuntime) -> Result<(), ButlerError>,
{
    let runtime = crate::packages::PackageRuntime::discover(context)?;
    if context.config.has_unresolved()
        && let Ok(ruby) = runtime.selected_ruby()
    {
        context
            .config
            .resolve_ruby_version(ruby.version.to_string());
    }
    f(&runtime)
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
    let context = context.for_completion(line, point)?;
    context.enter_working_directory()?;
    let backend = crate::packages::PackageBackend::from_context(&context);
    bash_complete_with_backend(&context, line, point, &backend)
}

pub(crate) fn bash_complete_with_backend(
    context: &CommandContext,
    line: &str,
    point: &str,
    backend: &crate::packages::PackageBackend,
) -> Result<(), ButlerError> {
    let butler_runtime = crate::packages::PackageRuntime::discover(context).ok();

    let bin_dirs = butler_runtime.and_then(|runtime| {
        backend
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
        project_file: context.project_file.clone(),
        rubies_dir: context.config.rubies_dir.get().clone(),
    };
    crate::completion::generate_completions(line, point, &environment);
    Ok(())
}
