mod progress;
pub(crate) mod request;

use crate::invocation::SelectedRuby;
use crate::{
    reporting::{Completion, PreparationReport},
    runtime_helpers::CommandContext,
};
use anyhow::{Context, Result, bail};
use progress::InstallProgress;
use rb_core::butler::ButlerError;
use rb_core::ruby::{RubyRuntime, RubyType};
use rb_ruby_installer::{Installation, Installer, Platform, Request};

#[derive(serde::Serialize)]
struct InstallationReport {
    version: String,
    path: std::path::PathBuf,
    installed: bool,
}

pub(crate) fn install(context: &CommandContext, version: Option<&str>) -> Result<(), ButlerError> {
    let installation = request::resolve(context, version)
        .and_then(|request| {
            install_inner(
                context,
                &request,
                Completion::Tree,
                crate::reporting::reporter,
            )
        })
        .map_err(error)?;
    let report = InstallationReport {
        version: installation.version.to_string(),
        path: installation.path,
        installed: installation.installed,
    };
    context.output.emit(
        "install.ruby",
        &report,
        crate::output::document::Document::default,
    )
}

fn installer(context: &CommandContext) -> Result<Installer> {
    Installer::with_inputs(
        crate::cache::directory(&context.process)?.join("rubies"),
        context.process.directory().to_owned(),
        context.process.environment().clone(),
    )
}

fn install_inner(
    context: &CommandContext,
    request: &Request,
    completion: Completion,
    make_reporter: impl FnOnce(String) -> rb_progress::Reporter,
) -> Result<Installation> {
    let platform =
        Platform::with_inputs(context.process.directory(), context.process.environment())
            .with_context(|| {
                format!(
                    "Unable to install Ruby in {}",
                    context.config().rubies_dir.get().display()
                )
            })?;
    let installer = installer(context)?;
    let mut progress = InstallProgress::new(make_reporter(format!("Installing Ruby {request}")));
    let result = installer.install(
        request,
        platform,
        &context
            .process
            .directory()
            .join(context.config().rubies_dir.get()),
        |event| progress.event(event),
    );
    progress.finish(result, completion)
}

pub(crate) fn prepare(
    context: &CommandContext,
    report: &mut PreparationReport,
) -> Result<SelectedRuby, ButlerError> {
    prepare_inner(context, report).map_err(error)
}

fn prepare_inner(context: &CommandContext, report: &mut PreparationReport) -> Result<SelectedRuby> {
    let request = request::resolve(context, None)?;
    let directory = context
        .process
        .directory()
        .join(context.config().rubies_dir.get());
    let mut installed = match context.ruby_inventory() {
        Ok(rubies) => rubies.clone(),
        Err(ButlerError::RubiesDirectoryNotFound(_)) => Vec::new(),
        Err(error) => return Err(anyhow::anyhow!(error.to_string())),
    };
    let selected = installed
        .iter()
        .find(|ruby| request.matches(&ruby.version))
        .cloned();
    let runtime = if let Some(ruby) = selected {
        ruby
    } else {
        if !*context.config().auto_install_ruby.get() {
            let argument = shlex::try_quote(&request.to_string())
                .context("Invalid Ruby request")?
                .into_owned();
            bail!(
                "Ruby {request} is not installed in {}. Run `rb install ruby {argument}` or enable --auto-install-ruby",
                directory.display()
            );
        }
        let installation = install_inner(context, &request, Completion::Tree, |title| {
            report.child(title)
        })?;
        let ruby = RubyRuntime::new(RubyType::CRuby, installation.version, installation.path);
        installed.push(ruby.clone());
        ruby
    };
    Ok(SelectedRuby {
        runtime,
        installations: installed,
    })
}

pub(crate) fn available(context: &CommandContext) -> Result<(), ButlerError> {
    (|| -> Result<()> {
        let platform =
            Platform::with_inputs(context.process.directory(), context.process.environment())?;
        let rubies = installer(context)?
            .catalog()?
            .distributions(platform)
            .into_iter()
            .map(|distribution| {
                let path = context.process.directory().join(
                    context
                        .config()
                        .rubies_dir
                        .get()
                        .join(format!("ruby-{}", distribution.version)),
                );
                crate::commands::info::runtime::AvailableRuby {
                    version: distribution.version.to_string(),
                    installed: rb_ruby_installer::complete(&path),
                }
            })
            .collect::<Vec<_>>();
        context.output.emit("info.runtime.available", &rubies, || {
            crate::presentation::info::runtime::available(&rubies)
        })?;
        Ok(())
    })()
    .map_err(error)
}

fn error(error: anyhow::Error) -> ButlerError {
    ButlerError::General(format!("{error:#}"))
}
