use crate::{
    LockMode, MemoryIndex, PackageIndex, PackagePlan, PackageRecord, PlanOptions, RubyEnvironment,
};
use anyhow::{Result, ensure};
use rb_gem_types::{PackageId, Platform};
use rb_lockfile::{Lockfile, Source, SourceKind};
use rb_resolver::Package;

pub(crate) fn plan(
    manifest: &rb_gemfile::Manifest,
    runtime: &RubyEnvironment,
    previous: &Lockfile,
    index: &dyn PackageIndex,
    options: &PlanOptions,
) -> Result<PackagePlan> {
    let mut graph = previous.clone();
    let needs_bundler = graph
        .sources
        .iter()
        .flat_map(|source| &source.packages)
        .any(|package| package.dependencies.contains_key("bundler"));
    if needs_bundler
        && !graph
            .sources
            .iter()
            .flat_map(|source| &source.packages)
            .any(|package| package.name == "bundler")
    {
        let version = runtime
            .bundler
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Bundler is required by the locked graph"))?;
        let mut source = Source::new(
            SourceKind::Registry,
            manifest
                .bundle
                .source
                .as_deref()
                .unwrap_or(crate::constants::DEFAULT_SOURCE),
        );
        source.packages.push(rb_lockfile::LockedPackage {
            name: "bundler".into(),
            version,
            platform: "ruby".into(),
            dependencies: Default::default(),
        });
        graph.sources.push(source);
    }
    let mut locked = MemoryIndex::default();
    for source in &graph.sources {
        ensure!(source.remotes.len() <= 1, "Ambiguous locked source");
        for package in &source.packages {
            let remote = source
                .remotes
                .first()
                .ok_or_else(|| anyhow::anyhow!("Locked package has no source"))?;
            let record = PackageRecord {
                package: Package {
                    id: PackageId {
                        name: package.name.clone(),
                        version: package.version.clone(),
                        platform: Platform::parse(&package.platform),
                    },
                    dependencies: package
                        .dependencies
                        .iter()
                        .map(|(name, requirements)| {
                            Ok((
                                name.clone(),
                                requirements
                                    .iter()
                                    .map(ToString::to_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                                    .parse()?,
                            ))
                        })
                        .collect::<Result<_>>()?,
                    ruby: Default::default(),
                },
                source: crate::PackageSource::from_lock(
                    source,
                    remote,
                    options.source_root.as_deref(),
                )?,
                archive_version: package.archive_version(),
                checksum: graph
                    .checksums
                    .as_ref()
                    .and_then(|sums| sums.get(&package.identity()))
                    .cloned()
                    .flatten()
                    .unwrap_or_else(|| "0".repeat(crate::constants::CHECKSUM_CHARACTERS)),
                rubygems: None,
                declared_ruby: None,
            };
            locked
                .packages
                .entry(package.name.clone())
                .or_default()
                .push(record);
        }
    }
    let strict = PlanOptions {
        mode: LockMode::Locked,
        ..options.clone()
    };
    let mut plan = crate::plan::resolve(manifest, runtime, Some(&graph), &locked, &strict)?;
    let selected: Vec<_> = plan
        .packages
        .iter()
        .map(|package| package.record.clone())
        .collect();
    let records = index.locked_records(&selected)?;
    ensure!(
        records.len() == selected.len(),
        "Incomplete locked metadata"
    );
    for (package, record) in plan.packages.iter_mut().zip(records) {
        record.validate()?;
        let expected = &package.record;
        ensure!(
            record.package.id == expected.package.id && record.source == expected.source,
            "Locked package identity changed"
        );
        ensure!(
            record.archive_version == expected.archive_version,
            "Locked archive spelling changed for {}",
            record.package.id
        );
        ensure!(
            record.package.ruby.matches(&runtime.version)
                && record
                    .rubygems
                    .as_ref()
                    .is_none_or(|required| required.matches(&runtime.rubygems)),
            "Locked package is incompatible with Ruby: {}",
            record.package.id
        );
        ensure!(
            expected.package.dependencies.len() == record.package.dependencies.len()
                && expected
                    .package
                    .dependencies
                    .iter()
                    .all(
                        |(name, required)| record.package.dependencies.get(name).is_some_and(
                            |actual| crate::validation::same_requirement(required, actual)
                        )
                    ),
            "Locked dependency metadata changed for {}",
            record.package.id
        );
        if let Some(Some(checksum)) = previous.checksums.as_ref().and_then(|sums| {
            sums.get(
                &rb_lockfile::LockedPackage {
                    name: record.package.id.name.clone(),
                    version: record.package.id.version.clone(),
                    platform: expected
                        .archive_version
                        .split_once('-')
                        .map_or("ruby", |(_, platform)| platform)
                        .into(),
                    dependencies: Default::default(),
                }
                .identity(),
            )
        }) {
            ensure!(
                checksum.eq_ignore_ascii_case(&record.checksum),
                "Locked checksum changed for {}",
                record.package.id
            );
        }
        package.record = record;
    }
    plan.lockfile = previous.clone();
    plan.lock_changed = false;
    Ok(plan)
}
