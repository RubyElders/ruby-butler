use crate::{PackageIndex, PackageRecord, RubyEnvironment, provider::source_url};
use anyhow::{Context, Result, ensure};
use rb_gem_types::{PackageId, Platform, Requirement};
use rb_lockfile::{LockedPackage, Lockfile, Source, SourceKind};
use rb_resolver::{Package, PackageProvider, ResolveRequest};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LockMode {
    #[default]
    Prefer,
    Locked,
    Update,
}

#[derive(Clone, Debug, Default)]
pub struct PlanOptions {
    pub mode: LockMode,
    pub without: BTreeSet<String>,
    pub with: BTreeSet<String>,
    pub allow_prereleases: bool,
    pub source_root: Option<std::path::PathBuf>,
}

#[derive(Clone, Debug)]
pub struct PlannedPackage {
    pub record: PackageRecord,
    pub install: bool,
}

#[derive(Clone, Debug)]
pub struct PackagePlan {
    pub packages: Vec<PlannedPackage>,
    pub lockfile: Lockfile,
    pub lock_changed: bool,
    pub without: BTreeSet<String>,
    pub with: BTreeSet<String>,
}

struct Provider<'a> {
    index: &'a dyn PackageIndex,
    records: Mutex<BTreeMap<String, Vec<PackageRecord>>>,
    sources: BTreeMap<String, rb_gemfile::Dependency>,
    default_source: Option<String>,
    pins: BTreeMap<String, Vec<crate::source_pin::SourcePin>>,
    strict: bool,
    runtime: &'a RubyEnvironment,
}

impl PackageProvider for Provider<'_> {
    type Error = anyhow::Error;
    fn candidates(&self, name: &str) -> Result<Vec<Package>> {
        if self.strict && !self.pins.contains_key(name) {
            return Ok(Vec::new());
        }
        let mut records = self.index.candidates(name)?;
        for record in &records {
            record.validate()?;
        }
        records.retain(|record| {
            record
                .rubygems
                .as_ref()
                .is_none_or(|requirement| requirement.matches(&self.runtime.rubygems))
                && (name != "bundler"
                    || self
                        .runtime
                        .bundler
                        .as_ref()
                        .is_none_or(|version| &record.package.id.version == version))
        });
        if let Some(source) = self.sources.get(name) {
            records = records
                .into_iter()
                .filter_map(|record| {
                    match record
                        .source
                        .matches(source, self.default_source.as_deref())
                    {
                        Ok(true) => Some(Ok(record)),
                        Ok(false) => None,
                        Err(error) => Some(Err(error)),
                    }
                })
                .collect::<Result<_>>()?;
        }
        if self.strict {
            records.retain(|record| {
                self.pins
                    .get(name)
                    .is_some_and(|pins| pins.iter().any(|pin| pin.matches(record)))
            });
            for record in &records {
                ensure!(
                    self.pins[name]
                        .iter()
                        .any(|pin| pin.matches(record) && pin.dependencies_match(record)),
                    "Locked dependency metadata changed for {}",
                    record.package.id
                );
            }
        }
        let mut identities = BTreeMap::new();
        for record in &records {
            if let Some(previous) = identities.insert(record.package.id.clone(), record) {
                ensure!(
                    previous == record,
                    "Ambiguous source or metadata for {}",
                    record.package.id
                );
            }
        }
        let packages = records.iter().map(|r| r.package.clone()).collect();
        self.records.lock().unwrap().insert(name.into(), records);
        Ok(packages)
    }
}

pub fn plan(
    manifest: &rb_gemfile::Manifest,
    runtime: &RubyEnvironment,
    previous: Option<&Lockfile>,
    index: &dyn PackageIndex,
    options: &PlanOptions,
) -> Result<PackagePlan> {
    let ruby_requirement: Requirement = if manifest.bundle.ruby.is_empty() {
        ">= 0".parse()?
    } else {
        manifest.bundle.ruby.join(", ").parse()?
    };
    ensure!(
        ruby_requirement.matches(&runtime.version),
        "Project requires Ruby {ruby_requirement}, selected Ruby is {}",
        runtime.version
    );
    if let Some(engine) = &manifest.bundle.engine {
        ensure!(
            engine == &runtime.engine,
            "Project requires Ruby engine {engine}"
        );
    }
    if let Some(version) = &manifest.bundle.engine_version {
        ensure!(
            version.parse::<rb_gem_types::Version>()? == runtime.engine_version,
            "Project Ruby engine version does not match selected Ruby"
        );
    }
    if let Some(patchlevel) = &manifest.bundle.patchlevel {
        ensure!(
            patchlevel.parse::<i32>()? == runtime.patchlevel,
            "Project Ruby patchlevel does not match selected Ruby"
        );
    }
    ensure!(
        manifest.bundle.plugins.is_empty(),
        "Bundler plugins are not supported by this coordinator"
    );
    ensure!(
        options.mode != LockMode::Locked || previous.is_some(),
        "Locked mode requires an existing lockfile"
    );
    if options.mode == LockMode::Prefer && previous.is_some() {
        let strict = PlanOptions {
            mode: LockMode::Locked,
            ..options.clone()
        };
        if let Ok(plan) = plan(manifest, runtime, previous, index, &strict) {
            return Ok(plan);
        }
    }
    let mut targets = vec![runtime.clone()];
    if options.mode != LockMode::Locked
        && let Some(previous) = previous
    {
        for platform in &previous.platforms {
            if targets.iter().any(|target| &target.platform == platform) {
                continue;
            }
            let mut target = runtime.clone();
            target.platform = platform.clone();
            target.engine = if target.target().os == "java" {
                "jruby".into()
            } else if runtime.engine == "jruby" {
                "ruby".into()
            } else {
                runtime.engine.clone()
            };
            targets.push(target);
        }
    }
    let target_plans = targets
        .iter()
        .map(|target| crate::target_plan::TargetPlan::new(manifest, target, options))
        .collect::<Result<Vec<_>>>()?;
    let declarations = target_plans[0].declarations.clone();
    let install_roots = target_plans[0].install_roots.clone();
    let mut sources = BTreeMap::new();
    for target in &target_plans {
        for (name, source) in &target.sources {
            if let Some(previous) = sources.insert(name.clone(), source.clone()) {
                ensure!(
                    previous.source == source.source
                        && previous.git == source.git
                        && previous.path == source.path
                        && crate::package_source::git_options(&previous)
                            == crate::package_source::git_options(source),
                    "Conflicting sources for {name}"
                );
            }
        }
    }
    let mut locked = Vec::new();
    let mut pins: BTreeMap<String, Vec<crate::source_pin::SourcePin>> = BTreeMap::new();
    if let Some(previous) = previous {
        for source in &previous.sources {
            for package in &source.packages {
                let id = PackageId {
                    name: package.name.clone(),
                    version: package.version.clone(),
                    platform: Platform::parse(&package.platform),
                };
                locked.push(id.clone());
                let checksum = previous
                    .checksums
                    .as_ref()
                    .and_then(|checksums| checksums.get(&package.identity()))
                    .cloned()
                    .flatten();
                for remote in &source.remotes {
                    pins.entry(id.name.clone()).or_default().push(
                        crate::source_pin::SourcePin::new(
                            id.clone(),
                            crate::PackageSource::from_lock(
                                source,
                                remote,
                                options.source_root.as_deref(),
                            )?,
                            checksum.clone(),
                            package,
                        )?,
                    );
                }
            }
        }
        if options.mode == LockMode::Locked {
            ensure!(
                previous.dependencies == declarations,
                "Dependency declarations changed in locked mode"
            );
        }
    }
    let provider = Provider {
        index,
        records: Mutex::new(BTreeMap::new()),
        sources,
        default_source: manifest.bundle.source.clone(),
        pins,
        strict: options.mode == LockMode::Locked,
        runtime,
    };
    let requests: Vec<_> = targets
        .iter()
        .zip(&target_plans)
        .map(|(target, input)| {
            let mut request =
                ResolveRequest::new(input.roots.clone(), target.version.clone(), target.target());
            if options.mode != LockMode::Update {
                request.locked = locked.clone();
            }
            request.allow_prereleases = options.allow_prereleases;
            request.force_ruby = input.force_ruby.clone();
            request
        })
        .collect();
    let resolved = rb_resolver::resolve_targets(&provider, &requests)?;
    let fetched = provider.records.into_inner().unwrap();
    let mut records = BTreeMap::new();
    let mut locked_records = BTreeMap::new();
    for (target, packages) in resolved.into_iter().enumerate() {
        for package in packages {
            let record = fetched[&package.id.name]
                .iter()
                .find(|r| r.package == package)
                .context("Resolved package metadata is missing")?
                .clone();
            locked_records.insert(package.id.clone(), record.clone());
            if target == 0 {
                records.insert(package.id.name.clone(), record);
            }
        }
    }
    if options.mode != LockMode::Update
        && let Some(previous) = previous
    {
        for record in locked_records.values() {
            for source in &previous.sources {
                if !source.remotes.iter().any(|remote| {
                    crate::PackageSource::from_lock(source, remote, options.source_root.as_deref())
                        .is_ok_and(|identity| identity == record.source)
                }) {
                    continue;
                }
                for package in &source.packages {
                    if package.name == record.package.id.name
                        && package.archive_version() == record.archive_version
                        && let Some(Some(checksum)) = previous
                            .checksums
                            .as_ref()
                            .and_then(|sums| sums.get(&package.identity()))
                    {
                        ensure!(
                            checksum.eq_ignore_ascii_case(&record.checksum),
                            "Locked checksum changed for {}",
                            record.package.id
                        );
                    }
                }
            }
        }
    }
    let mut installed = BTreeSet::new();
    let mut pending: Vec<_> = install_roots.into_iter().collect();
    while let Some(name) = pending.pop() {
        if installed.insert(name.clone()) {
            pending.extend(records[&name].package.dependencies.keys().cloned());
        }
    }
    let mut lock = Lockfile {
        dependencies: declarations,
        platforms: targets
            .iter()
            .map(|target| target.platform.clone())
            .collect(),
        bundled_with: runtime.bundler.clone(),
        ruby_version: (!manifest.bundle.ruby.is_empty()).then(|| {
            let mut ruby = format!("ruby {}", runtime.version);
            if runtime.patchlevel >= 0 {
                ruby.push_str(&format!("p{}", runtime.patchlevel));
            }
            if runtime.engine == "ruby" {
                ruby
            } else {
                format!("{ruby} ({} {})", runtime.engine, runtime.engine_version)
            }
        }),
        ..Default::default()
    };
    let mut sections: BTreeMap<crate::PackageSource, Source> = BTreeMap::new();
    let mut checksums = BTreeMap::new();
    for record in locked_records.values() {
        let (_, platform) = record
            .archive_version
            .split_once('-')
            .unwrap_or((&record.archive_version, "ruby"));
        let package = LockedPackage {
            name: record.package.id.name.clone(),
            version: record.package.id.version.clone(),
            platform: platform.into(),
            dependencies: record
                .package
                .dependencies
                .iter()
                .map(|(n, r)| (n.clone(), vec![r.clone()]))
                .collect(),
        };
        checksums.insert(
            package.identity(),
            if matches!(record.source, crate::PackageSource::Registry(_)) {
                Some(record.checksum.clone())
            } else {
                None
            },
        );
        sections
            .entry(record.source.clone())
            .or_insert_with(|| record.source.lock_source())
            .packages
            .push(package);
    }
    if let Some(source) = &manifest.bundle.source {
        let source = source_url(source)?;
        sections
            .entry(source.clone().into())
            .or_insert_with(|| Source::new(SourceKind::Registry, source));
    } else if manifest.dependencies.values().all(|dependency| {
        std::iter::once(dependency)
            .chain(&dependency.variants)
            .all(|declaration| {
                !declaration.enabled
                    || declaration.source.is_some()
                    || declaration.git.is_some()
                    || declaration.path.is_some()
            })
    }) {
        let mut source = Source::new(SourceKind::Registry, "");
        source.remotes.clear();
        sections.insert(crate::PackageSource::Registry(String::new()), source);
    }
    lock.sources = sections.into_values().collect();
    lock.checksums = Some(checksums);
    if let Some(previous) = previous
        && options.mode == LockMode::Locked
    {
        ensure!(
            previous
                .sources
                .iter()
                .all(|source| source.remotes.iter().all(|remote| {
                    crate::PackageSource::from_lock(source, remote, options.source_root.as_deref())
                        .is_ok()
                })),
            "Invalid locked source"
        );
    }
    if options.mode == LockMode::Locked {
        for record in records.values() {
            ensure!(
                previous.unwrap().sources.iter().any(|source| source
                    .packages
                    .iter()
                    .any(|package| package.name == record.package.id.name
                        && package.archive_version() == record.archive_version)),
                "Locked archive spelling changed for {}",
                record.package.id
            );
        }
    }
    let lock_changed = previous.is_none_or(|previous| previous != &lock);
    if options.mode == LockMode::Locked {
        lock = previous.unwrap().clone();
    }
    Ok(PackagePlan {
        packages: records
            .into_iter()
            .map(|(name, record)| PlannedPackage {
                record,
                install: installed.contains(&name),
            })
            .collect(),
        lockfile: lock,
        lock_changed: lock_changed && options.mode != LockMode::Locked,
        without: options.without.clone(),
        with: options.with.clone(),
    })
}
