pub mod config;
pub mod env;
pub mod project;
pub mod runtime;

use crate::config::TrackedConfig;
use rb_core::butler::ButlerError;

pub fn info_config_command(config: &TrackedConfig) -> Result<(), ButlerError> {
    config::config_command(config)
}
