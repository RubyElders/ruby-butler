use crate::{
    invocation::SelectedRuby,
    packages::{PackageBackend, PackageRuntime},
    runtime_helpers::CommandContext,
};
use anyhow::{Context, Result};
use rb_core::butler::ButlerError;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub(crate) struct EnvironmentReport {
    pub inspection: &'static str,
    pub environment: Option<super::env::EnvironmentDetails>,
    pub script_project: Option<super::project::ProjectSummary>,
    pub rb_version: &'static str,
    pub working_directory: PathBuf,
    pub backend: String,
    pub project: Option<DependencyProject>,
    pub ruby: RubySelection,
    pub cache: Option<crate::packages::PackageCache>,
}

#[derive(Serialize)]
pub(crate) struct DependencyProject {
    pub root: PathBuf,
    pub project_file: Option<PathBuf>,
    pub gemfile: Option<PathBuf>,
}

#[derive(Serialize)]
pub(crate) struct RubySelection {
    pub request: String,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
    pub installed: bool,
}

pub(crate) fn discover(
    context: &mut CommandContext,
    probe: bool,
    resolve_ruby: bool,
) -> Result<EnvironmentReport> {
    let request = crate::ruby_install::request::resolve(context, None)?;
    let directory = context
        .process
        .directory()
        .join(context.config().rubies_dir.get());
    let inventory = match context.ruby_inventory() {
        Ok(inventory) => inventory.clone(),
        Err(ButlerError::RubiesDirectoryNotFound(_)) => Vec::new(),
        Err(error) => anyhow::bail!(error.to_string()),
    };
    let selected = inventory
        .iter()
        .find(|ruby| request.matches(&ruby.version))
        .cloned();
    let version = if let Some(ruby) = &selected {
        Some(ruby.version.clone())
    } else if let Some(exact) = request.exact_version() {
        Some(exact)
    } else if resolve_ruby {
        let platform = rb_ruby_installer::Platform::with_inputs(
            context.process.directory(),
            context.process.environment(),
        )?;
        Some(
            crate::ruby_install::installer(context)?
                .catalog()?
                .select(&request, platform)?
                .version,
        )
    } else {
        None
    };
    let path = selected.as_ref().map(|ruby| ruby.root.clone()).or_else(|| {
        version
            .as_ref()
            .map(|version| directory.join(format!("ruby-{version}")))
    });
    let installed = path.as_ref().is_some_and(|path| {
        path.join(if cfg!(windows) {
            "bin/ruby.exe"
        } else {
            "bin/ruby"
        })
        .is_file()
    });
    let backend = PackageBackend::from_context(context);
    let project = backend.location().map(|location| DependencyProject {
        root: location.root,
        project_file: location.project_file,
        gemfile: location.gemfile,
    });
    let mut setup = EnvironmentReport {
        inspection: if probe { "dependencies" } else { "discovery" },
        environment: None,
        script_project: None,
        rb_version: env!("CARGO_PKG_VERSION"),
        working_directory: context.process.directory().to_owned(),
        backend: context.config().package_backend.get().to_string(),
        project,
        ruby: RubySelection {
            request: request.to_string(),
            version: version.map(|version| version.to_string()),
            path,
            installed,
        },
        cache: None,
    };
    if !probe {
        return Ok(setup);
    }
    let ruby = selected.context("Install the selected Ruby before probing dependency inputs")?;
    context.ruby = Some(SelectedRuby {
        runtime: ruby,
        installations: inventory,
    });
    let runtime = PackageRuntime::discover(context)?;
    setup.cache = backend.cache_inputs(&runtime)?;
    Ok(setup)
}
