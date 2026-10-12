use crate::{
    InstallOptions, RubyEnvironment, constants::INSTALL_RECEIPT, receipt::InstalledReceipt,
    slot::Slots, wrappers::wrappers,
};
use anyhow::{Context, Result, ensure};
use rb_gem_installer::{BuildEvent, NativeBuilder};
use rb_task::TaskContext;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn install_slot(
    slots: &Slots,
    name: &str,
    home: &Path,
    runtime: &RubyEnvironment,
    options: &InstallOptions,
    context: &TaskContext,
) -> Result<()> {
    let mut slot = slots[name].lock().unwrap();
    if !slot.reused {
        let root = slot.staging.as_ref().unwrap().path().join("gem");
        let metadata = slot.metadata.as_ref().unwrap();
        let payload_files = crate::source_files::payload_files(&root)?;
        ensure!(
            metadata.rubygems.matches(&runtime.rubygems),
            "Gem requires RubyGems {}",
            metadata.rubygems
        );
        let mut build = options.build.clone();
        build.ruby = runtime.executable.clone();
        for dependency in slot
            .dependencies
            .iter()
            .filter(|_| !metadata.extensions.is_empty())
        {
            let other = slots
                .get(dependency)
                .context("Missing build dependency")?
                .lock()
                .unwrap();
            if !metadata.extensions.is_empty() {
                build.dependencies.push(
                    other
                        .gem
                        .clone()
                        .context("Build dependency is incomplete")?,
                );
            }
        }
        let report = if metadata.extensions.is_empty() {
            None
        } else {
            let mut stdout = crate::output_lines::OutputLines::default();
            let mut stderr = crate::output_lines::OutputLines::default();
            let result = NativeBuilder::new(build)?.build(metadata, &root, |event| {
                if let BuildEvent::Output { bytes, stream, .. } = event {
                    let lines = if stream == rb_gem_installer::OutputStream::Stdout {
                        &mut stdout
                    } else {
                        &mut stderr
                    };
                    lines.push(&bytes, |line| context.output(line));
                }
            });
            stdout.finish(|line| context.output(line));
            stderr.finish(|line| context.output(line));
            Some(result?)
        };
        let artifacts = report
            .as_ref()
            .map(|report| report.artifacts.as_slice())
            .unwrap_or(&[]);
        let staged_home = slot.staging.as_ref().unwrap().path();
        let specification = crate::specification::write(
            &runtime.executable,
            metadata,
            &slot.receipt.archive_version,
            staged_home,
        )?;
        let extension_name = format!("{}-{}", metadata.id.name, slot.receipt.archive_version);
        let extension = staged_home
            .join(crate::constants::EXTENSIONS_DIRECTORY)
            .join(&runtime.platform)
            .join(&runtime.abi)
            .join(&extension_name);
        if !metadata.extensions.is_empty() {
            fs::create_dir_all(&extension)?;
            for artifact in artifacts {
                if let Some(relative) = report
                    .as_ref()
                    .and_then(|report| report.extension_directory.as_ref())
                    .and_then(|directory| artifact.strip_prefix(directory).ok())
                {
                    let target = extension.join(relative);
                    fs::create_dir_all(target.parent().unwrap())?;
                    fs::copy(root.join(artifact), target)?;
                }
            }
            fs::write(extension.join(crate::constants::BUILD_COMPLETE_MARKER), b"")?;
        }
        let gem = installed_gem(metadata, &slot.directory);
        fs::write(
            root.join(INSTALL_RECEIPT),
            serde_json::to_vec(&InstalledReceipt {
                receipt: slot.receipt.clone(),
                gem: gem.clone(),
                files: payload_files
                    .into_iter()
                    .chain(artifacts.iter().cloned())
                    .collect(),
                build_artifacts: artifacts.to_vec(),
                extension: (!metadata.extensions.is_empty()).then(|| {
                    PathBuf::from(crate::constants::EXTENSIONS_DIRECTORY)
                        .join(&runtime.platform)
                        .join(&runtime.abi)
                        .join(&extension_name)
                }),
            })?,
        )?;
        fs::create_dir_all(home.join(crate::constants::GEMS_DIRECTORY))?;
        fs::create_dir_all(home.join(crate::constants::SPECIFICATIONS_DIRECTORY))?;
        fs::rename(&root, &slot.directory)?;
        if extension.exists() {
            let destination = home
                .join(crate::constants::EXTENSIONS_DIRECTORY)
                .join(&runtime.platform)
                .join(&runtime.abi)
                .join(&extension_name);
            fs::create_dir_all(destination.parent().unwrap())?;
            if destination.exists() {
                fs::remove_dir_all(&destination)?;
            }
            fs::rename(extension, destination)?;
        }
        let destination_spec = home
            .join(crate::constants::SPECIFICATIONS_DIRECTORY)
            .join(specification.file_name().unwrap());
        rb_gemfile::write_project(destination_spec, &fs::read_to_string(specification)?)?;
        wrappers(home, metadata)?;
        slot.gem = Some(gem);
        slot.staging = None;
    }
    context.progress("install", 1, 1, Some(name.into()));
    Ok(())
}

fn installed_gem(
    metadata: &rb_gem_archive::GemMetadata,
    root: &Path,
) -> rb_microgems::InstalledGem {
    rb_microgems::InstalledGem {
        name: metadata.id.name.clone(),
        version: metadata.id.version.to_string(),
        platform: metadata.id.platform.to_string(),
        root: root.into(),
        require_paths: metadata.require_paths.clone(),
        metadata: metadata.metadata.clone(),
        executables: metadata.executables.clone(),
        bindir: metadata.bindir.clone(),
    }
}
