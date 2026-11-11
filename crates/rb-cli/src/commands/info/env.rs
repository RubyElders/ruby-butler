use super::project::ProjectSummary;
use crate::{
    output::Output,
    packages::{PackageBackend, PackageRuntime},
};
use rb_core::{butler::ButlerError, project::ProjectRuntime};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub(crate) struct EnvironmentReport {
    pub rb_version: &'static str,
    pub working_directory: PathBuf,
    pub backend: String,
    pub ruby: RubySelection,
    pub environment: EnvironmentDetails,
    pub script_project: Option<ProjectSummary>,
}

#[derive(Serialize)]
pub(crate) struct RubySelection {
    pub request: String,
    pub version: String,
    pub path: PathBuf,
    pub installed: bool,
}

#[derive(Serialize)]
pub(crate) struct PackageDetails {
    pub root: PathBuf,
    pub gemfile: Option<PathBuf>,
    pub configuration: Option<PathBuf>,
    pub installation: PathBuf,
    pub required_ruby: Option<String>,
    pub configured: bool,
}
#[derive(Serialize)]
pub(crate) struct EnvironmentDetails {
    pub gem_home: Option<PathBuf>,
    pub gem_dirs: Vec<PathBuf>,
    pub bin_dirs: Vec<PathBuf>,
    pub package_name: String,
    pub package: Option<PackageDetails>,
    pub synchronized: Option<bool>,
}
pub(crate) fn environment_command(
    runtime: &PackageRuntime,
    project: Option<ProjectRuntime>,
    backend: &PackageBackend,
    backend_name: &str,
    output: &Output,
) -> Result<(), ButlerError> {
    log::info!("Presenting current Ruby environment from the working directory");
    let environment = backend
        .inspect(runtime)
        .map_err(|error| ButlerError::General(format!("{error:#}")))?;
    let synchronized = if environment.details.as_ref().is_some_and(|d| d.configured) {
        backend
            .check(runtime)
            .map(|status| matches!(status, crate::packages::PackageStatus::Ready(_)))
            .map_err(|error| {
                log::debug!("Unable to check package synchronization: {error:#}");
                error
            })
            .ok()
    } else {
        Some(false)
    };
    log::debug!(
        "Current working directory: {}",
        environment.current_dir().display()
    );
    let ruby = environment.ruby()?;
    let report = EnvironmentReport {
        rb_version: env!("CARGO_PKG_VERSION"),
        working_directory: environment.current_dir().into(),
        backend: backend_name.into(),
        ruby: RubySelection {
            request: runtime
                .requested_ruby_version()
                .map(str::to_owned)
                .unwrap_or_else(|| "latest".into()),
            version: ruby.version.to_string(),
            path: ruby.root.clone(),
            installed: true,
        },
        script_project: project.as_ref().map(ProjectSummary::collect),
        environment: EnvironmentDetails {
            gem_home: environment.gem_home(),
            gem_dirs: environment.gem_dirs(),
            bin_dirs: environment.bin_dirs(),
            package_name: environment.name.into(),
            synchronized,
            package: environment.details.as_ref().map(|details| PackageDetails {
                root: details.root.clone(),
                gemfile: details.gemfile.clone(),
                configuration: details.configuration.clone(),
                installation: details.installation.clone(),
                required_ruby: details.required_ruby.as_ref().map(ToString::to_string),
                configured: details.configured,
            }),
        },
    };
    output.emit("info.env", &report, || {
        crate::presentation::info::env::document(&report)
    })
}
