use crate::{
    InstallOptions, InstalledBundle, PackageIndex, PackagePlan, PackageRecord, RubyEnvironment,
    constants::{GEM_HOME_DIRECTORY, INSTALL_RECEIPT, STATE_DIRECTORY},
    copy_tree::copy_tree,
    package_install::install_slot,
    receipt::{InstalledReceipt, Receipt},
    slot::Slot,
    validation::{ensure_record, verify},
};
use anyhow::{Context, Result, ensure};
use rb_task::{Executor, TaskAction, TaskEvents, TaskGraph};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

pub fn install(
    project: &Path,
    plan: &PackagePlan,
    runtime: &RubyEnvironment,
    index: Arc<dyn PackageIndex>,
    options: InstallOptions,
    events: TaskEvents,
) -> Result<InstalledBundle> {
    let project = project.to_owned();
    let init_plan = plan.clone();
    let init_runtime = runtime.clone();
    let (bundle, home, lockfile, transaction, staged_home, runtime, options, slots) =
        crate::task::run("prepare installation", events.clone(), move |_| {
            let project = &project;
            let plan = &init_plan;
            let runtime = &init_runtime;
            let options = options.clone();
            ensure!(options.workers > 0, "Worker count must be positive");
            let bundle = project.join(STATE_DIRECTORY).join(GEM_HOME_DIRECTORY);
            let home = bundle.join(&runtime.engine).join(
                runtime
                    .version
                    .to_string()
                    .split('.')
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(".")
                    + ".0",
            );
            fs::create_dir_all(home.parent().context("Missing gem home parent")?)?;
            let lockfile = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(home.parent().unwrap().join(crate::constants::INSTALL_LOCK))?;
            lockfile.lock()?;
            let transaction = tempfile::Builder::new()
                .prefix(".bundle-")
                .tempdir_in(home.parent().unwrap())?;
            let staged_home = transaction.path().join("home");
            if home.exists() {
                copy_tree(&home, &staged_home)?;
            } else {
                fs::create_dir_all(&staged_home)?;
            }
            let staged_home = Arc::new(staged_home);
            let runtime = Arc::new(runtime.clone());
            let options = Arc::new(options);
            let records: BTreeMap<_, _> = plan
                .packages
                .iter()
                .filter(|p| p.install)
                .map(|p| (p.record.package.id.name.clone(), p.record.clone()))
                .collect();
            let tools: Vec<_> = records
                .get("rake")
                .map(|rake| {
                    std::iter::once(rake)
                        .chain(
                            dependencies(&records, "rake")
                                .into_iter()
                                .map(|name| &records[&name]),
                        )
                        .collect()
                })
                .unwrap_or_default();
            let mut slots = BTreeMap::new();
            for planned in plan.packages.iter().filter(|p| p.install) {
                let record = &planned.record;
                record.validate()?;
                let mut receipt = Receipt::new(record, &runtime, &options.build, &tools)?;
                receipt.dependencies = dependencies(&records, &record.package.id.name)
                    .into_iter()
                    .map(|name| {
                        let dependency = &records[&name];
                        (
                            name,
                            (
                                dependency.source.clone(),
                                dependency.archive_version.clone(),
                                dependency.checksum.clone(),
                            ),
                        )
                    })
                    .collect();
                let directory = staged_home
                    .join(crate::constants::GEMS_DIRECTORY)
                    .join(format!(
                        "{}-{}",
                        record.package.id.name, record.archive_version
                    ));
                let registered = fs::read(directory.join(INSTALL_RECEIPT))
                    .ok()
                    .and_then(|b| serde_json::from_slice::<InstalledReceipt>(&b).ok());
                let reused = registered.as_ref().is_some_and(|previous| {
                    previous.receipt == receipt
                        && previous
                            .files
                            .iter()
                            .all(|path| fs::symlink_metadata(directory.join(path)).is_ok())
                        && previous.extension.as_ref().is_none_or(|path| {
                            staged_home
                                .join(path)
                                .join(crate::constants::BUILD_COMPLETE_MARKER)
                                .is_file()
                        })
                }) && staged_home
                    .join(crate::constants::SPECIFICATIONS_DIRECTORY)
                    .join(format!(
                        "{}-{}.gemspec",
                        record.package.id.name, record.archive_version
                    ))
                    .is_file();
                if directory.exists() && !reused {
                    fs::remove_dir_all(&directory)?;
                }
                let gem = if reused {
                    registered.map(|mut r| {
                        r.gem.root = directory.clone();
                        r.gem
                    })
                } else {
                    None
                };
                slots.insert(
                    record.package.id.name.clone(),
                    Arc::new(Mutex::new(Slot {
                        directory,
                        archive: None,
                        metadata: None,
                        staging: None,
                        reused,
                        receipt,
                        gem,
                        dependencies: dependencies(&records, &record.package.id.name),
                    })),
                );
            }
            let slots = Arc::new(slots);
            Ok((
                bundle,
                home,
                lockfile,
                transaction,
                staged_home,
                runtime,
                options,
                slots,
            ))
        })?;
    let _installation_lock = lockfile;
    let mut graph = TaskGraph::new();
    let mut actions: BTreeMap<_, TaskAction> = BTreeMap::new();
    for planned in plan.packages.iter().filter(|p| p.install) {
        let record = planned.record.clone();
        let name = record.package.id.name.clone();
        let download = graph.add(format!("download {}", record.package.id), []);
        let extract = graph.add(format!("extract {}", record.package.id), [download]);
        let index = index.clone();
        let slot_download = slots[&name].clone();
        let download_record = record.clone();
        actions.insert(
            download,
            Arc::new(move |context| {
                let mut slot = slot_download.lock().unwrap();
                if !slot.reused {
                    let path = index
                        .archive(&download_record)
                        .map_err(|e| format!("{e:#}"))?;
                    verify(&path, &download_record.checksum).map_err(|e| format!("{e:#}"))?;
                    slot.archive = Some(path);
                }
                context.progress(
                    "download",
                    1,
                    1,
                    Some(if slot.reused {
                        "reused".into()
                    } else {
                        download_record.package.id.to_string()
                    }),
                );
                Ok(())
            }),
        );
        let slot_extract = slots[&name].clone();
        let home_extract = staged_home.clone();
        actions.insert(
            extract,
            Arc::new(move |context| {
                let mut slot = slot_extract.lock().unwrap();
                if slot.reused {
                    return Ok(());
                }
                let staging = tempfile::Builder::new()
                    .prefix(".package-")
                    .tempdir_in(home_extract.as_path())
                    .map_err(|e| e.to_string())?;
                let directory = staging.path().join("gem");
                let metadata = rb_gem_installer::install(
                    fs::File::open(slot.archive.as_ref().unwrap()).map_err(|e| e.to_string())?,
                    &directory,
                )
                .map_err(|e| format!("{e:#}"))?;
                ensure_record(&metadata, &record).map_err(|e| format!("{e:#}"))?;
                slot.staging = Some(staging);
                slot.metadata = Some(metadata);
                context.progress("extract", 1, 1, Some(name.clone()));
                Ok(())
            }),
        );
    }
    Executor::new(options.workers, events.clone()).run(graph, actions)?;
    let mut graph = TaskGraph::new();
    let mut actions: BTreeMap<_, TaskAction> = BTreeMap::new();
    if let Some(rake) = slots.get("rake") {
        let dependencies = rake.lock().unwrap().dependencies.clone();
        for (name, slot) in slots.iter() {
            let mut slot = slot.lock().unwrap();
            if name != "rake"
                && slot
                    .metadata
                    .as_ref()
                    .is_some_and(crate::build_tools::requires_rake)
            {
                slot.dependencies.insert("rake".into());
                slot.dependencies.extend(dependencies.iter().cloned());
            }
        }
    }
    let mut pending: BTreeMap<_, _> = plan
        .packages
        .iter()
        .filter(|p| p.install)
        .map(|p| (p.record.package.id.name.clone(), p.record.clone()))
        .collect();
    let mut completed = BTreeMap::new();
    while !pending.is_empty() {
        let ready: Vec<_> = pending
            .iter()
            .filter(|(name, _)| {
                let slot = slots[*name].lock().unwrap();
                slot.reused
                    || slot
                        .metadata
                        .as_ref()
                        .is_some_and(|m| m.extensions.is_empty())
                    || slot.dependencies.iter().all(|d| completed.contains_key(d))
            })
            .map(|(name, _)| name.clone())
            .collect();
        ensure!(
            !ready.is_empty(),
            "Native build dependency cycle: {}",
            pending.keys().cloned().collect::<Vec<_>>().join(", ")
        );
        for name in ready {
            let record = pending.remove(&name).unwrap();
            let dependency_ids: Vec<_> = {
                let slot = slots[&name].lock().unwrap();
                if slot.reused
                    || slot
                        .metadata
                        .as_ref()
                        .is_some_and(|m| m.extensions.is_empty())
                {
                    Vec::new()
                } else {
                    slot.dependencies
                        .iter()
                        .filter_map(|name| completed.get(name).copied())
                        .collect()
                }
            };
            let id = graph.add(format!("install {}", record.package.id), dependency_ids);
            completed.insert(name.clone(), id);
            let slots = slots.clone();
            let home = staged_home.clone();
            let runtime = runtime.clone();
            let options = options.clone();
            actions.insert(
                id,
                Arc::new(move |context| {
                    install_slot(&slots, &name, &home, &runtime, &options, &context)
                        .map_err(|e| format!("{e:#}"))
                }),
            );
        }
    }
    Executor::new(options.workers, events.clone()).run(graph, actions)?;
    let mut reused = 0;
    for slot in slots.values() {
        if slot.lock().unwrap().reused {
            reused += 1;
        }
    }
    let staged = staged_home.as_ref().clone();
    let destination = home.clone();
    let transaction_path = transaction.path().to_owned();
    let source_index = index.clone();
    let source_home = staged_home.as_ref().clone();
    let source_plan = plan.clone();
    let source_runtime = runtime.clone();
    let sources_changed =
        crate::task::run("prepare source snapshots", events.clone(), move |_| {
            source_index.stage_sources(&source_home, &source_plan, &source_runtime)
        })?;
    if reused != slots.len() || sources_changed {
        crate::task::run("publish bundle", events, move |_| {
            let backup = transaction_path.join("previous");
            if destination.exists() {
                fs::rename(&destination, &backup)?;
            }
            if let Err(error) = fs::rename(&staged, &destination) {
                if backup.exists() {
                    fs::rename(backup, &destination)?;
                }
                return Err(error.into());
            }
            Ok(())
        })?;
    }
    Ok(InstalledBundle {
        gem_home: home,
        bundle_path: bundle,
        installed: slots.len() - reused,
        reused,
        without: plan.without.iter().cloned().collect(),
        with: plan.with.iter().cloned().collect(),
        gemfile: None,
    })
}

fn dependencies(
    records: &BTreeMap<String, PackageRecord>,
    name: &str,
) -> std::collections::BTreeSet<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut pending: Vec<_> = records[name].package.dependencies.keys().cloned().collect();
    while let Some(dependency) = pending.pop() {
        if dependency != name
            && seen.insert(dependency.clone())
            && let Some(record) = records.get(&dependency)
        {
            pending.extend(record.package.dependencies.keys().cloned());
        }
    }
    seen
}
