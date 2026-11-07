pub mod config;
pub mod env;
pub mod project;
pub mod runtime;

use crate::packages::PackageRuntime;
use rb_core::butler::ButlerError;
use std::path::PathBuf;

use crate::InfoCommands;
use crate::config::TrackedConfig;

pub(crate) fn info_command(
    command: &InfoCommands,
    butler_runtime: &PackageRuntime,
    project_file: Option<PathBuf>,
    backend: &crate::packages::PackageBackend,
) -> Result<(), ButlerError> {
    match command {
        InfoCommands::Runtime { .. } => runtime::runtime_command(butler_runtime),
        InfoCommands::Env => env::environment_command(butler_runtime, project_file, backend),
        InfoCommands::Project => project::project_command(butler_runtime, project_file),
        InfoCommands::Config => {
            // Config command doesn't actually need the runtime, but we have it available
            // For now, return an error - this will be handled specially in dispatch
            Err(ButlerError::General(
                "Config command should be handled in dispatch".to_string(),
            ))
        }
    }
}

pub fn info_config_command(config: &TrackedConfig) -> Result<(), ButlerError> {
    config::config_command(config)
}
