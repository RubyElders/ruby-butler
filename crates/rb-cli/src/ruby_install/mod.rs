mod progress;
mod request;

use crate::{
    config::ConfigValue,
    reporting::{Completion, PreparationReport},
    runtime_helpers::CommandContext,
};
use anyhow::{Context, Result, bail};
use progress::InstallProgress;
use rb_core::{butler::ButlerError, ruby::RubyRuntimeDetector};
use rb_ruby_installer::{Installer, Platform, Request};

pub(crate) fn install(context: &CommandContext, version: Option<&str>) -> Result<(), ButlerError> {
    request::resolve(context, version)
        .and_then(|request| {
            install_inner(
                context,
                &request,
                Completion::Tree,
                crate::reporting::reporter,
            )
        })
        .map(|_| ())
        .map_err(error)
}

fn installer() -> Result<Installer> {
    Installer::new(crate::cache::directory()?.join("rubies"))
}

fn install_inner(
    context: &CommandContext,
    request: &Request,
    completion: Completion,
    make_reporter: impl FnOnce(String) -> rb_progress::Reporter,
) -> Result<semver::Version> {
    let platform = Platform::current().with_context(|| {
        format!(
            "Unable to install Ruby in {}",
            context.config.rubies_dir.get().display()
        )
    })?;
    let installer = installer()?;
    let mut progress = InstallProgress::new(make_reporter(format!("Installing Ruby {request}")));
    let result = installer.install(
        request,
        platform,
        context.config.rubies_dir.get(),
        |event| progress.event(event),
    );
    progress.finish(result, completion)
}

pub(crate) fn prepare(
    context: &mut CommandContext,
    report: &mut PreparationReport,
) -> Result<(), ButlerError> {
    prepare_inner(context, report).map_err(error)
}

fn prepare_inner(context: &mut CommandContext, report: &mut PreparationReport) -> Result<()> {
    let request = request::resolve(context, None)?;
    let directory = context.config.rubies_dir.get();
    let installed = match RubyRuntimeDetector::discover(directory) {
        Ok(rubies) => rubies,
        Err(rb_core::ruby::RubyDiscoveryError::DirectoryNotFound(_)) => Vec::new(),
        Err(error) => return Err(anyhow::anyhow!(error.to_string())),
    };
    let selected = installed
        .into_iter()
        .find(|ruby| request.matches(&ruby.version));
    let version = if let Some(ruby) = selected {
        ruby.version
    } else {
        if !*context.config.auto_install_ruby.get() {
            let argument = shlex::try_quote(&request.to_string())
                .context("Invalid Ruby request")?
                .into_owned();
            bail!(
                "Ruby {request} is not installed in {}. Run `rb install ruby {argument}` or enable --auto-install-ruby",
                directory.display()
            );
        }
        install_inner(context, &request, Completion::Tree, |title| {
            report.child(title)
        })?
    };
    let source = context
        .config
        .ruby_version
        .as_ref()
        .map(|value| value.source)
        .unwrap_or(crate::config::ConfigSource::Resolved);
    context.config.ruby_version = Some(ConfigValue::new(version.to_string(), source));
    Ok(())
}

pub(crate) fn available(context: &CommandContext) -> Result<(), ButlerError> {
    (|| -> Result<()> {
        let platform = Platform::current()?;
        for distribution in installer()?.catalog()?.distributions(platform) {
            let path = context
                .config
                .rubies_dir
                .get()
                .join(format!("ruby-{}", distribution.version));
            println!(
                "{}{}",
                distribution.version,
                if rb_ruby_installer::complete(&path) {
                    " (installed)"
                } else {
                    ""
                }
            );
        }
        Ok(())
    })()
    .map_err(error)
}

fn error(error: anyhow::Error) -> ButlerError {
    ButlerError::General(format!("{error:#}"))
}
