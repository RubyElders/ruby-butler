use crate::{
    LockMode, PackageIndex, PackageRecord, PackageSource, ProjectInput, RegistryIndex,
    RubyEnvironment, source_package::SourcePackage,
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct SourceIndex {
    registry: RegistryIndex,
    pub(crate) packages: BTreeMap<String, Vec<SourcePackage>>,
    roots: BTreeMap<String, rb_gemfile::Dependency>,
    affinity: std::sync::Mutex<BTreeMap<String, PackageSource>>,
}

impl SourceIndex {
    pub fn new(
        input: &ProjectInput,
        runtime: &RubyEnvironment,
        cache: &Path,
        offline: bool,
        previous: Option<&rb_lockfile::Lockfile>,
        mode: LockMode,
    ) -> Result<Self> {
        ensure!(
            input.manifest.dependencies.values().all(|dependency| {
                std::iter::once(dependency)
                    .chain(&dependency.variants)
                    .all(|declaration| {
                        !declaration.enabled
                            || declaration.platforms.is_empty()
                            || (declaration.git.is_none() && declaration.path.is_none())
                    })
            }),
            "Platform-specific Git/path sources require multi-target source routing"
        );
        let mut packages: BTreeMap<String, Vec<SourcePackage>> = BTreeMap::new();
        let mut seen = std::collections::BTreeSet::new();
        for declaration in input
            .manifest
            .dependencies
            .values()
            .flat_map(|d| std::iter::once(d).chain(&d.variants))
            .filter(|d| d.enabled && (d.git.is_some() || d.path.is_some()))
        {
            let key = serde_json::to_string(
                &serde_json::json!({"git":declaration.git,"path":declaration.path,"options":crate::package_source::git_options(declaration)}),
            )?;
            if !seen.insert(key) {
                continue;
            }
            let (source, directory) = if let Some(remote) = &declaration.git {
                let options = crate::package_source::git_options(declaration);
                let pinned = if mode == LockMode::Update {
                    None
                } else {
                    previous
                        .and_then(|lock| {
                            lock.sources.iter().find(|s| {
                                let mut old = s.options.clone();
                                old.remove("revision");
                                if old.get("glob").is_some_and(|value| {
                                    value == crate::constants::DEFAULT_GEMSPEC_GLOB
                                }) {
                                    old.remove("glob");
                                }
                                s.kind == rb_lockfile::SourceKind::Git
                                    && s.remotes.iter().any(|previous| {
                                        previous.trim_end_matches('/')
                                            == remote.trim_end_matches('/')
                                    })
                                    && old == options
                            })
                        })
                        .and_then(|s| s.options.get("revision"))
                        .map(String::as_str)
                };
                let (revision, directory) = crate::git_cache::checkout(
                    &input.root,
                    remote,
                    &options,
                    cache,
                    pinned,
                    offline,
                )?;
                (
                    PackageSource::Git {
                        remote: remote.trim_end_matches('/').into(),
                        revision,
                        options,
                    },
                    directory,
                )
            } else {
                let directory =
                    dunce::canonicalize(input.root.join(declaration.path.as_ref().unwrap()))?;
                (
                    PackageSource::Path {
                        directory: directory.clone(),
                        glob: crate::package_source::normalized_glob(declaration.glob.as_deref()),
                    },
                    directory,
                )
            };
            let snapshot = crate::source_files::fingerprint(&directory)?;
            let specifications = crate::source_specs::read(
                &runtime.executable,
                &directory,
                declaration.glob.as_deref(),
            )?;
            ensure!(
                crate::source_files::fingerprint(&directory)? == snapshot,
                "Gemspec evaluation modified its source directory"
            );
            for spec in specifications {
                let package = SourcePackage::new(spec, &source, &directory, cache, &snapshot)?;
                packages
                    .entry(package.record.package.id.name.clone())
                    .or_default()
                    .push(package);
            }
        }
        let mut registry = RegistryIndex::new(&input.manifest, cache, offline)?;
        registry.load_credentials(&input.root, offline)?;
        Ok(Self {
            registry,
            packages,
            roots: input
                .manifest
                .dependencies
                .iter()
                .filter_map(|(name, d)| {
                    std::iter::once(d)
                        .chain(&d.variants)
                        .find(|d| d.enabled)
                        .map(|d| (name.clone(), d.clone()))
                })
                .collect(),
            affinity: std::sync::Mutex::new(BTreeMap::new()),
        })
    }

    pub(crate) fn local(&self, record: &PackageRecord) -> Result<&SourcePackage> {
        self.packages
            .get(&record.package.id.name)
            .and_then(|entries| entries.iter().find(|entry| entry.record == *record))
            .context("Source package is unavailable")
    }
}

impl PackageIndex for SourceIndex {
    fn candidates(&self, name: &str) -> Result<Vec<PackageRecord>> {
        let records = if let Some(root) = self.roots.get(name) {
            if root.git.is_none() && root.path.is_none() {
                return self.registry.candidates(name);
            }
            self.packages
                .get(name)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|p| match p.record.source.matches(root, None) {
                            Ok(true) => Some(Ok(p.record.clone())),
                            Ok(false) => None,
                            Err(error) => Some(Err(error)),
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?
                .unwrap_or_default()
        } else if let Some(source) = self.affinity.lock().unwrap().get(name).cloned() {
            self.packages
                .get(name)
                .map(|entries| {
                    entries
                        .iter()
                        .filter(|p| p.record.source == source)
                        .map(|p| p.record.clone())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            return self.registry.candidates(name);
        };
        let mut affinity = self.affinity.lock().unwrap();
        for record in &records {
            for dependency in record
                .package
                .dependencies
                .keys()
                .filter(|name| !self.roots.contains_key(*name))
            {
                if self
                    .packages
                    .get(dependency)
                    .is_some_and(|entries| entries.iter().any(|p| p.record.source == record.source))
                {
                    if let Some(previous) = affinity.get(dependency) {
                        ensure!(
                            previous == &record.source,
                            "Ambiguous source for {dependency}"
                        );
                    }
                    affinity.insert(dependency.clone(), record.source.clone());
                }
            }
        }
        Ok(records)
    }
    fn archive(&self, record: &PackageRecord) -> Result<PathBuf> {
        if matches!(record.source, PackageSource::Registry(_)) {
            self.registry.archive(record)
        } else {
            Ok(self.local(record)?.archive.clone())
        }
    }
    fn stage_sources(
        &self,
        home: &Path,
        plan: &crate::PackagePlan,
        runtime: &RubyEnvironment,
    ) -> Result<bool> {
        crate::source_install::stage(self, home, plan, runtime)
    }
    fn compatibility_path(&self, record: &PackageRecord, home: &Path) -> Result<Option<PathBuf>> {
        if matches!(record.source, PackageSource::Path { .. }) {
            Ok(Some(crate::source_install::directory(
                self.local(record)?,
                home,
            )?))
        } else {
            Ok(None)
        }
    }
}
