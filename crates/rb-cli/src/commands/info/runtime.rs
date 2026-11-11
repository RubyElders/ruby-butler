use crate::{output::Output, packages::PackageRuntime};
use rb_core::butler::ButlerError;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub(crate) struct RubyEntry {
    pub engine: &'static str,
    pub version: String,
    pub path: PathBuf,
    pub gem_home: Option<PathBuf>,
    pub gem_dirs: Vec<PathBuf>,
    pub bin_dirs: Vec<PathBuf>,
}
#[derive(Serialize)]
pub(crate) struct RuntimeReport {
    pub directory: PathBuf,
    pub requested: Option<String>,
    pub installations: Vec<RubyEntry>,
    pub selected: Option<usize>,
}
#[derive(Serialize)]
pub(crate) struct AvailableRuby {
    pub version: String,
    pub installed: bool,
}

pub(crate) fn runtime_command(
    runtime: &PackageRuntime,
    output: &Output,
) -> Result<(), ButlerError> {
    log::info!(
        "Surveying Ruby installations in distinguished directory: {}",
        runtime.rubies_dir().display()
    );
    log::debug!("Surveying directory: {}", runtime.rubies_dir().display());
    log::debug!(
        "Found {} Ruby installations",
        runtime.ruby_installations().len()
    );
    let requested = runtime.requested_ruby_version().map(str::to_owned);
    let selected = if let Some(request) = &requested {
        runtime.ruby_installations().iter().position(|ruby| {
            if let Ok(version) = semver::Version::parse(request) {
                ruby.version == version
            } else {
                ruby.version.to_string() == *request
            }
        })
    } else {
        runtime
            .ruby_installations()
            .iter()
            .enumerate()
            .max_by_key(|(_, ruby)| &ruby.version)
            .map(|(index, _)| index)
    };
    let installations = runtime
        .ruby_installations()
        .iter()
        .map(|ruby| {
            let gems = if let Some(base) = runtime.gem_base_dir() {
                log::debug!(
                    "Using custom gem base directory for Ruby {}: {}",
                    ruby.version,
                    base.display()
                );
                Ok(ruby.gem_runtime_for_base(base))
            } else {
                ruby.infer_gem_runtime()
            };
            let (gem_home, mut gem_dirs, mut bin_dirs) = match gems {
                Ok(gems) => {
                    log::debug!(
                        "Created gem runtime for Ruby {}: {}",
                        ruby.version,
                        gems.gem_home.display()
                    );
                    (
                        Some(gems.gem_home.clone()),
                        gems.gem_dirs(),
                        gems.bin_dirs(),
                    )
                }
                Err(error) => {
                    log::debug!(
                        "Failed to create gem runtime for Ruby {}: {}",
                        ruby.version,
                        error
                    );
                    (None, Vec::new(), Vec::new())
                }
            };
            gem_dirs.extend(ruby.gem_dirs());
            bin_dirs.extend(ruby.bin_dirs());
            if gem_home.is_some() {
                log::debug!(
                    "Composed paths for Ruby {}: {} bin dirs, {} gem dirs",
                    ruby.version,
                    bin_dirs.len(),
                    gem_dirs.len()
                );
            }
            RubyEntry {
                engine: "CRuby",
                version: ruby.version.to_string(),
                path: ruby.root.clone(),
                gem_home,
                gem_dirs,
                bin_dirs,
            }
        })
        .collect();
    let report = RuntimeReport {
        directory: runtime.rubies_dir().into(),
        requested,
        selected,
        installations,
    };
    let error = if report.installations.is_empty() {
        Some(ButlerError::NoSuitableRuby(
            "No Ruby installations found".into(),
        ))
    } else if report.requested.is_some() && report.selected.is_none() {
        Some(ButlerError::NoSuitableRuby(format!(
            "The requested Ruby version {} could not be located",
            report.requested.as_deref().unwrap()
        )))
    } else {
        None
    };
    if let Some(error) = error {
        output.partial(|| crate::presentation::info::runtime::document(&report))?;
        return Err(error);
    }
    if let Some(index) = report.selected {
        let ruby = &report.installations[index];
        if let Some(request) = &report.requested {
            log::debug!("Seeking your requested Ruby version: {}", request);
            log::info!(
                "Your requested Ruby environment has been located: {} {}",
                ruby.engine,
                ruby.version
            );
        } else {
            log::info!(
                "Presenting your finest Ruby installation: {} {}",
                ruby.engine,
                ruby.version
            );
        }
    }
    output.emit("info.runtime", &report, || {
        crate::presentation::info::runtime::document(&report)
    })
}
