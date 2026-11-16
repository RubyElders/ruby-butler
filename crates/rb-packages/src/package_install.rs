use crate::{InstallOptions, RubyEnvironment, slot::Slots};
use anyhow::{Context, Result, ensure};
use rb_task::TaskContext;
use std::path::Path;

pub(crate) fn install_slot(
    slots: &Slots,
    name: &str,
    home: &Path,
    build_directory: &Path,
    runtime: &RubyEnvironment,
    options: &InstallOptions,
    context: &TaskContext,
) -> Result<()> {
    let mut slot = slots[name].lock().unwrap();
    if !slot.reused {
        let metadata = slot.metadata.as_ref().unwrap();
        let mut dependencies = Vec::new();
        let mut markers = Vec::new();
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
            let mut gem = other
                .gem
                .clone()
                .context("Build dependency is incomplete")?;
            gem.root =
                crate::build_dependencies::stage(build_directory, &gem, &other.receipt, runtime)?;
            markers.push(gem.root.join(crate::constants::INSTALL_RECEIPT));
            dependencies.push(gem);
        }
        if crate::build_tools::requires_rake(metadata)
            && !dependencies.iter().any(|gem| gem.name == "rake")
        {
            dependencies.push(runtime.build_tools.iter().find(|gem| gem.name == "rake")
                .context("Rakefile extension requires rake; provide a build dependency or install rake with the selected Ruby")?.clone());
        }
        slot.receipt.build_directory = (!markers.is_empty()).then(|| build_directory.to_owned());
        if let Some(cache) = &options.cache {
            let directory = slot.directory.clone();
            let full_name = directory.file_name().unwrap().to_str().unwrap();
            context.progress("checking shared cache", 0, 1, Some(name.into()));
            let entry =
                crate::installation_cache::Entry::open(cache, &slot.receipt, full_name, runtime)?;
            let record = slot.record.clone();
            if !entry.restore(&mut slot, home, &record, runtime)? {
                entry.begin()?;
                let root = entry.gem();
                crate::copy_tree::copy_tree(
                    &slot.staging.as_ref().unwrap().path().join("gem"),
                    &root,
                )?;
                slot.directory = root.clone();
                crate::package_build::build(
                    &mut slot,
                    &root,
                    &entry.home(),
                    runtime,
                    options,
                    dependencies,
                    context,
                )?;
                entry.publish(slot.metadata.as_ref().unwrap(), markers)?;
                slot.directory = directory;
                ensure!(
                    entry.restore(&mut slot, home, &record, runtime)?,
                    "Completed installation cache is missing"
                );
                slot.reused = false;
            }
        } else {
            let root = slot.staging.as_ref().unwrap().path().join("gem");
            crate::package_build::build(
                &mut slot,
                &root,
                home,
                runtime,
                options,
                dependencies,
                context,
            )?;
        }
    }
    context.progress(
        if slot.reused && slot.restored {
            "reuse"
        } else {
            "install"
        },
        1,
        1,
        Some(name.into()),
    );
    Ok(())
}
