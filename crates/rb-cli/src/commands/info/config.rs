use crate::config::{ConfigSource, TrackedConfig};
use crate::output::Output;
use rb_core::butler::ButlerError;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub(crate) struct Setting<T> {
    pub value: T,
    pub source: &'static str,
}
impl<T> Setting<T> {
    fn new(value: T, source: &ConfigSource) -> Self {
        Self {
            value,
            source: match source {
                ConfigSource::Unresolved => "unresolved",
                ConfigSource::Default => "default",
                ConfigSource::EnvVar => "environment",
                ConfigSource::ConfigFile => "config_file",
                ConfigSource::Cli => "cli",
                ConfigSource::Resolved => "resolved",
            },
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ConfigurationReport {
    pub rubies_dir: Setting<PathBuf>,
    pub ruby_version: Setting<Option<String>>,
    pub ruby_unresolved: bool,
    pub gem_home: Setting<PathBuf>,
    pub no_bundler: Setting<bool>,
    pub work_dir: Setting<PathBuf>,
    pub auto_install_ruby: Setting<bool>,
    pub package_backend: Setting<String>,
}
impl ConfigurationReport {
    pub(crate) fn collect(config: &TrackedConfig) -> Self {
        Self {
            rubies_dir: Setting::new(config.rubies_dir.get().clone(), &config.rubies_dir.source),
            ruby_version: Setting::new(
                config.ruby_version.as_ref().map(|v| v.get().clone()),
                &config
                    .ruby_version
                    .as_ref()
                    .map(|v| v.source)
                    .unwrap_or(ConfigSource::Default),
            ),
            ruby_unresolved: config
                .ruby_version
                .as_ref()
                .is_some_and(|v| v.is_unresolved()),
            gem_home: Setting::new(config.gem_home.get().clone(), &config.gem_home.source),
            no_bundler: Setting::new(*config.no_bundler.get(), &config.no_bundler.source),
            work_dir: Setting::new(config.work_dir.get().clone(), &config.work_dir.source),
            auto_install_ruby: Setting::new(
                *config.auto_install_ruby.get(),
                &config.auto_install_ruby.source,
            ),
            package_backend: Setting::new(
                config.package_backend.get().to_string(),
                &config.package_backend.source,
            ),
        }
    }
}

/// Display current configuration with sources
pub fn config_command(config: &TrackedConfig) -> Result<(), ButlerError> {
    config_command_with_output(config, &Output::default())
}
pub(crate) fn config_command_with_output(
    config: &TrackedConfig,
    output: &Output,
) -> Result<(), ButlerError> {
    let report = ConfigurationReport::collect(config);
    output.emit("info.config", &report, || {
        crate::presentation::info::config::document(&report)
    })
}
