use crate::{
    PackagePlan, PackageSource, RubyEnvironment, source_index::SourceIndex,
    source_package::SourcePackage,
};
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn directory(package: &SourcePackage, home: &Path) -> Result<PathBuf> {
    Ok(match &package.record.source {
        PackageSource::Git {
            remote, revision, ..
        } => {
            let basename = if Path::new(remote).is_absolute() {
                Path::new(remote).file_name().unwrap().to_string_lossy()
            } else {
                std::borrow::Cow::Borrowed(
                    remote
                        .trim_end_matches('/')
                        .rsplit(['/', ':'])
                        .next()
                        .unwrap(),
                )
            };
            let base = basename.strip_suffix(".git").unwrap_or(&basename);
            ensure!(
                !base.is_empty()
                    && base
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')),
                "Nonportable Git repository name"
            );
            home.join(crate::constants::BUNDLER_DIRECTORY)
                .join(crate::constants::GEMS_DIRECTORY)
                .join(format!(
                    "{base}-{}",
                    &revision[..crate::constants::GIT_SHORT_REVISION_LENGTH]
                ))
        }
        PackageSource::Path { .. } => {
            let id = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&(
                    &package.record.source,
                    &package.snapshot
                ))?)
            );
            home.join(crate::constants::BUNDLER_DIRECTORY)
                .join(crate::constants::SOURCE_PATHS_DIRECTORY)
                .join(id)
        }
        PackageSource::Registry(_) => anyhow::bail!("Registry package has no source snapshot"),
    })
}

fn copy_snapshot(source: &Path, destination: &Path, files: &BTreeSet<PathBuf>) -> Result<()> {
    fs::create_dir_all(destination)?;
    for relative in files {
        let from = source.join(relative);
        let to = destination.join(relative);
        fs::create_dir_all(to.parent().unwrap())?;
        if fs::symlink_metadata(&from)?.file_type().is_symlink() {
            let target = fs::read_link(&from)?;
            ensure!(!target.is_absolute(), "Source links must be relative");
            #[cfg(unix)]
            std::os::unix::fs::symlink(target, to)?;
            #[cfg(windows)]
            {
                if from.is_dir() {
                    std::os::windows::fs::symlink_dir(target, to)?;
                } else {
                    std::os::windows::fs::symlink_file(target, to)?;
                }
            }
        } else {
            fs::copy(from, to)?;
        }
    }
    Ok(())
}

pub(crate) fn stage(
    index: &SourceIndex,
    home: &Path,
    plan: &PackagePlan,
    runtime: &RubyEnvironment,
) -> Result<bool> {
    let mut copied = BTreeSet::new();
    let mut extension_targets = BTreeSet::new();
    let mut changed = false;
    for planned in plan
        .packages
        .iter()
        .filter(|p| !matches!(p.record.source, PackageSource::Registry(_)))
    {
        let package = index.local(&planned.record)?;
        let destination = directory(package, home)?;
        if copied.insert(destination.clone()) {
            let stamp = destination.join(crate::constants::SOURCE_STAMP_FILE);
            let files: BTreeSet<_> = index
                .packages
                .values()
                .flatten()
                .filter(|entry| entry.record.source == package.record.source)
                .flat_map(|entry| {
                    entry
                        .payload
                        .iter()
                        .map(|path| entry.gemspec.parent().unwrap().join(path))
                })
                .collect();
            let complete = files.iter().all(|path| destination.join(path).exists());
            if !complete || fs::read_to_string(&stamp).ok().as_deref() != Some(&package.snapshot) {
                if destination.exists() {
                    fs::remove_dir_all(&destination)?;
                }
                copy_snapshot(&package.directory, &destination, &files)?;
                fs::write(&stamp, &package.snapshot)?;
                changed = true;
            }
        }
        let spec = destination.join(&package.gemspec);
        if fs::read_to_string(&spec).ok().as_deref() != Some(&package.serialized) {
            rb_gemfile::write_project(&spec, &package.serialized)?;
            changed = true;
        }
        let full_name = format!(
            "{}-{}",
            planned.record.package.id.name, planned.record.archive_version
        );
        if !planned.install {
            continue;
        }
        let installed = home.join(crate::constants::GEMS_DIRECTORY).join(&full_name);
        let receipt = fs::read(installed.join(crate::constants::INSTALL_RECEIPT))?;
        let metadata: crate::receipt::InstalledReceipt = serde_json::from_slice(&receipt)?;
        if let Some(extension) = &metadata.extension {
            let gem_directory = destination.join(package.gemspec.parent().unwrap());
            for relative in &metadata.build_artifacts {
                let target = gem_directory.join(relative);
                fs::create_dir_all(target.parent().unwrap())?;
                fs::copy(installed.join(relative), target)?;
            }
            let source_extensions = home.join(extension);
            let unique = if matches!(package.record.source, PackageSource::Git { .. }) {
                let scope = destination.file_name().unwrap().to_string_lossy();
                let name = gem_directory.file_name().unwrap().to_string_lossy();
                if scope == name {
                    scope.into_owned()
                } else {
                    format!("{scope}-{name}")
                }
            } else {
                full_name.clone()
            };
            let target = gem_directory
                .parent()
                .unwrap()
                .join(crate::constants::EXTENSIONS_DIRECTORY)
                .join(&runtime.platform)
                .join(&runtime.abi)
                .join(unique);
            if extension_targets.insert(target.clone()) && target.exists() {
                fs::remove_dir_all(&target)?;
            }
            crate::copy_tree::copy_tree(&source_extensions, &target)?;
            changed = true;
        }
    }
    Ok(changed)
}
