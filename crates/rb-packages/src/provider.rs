use crate::constants::DEFAULT_SOURCE;
use anyhow::{Context, Result, ensure};
use rb_compact_index::{Client, GemInfo};
use rb_gem_types::{PackageId, Platform};
use rb_resolver::Package;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageRecord {
    pub package: Package,
    pub source: crate::PackageSource,
    pub archive_version: String,
    pub checksum: String,
    pub rubygems: Option<rb_gem_types::Requirement>,
}

impl PackageRecord {
    pub(crate) fn validate(&self) -> Result<()> {
        for value in [&self.package.id.name, &self.archive_version] {
            ensure!(
                !value.is_empty()
                    && value != "."
                    && value != ".."
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric()
                            || matches!(byte, b'_' | b'-' | b'.')),
                "Invalid package archive identity"
            );
        }
        let (version, platform) = self
            .archive_version
            .split_once('-')
            .unwrap_or((&self.archive_version, "ruby"));
        ensure!(
            version.parse::<rb_gem_types::Version>()? == self.package.id.version
                && Platform::parse(platform) == self.package.id.platform,
            "Archive filename does not match package identity"
        );
        ensure!(
            self.checksum.len() == crate::constants::CHECKSUM_CHARACTERS
                && self.checksum.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "Invalid archive checksum"
        );
        self.source.validate()?;
        Ok(())
    }

    pub fn from_info(name: &str, source: &str, record: GemInfo) -> Result<Self> {
        let (version, platform) = record
            .version
            .split_once('-')
            .unwrap_or((&record.version, "ruby"));
        Ok(Self {
            package: Package {
                id: PackageId {
                    name: name.into(),
                    version: version.parse()?,
                    platform: Platform::parse(platform),
                },
                dependencies: record
                    .dependencies
                    .iter()
                    .map(|(name, value)| Ok((name.clone(), value.replace('&', ", ").parse()?)))
                    .collect::<Result<_>>()?,
                ruby: record
                    .metadata
                    .get("ruby")
                    .map(|v| v.replace('&', ", ").parse())
                    .transpose()?
                    .unwrap_or_default(),
            },
            rubygems: record
                .metadata
                .get("rubygems")
                .map(|v| v.replace('&', ", ").parse())
                .transpose()?,
            source: source_url(source)?.into(),
            archive_version: record.version.clone(),
            checksum: record
                .metadata
                .get("checksum")
                .context("Missing archive checksum")?
                .clone(),
        })
    }
}

pub trait PackageIndex: Send + Sync {
    fn candidates(&self, name: &str) -> Result<Vec<PackageRecord>>;
    fn archive(&self, record: &PackageRecord) -> Result<PathBuf>;
    fn stage_sources(
        &self,
        _home: &Path,
        plan: &crate::PackagePlan,
        _runtime: &crate::RubyEnvironment,
    ) -> Result<bool> {
        ensure!(
            plan.packages
                .iter()
                .all(|package| matches!(package.record.source, crate::PackageSource::Registry(_))),
            "This index does not provide source snapshots"
        );
        Ok(false)
    }
    fn compatibility_path(&self, _record: &PackageRecord, _home: &Path) -> Result<Option<PathBuf>> {
        Ok(None)
    }
}

#[derive(Default)]
pub struct MemoryIndex {
    pub packages: BTreeMap<String, Vec<PackageRecord>>,
    pub archives: BTreeMap<String, PathBuf>,
}

impl PackageIndex for MemoryIndex {
    fn candidates(&self, name: &str) -> Result<Vec<PackageRecord>> {
        Ok(self.packages.get(name).cloned().unwrap_or_default())
    }
    fn archive(&self, record: &PackageRecord) -> Result<PathBuf> {
        self.archives
            .get(&record.checksum)
            .cloned()
            .context("Archive is not available in the memory index")
    }
}

pub struct RegistryIndex {
    clients: BTreeMap<String, Client>,
    default: Option<String>,
    explicit: BTreeSet<String>,
    metadata: Mutex<BTreeMap<String, Vec<PackageRecord>>>,
    routes: Mutex<BTreeMap<String, String>>,
}

impl RegistryIndex {
    pub fn new(manifest: &rb_gemfile::Manifest, cache: &Path, offline: bool) -> Result<Self> {
        let locations: BTreeSet<_> = manifest
            .dependencies
            .values()
            .flat_map(|dependency| std::iter::once(dependency).chain(&dependency.variants))
            .filter(|dependency| dependency.enabled)
            .filter_map(|dependency| dependency.source.as_deref())
            .map(source_url)
            .collect::<Result<_>>()?;
        let default = if let Some(source) = &manifest.bundle.source {
            Some(source_url(source)?)
        } else if locations.len() == 1 {
            locations.first().cloned()
        } else if locations.is_empty() {
            Some(source_url(DEFAULT_SOURCE)?)
        } else {
            None
        };
        let mut clients = BTreeMap::new();
        if let Some(source) = &default {
            clients.insert(source.clone(), Client::new(source, cache, offline)?);
        }
        let mut routes = BTreeMap::new();
        for (name, dependency) in &manifest.dependencies {
            let declarations = std::iter::once(dependency).chain(&dependency.variants);
            for declaration in declarations.filter(|d| d.enabled) {
                if declaration.git.is_some() || declaration.path.is_some() {
                    continue;
                }
                let source = source_url(
                    declaration
                        .source
                        .as_deref()
                        .or(default.as_deref())
                        .context("Dependency needs an explicit source")?,
                )?;
                if let Some(previous) = routes.insert(name.clone(), source.clone()) {
                    ensure!(previous == source, "Conflicting sources for {name}");
                }
                if !clients.contains_key(&source) {
                    clients.insert(source.clone(), Client::new(&source, cache, offline)?);
                }
            }
        }
        Ok(Self {
            clients,
            default,
            explicit: routes.keys().cloned().collect(),
            routes: Mutex::new(routes),
            metadata: Mutex::new(BTreeMap::new()),
        })
    }
}

impl PackageIndex for RegistryIndex {
    fn candidates(&self, name: &str) -> Result<Vec<PackageRecord>> {
        let mut metadata = self.metadata.lock().unwrap();
        if let Some(records) = metadata.get(name) {
            return Ok(records.clone());
        }
        let source = self
            .routes
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .or_else(|| self.default.clone())
            .context("Transitive dependency has no unambiguous source")?;
        let records: Vec<_> = self.clients[&source]
            .info(name)?
            .into_iter()
            .map(|info| PackageRecord::from_info(name, &source, info))
            .collect::<Result<_>>()?;
        if self.default.is_none() {
            let mut routes = self.routes.lock().unwrap();
            for dependency in records
                .iter()
                .flat_map(|record| record.package.dependencies.keys())
            {
                if self.explicit.contains(dependency) {
                    continue;
                }
                if let Some(previous) = routes.get(dependency) {
                    ensure!(
                        previous == &source,
                        "Ambiguous transitive source for {dependency}"
                    );
                }
                routes.insert(dependency.clone(), source.clone());
            }
        }
        metadata.insert(name.into(), records.clone());
        Ok(records)
    }
    fn archive(&self, record: &PackageRecord) -> Result<PathBuf> {
        let crate::PackageSource::Registry(source) = &record.source else {
            anyhow::bail!("Not a registry archive");
        };
        self.clients
            .get(source)
            .context("Unknown package source")?
            .download(
                &record.package.id.name,
                &record.archive_version,
                &record.checksum,
            )
    }
}

pub(crate) fn source_url(source: &str) -> Result<String> {
    let mut url = url::Url::parse(source)?;
    ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "Invalid registry source"
    );
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url.to_string())
}
