use crate::{PackageIndex, PackageRecord, constants::DEFAULT_SOURCE, provider::source_url};
use anyhow::{Context, Result, ensure};
use rb_compact_index::Client;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Default)]
struct Metadata {
    discovered: bool,
    names: BTreeMap<String, Option<BTreeSet<String>>>,
    routes: BTreeMap<String, String>,
    records: BTreeMap<(String, String), Vec<PackageRecord>>,
}

pub struct RegistryIndex {
    clients: BTreeMap<String, Client>,
    default: Option<String>,
    roots: BTreeMap<String, String>,
    explicit: BTreeSet<String>,
    metadata: Mutex<Metadata>,
}

impl RegistryIndex {
    pub(crate) fn load_credentials(&mut self, project: &Path, offline: bool) -> Result<()> {
        if offline {
            return Ok(());
        }
        let settings = crate::credentials::Credentials::load(project)?;
        for (source, client) in &mut self.clients {
            if let Some(credentials) = settings.credentials(source)? {
                *client = client.clone().with_credentials(&credentials)?;
            }
        }
        Ok(())
    }

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
        let mut roots = BTreeMap::new();
        let mut explicit = BTreeSet::new();
        for (name, dependency) in &manifest.dependencies {
            for declaration in std::iter::once(dependency)
                .chain(&dependency.variants)
                .filter(|d| d.enabled)
            {
                explicit.insert(name.clone());
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
                if let Some(previous) = roots.insert(name.clone(), source.clone()) {
                    ensure!(previous == source, "Conflicting sources for {name}");
                }
                if !clients.contains_key(&source) {
                    clients.insert(source.clone(), Client::new(&source, cache, offline)?);
                }
            }
        }
        let metadata = Metadata {
            routes: roots.clone(),
            ..Default::default()
        };
        Ok(Self {
            clients,
            default,
            roots,
            explicit,
            metadata: Mutex::new(metadata),
        })
    }

    fn fetch(
        &self,
        metadata: &mut Metadata,
        source: &str,
        name: &str,
    ) -> Result<Vec<PackageRecord>> {
        let key = (source.to_owned(), name.to_owned());
        if let Some(records) = metadata.records.get(&key) {
            return Ok(records.clone());
        }
        let records = self.clients[source]
            .info_or_empty(name)?
            .into_iter()
            .map(|info| PackageRecord::from_info(name, source, info))
            .collect::<Result<Vec<_>>>()?;
        metadata.records.insert(key, records.clone());
        Ok(records)
    }

    fn discover_scoped_sources(&self, metadata: &mut Metadata) -> Result<()> {
        if metadata.discovered {
            return Ok(());
        }
        let mut pending: BTreeSet<_> = self
            .roots
            .iter()
            .filter(|(_, source)| Some(source.as_str()) != self.default.as_deref())
            .map(|(name, source)| (source.clone(), name.clone()))
            .collect();
        let mut visited = BTreeSet::new();
        while let Some((source, name)) = pending.pop_first() {
            if !visited.insert((source.clone(), name.clone())) {
                continue;
            }
            if !self.explicit.contains(&name) {
                if !metadata.names.contains_key(&source) {
                    metadata
                        .names
                        .insert(source.clone(), self.clients[&source].names()?);
                }
                if metadata.names[&source]
                    .as_ref()
                    .is_some_and(|names| !names.contains(&name))
                {
                    continue;
                }
            }
            let records = self.fetch(metadata, &source, &name)?;
            if records.is_empty() {
                continue;
            }
            if !self.explicit.contains(&name)
                && let Some(previous) = metadata.routes.insert(name.clone(), source.clone())
            {
                ensure!(previous == source, "Ambiguous transitive source for {name}");
            }
            for dependency in records
                .iter()
                .flat_map(|record| record.package.dependencies.keys())
            {
                if !self.explicit.contains(dependency) {
                    pending.insert((source.clone(), dependency.clone()));
                }
            }
        }
        metadata.discovered = true;
        Ok(())
    }
}

impl PackageIndex for RegistryIndex {
    fn candidates(&self, name: &str) -> Result<Vec<PackageRecord>> {
        let mut metadata = self.metadata.lock().unwrap();
        self.discover_scoped_sources(&mut metadata)?;
        let source = metadata
            .routes
            .get(name)
            .cloned()
            .or_else(|| self.default.clone())
            .context("Transitive dependency has no unambiguous source")?;
        self.fetch(&mut metadata, &source, name)
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
