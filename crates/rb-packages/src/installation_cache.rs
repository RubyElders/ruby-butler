use crate::{
    PackageRecord, PackageSource, RubyEnvironment,
    constants::{
        GEMS_DIRECTORY, INSTALL_RECEIPT, INSTALLATION_CACHE_DIRECTORY, INSTALLATION_CACHE_MANIFEST,
        SPECIFICATIONS_DIRECTORY,
    },
    copy_tree::copy_tree,
    receipt::{InstalledReceipt, Receipt},
    slot::Slot,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
struct Manifest {
    home: PathBuf,
    metadata: rb_gem_archive::GemMetadata,
    dependencies: Vec<PathBuf>,
}

pub(crate) struct Entry {
    directory: PathBuf,
    full_name: String,
    lock: fs::File,
}

impl Entry {
    pub(crate) fn open(
        cache: &Path,
        receipt: &Receipt,
        full_name: &str,
        runtime: &RubyEnvironment,
    ) -> Result<Self> {
        let entry = Self::unlocked(cache, receipt, full_name, runtime)?;
        entry.lock.lock()?;
        Ok(entry)
    }

    pub(crate) fn try_open(
        cache: &Path,
        receipt: &Receipt,
        full_name: &str,
        runtime: &RubyEnvironment,
    ) -> Result<Option<Self>> {
        let entry = Self::unlocked(cache, receipt, full_name, runtime)?;
        Ok(entry.lock.try_lock().is_ok().then_some(entry))
    }

    fn unlocked(
        cache: &Path,
        receipt: &Receipt,
        full_name: &str,
        runtime: &RubyEnvironment,
    ) -> Result<Self> {
        let root = cache.join(INSTALLATION_CACHE_DIRECTORY);
        fs::create_dir_all(&root)?;
        let root = dunce::canonicalize(root)?;
        let key = key(receipt, full_name, runtime)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(format!("{key}.lock")))?;
        Ok(Self {
            directory: root.join(key),
            full_name: full_name.into(),
            lock,
        })
    }

    pub(crate) fn home(&self) -> PathBuf {
        self.directory.join("home")
    }

    pub(crate) fn gem(&self) -> PathBuf {
        self.home().join(GEMS_DIRECTORY).join(&self.full_name)
    }

    fn manifest_path(&self) -> PathBuf {
        self.directory.join(INSTALLATION_CACHE_MANIFEST)
    }

    fn read(&self) -> Result<Manifest> {
        let manifest: Manifest = serde_json::from_slice(&fs::read(self.manifest_path())?)?;
        ensure!(manifest.home == self.home(), "Installation cache moved");
        ensure!(
            self.gem().is_dir()
                && self.gem().join(INSTALL_RECEIPT).is_file()
                && self
                    .home()
                    .join(SPECIFICATIONS_DIRECTORY)
                    .join(format!("{}.gemspec", self.full_name))
                    .is_file(),
            "Cached installation metadata is missing"
        );
        ensure!(
            manifest.dependencies.iter().all(|path| path.is_file()),
            "Cached build dependency is missing"
        );
        Ok(manifest)
    }

    pub(crate) fn restore(
        &self,
        slot: &mut Slot,
        home: &Path,
        record: &PackageRecord,
        runtime: &RubyEnvironment,
    ) -> Result<bool> {
        let manifest = match self.read() {
            Ok(manifest) => manifest,
            Err(_) => return Ok(false),
        };
        crate::validation::ensure_record(&manifest.metadata, record, &runtime.version)?;
        let mut installed: InstalledReceipt =
            serde_json::from_slice(&fs::read(self.gem().join(INSTALL_RECEIPT))?)?;
        ensure!(
            key(&installed.receipt, &self.full_name, runtime)?
                == key(&slot.receipt, &self.full_name, runtime)?,
            "Cached installation identity changed"
        );
        if slot.directory.exists() {
            fs::remove_dir_all(&slot.directory)?;
        }
        copy_tree(&self.gem(), &slot.directory)?;
        let specification = self
            .home()
            .join(SPECIFICATIONS_DIRECTORY)
            .join(format!("{}.gemspec", self.full_name));
        let target = home
            .join(SPECIFICATIONS_DIRECTORY)
            .join(specification.file_name().unwrap());
        fs::create_dir_all(target.parent().unwrap())?;
        fs::copy(specification, target)?;
        if let Some(extension) = &installed.extension {
            let target = home.join(extension);
            if target.exists() {
                fs::remove_dir_all(&target)?;
            }
            copy_tree(&self.home().join(extension), &target)?;
        }
        let receipt = normalized_receipt(&installed.receipt)?;
        if receipt != installed.receipt {
            installed.receipt = receipt;
            fs::write(
                self.gem().join(INSTALL_RECEIPT),
                serde_json::to_vec(&installed)?,
            )?;
        }
        installed.cache_dependencies = if manifest.metadata.extensions.is_empty() {
            Vec::new()
        } else {
            manifest.dependencies
        };
        installed.receipt = slot.receipt.clone();
        installed.gem.root = slot.directory.clone();
        fs::write(
            slot.directory.join(INSTALL_RECEIPT),
            serde_json::to_vec(&installed)?,
        )?;
        slot.gem = Some(installed.gem);
        slot.metadata = Some(manifest.metadata);
        slot.staging = None;
        slot.reused = true;
        slot.restored = true;
        Ok(true)
    }

    pub(crate) fn begin(&self) -> Result<()> {
        if self.directory.exists() {
            fs::remove_dir_all(&self.directory)?;
        }
        fs::create_dir_all(self.home().join(GEMS_DIRECTORY))?;
        Ok(())
    }

    pub(crate) fn publish(
        &self,
        metadata: &rb_gem_archive::GemMetadata,
        dependencies: Vec<PathBuf>,
    ) -> Result<()> {
        let manifest = Manifest {
            home: self.home(),
            metadata: metadata.clone(),
            dependencies,
        };
        let mut file = tempfile::NamedTempFile::new_in(&self.directory)?;
        use std::io::Write;
        file.write_all(&serde_json::to_vec(&manifest)?)?;
        file.persist(self.manifest_path())?;
        Ok(())
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        let _ = self.lock.unlock();
    }
}

pub(crate) fn dependency(
    cache: &Path,
    receipt: &Receipt,
    full_name: &str,
    runtime: &RubyEnvironment,
) -> Result<Option<(PathBuf, PathBuf)>> {
    let entry = Entry::open(cache, receipt, full_name, runtime)?;
    Ok(entry
        .read()
        .is_ok()
        .then(|| (entry.gem(), entry.manifest_path())))
}

fn normalized_source(source: &PackageSource) -> Result<PackageSource> {
    Ok(match source {
        PackageSource::Registry(source) => {
            PackageSource::Registry(crate::provider::source_url(source)?)
        }
        PackageSource::Path { glob, .. } => PackageSource::Path {
            directory: PathBuf::new(),
            glob: glob.clone(),
        },
        source => source.clone(),
    })
}

fn key(receipt: &Receipt, full_name: &str, runtime: &RubyEnvironment) -> Result<String> {
    let receipt = normalized_receipt(receipt)?;
    let bytes = serde_json::to_vec(&(
        &receipt,
        full_name,
        &runtime.engine,
        &runtime.engine_version,
        &runtime.rubygems,
        std::env::consts::OS,
        std::env::consts::ARCH,
    ))
    .context("Identifying cached installation")?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(crate) fn normalized_receipt(receipt: &Receipt) -> Result<Receipt> {
    let mut receipt = receipt.clone();
    receipt.source = normalized_source(&receipt.source)?;
    for (source, _, _) in receipt.dependencies.values_mut() {
        *source = normalized_source(source)?;
    }
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn receipt() -> Receipt {
        Receipt {
            checksum: "a".repeat(64),
            archive_version: "1.0.0".into(),
            source: "https://gems.example/".into(),
            ruby: "/rubies/ruby/bin/ruby".into(),
            version: "4.0.9".into(),
            platform: "x86_64-linux".into(),
            abi: "4.0.0".into(),
            configuration_hash: "b".repeat(64),
            dependencies: BTreeMap::new(),
        }
    }

    #[test]
    fn keys_separate_sources_payloads_rubies_builds_and_dependencies() {
        let runtime = crate::runtime::RubyEnvironment {
            executable: "/rubies/ruby/bin/ruby".into(),
            version: "4.0.9".parse().unwrap(),
            platform: "x86_64-linux".into(),
            engine: "ruby".into(),
            engine_version: "4.0.9".parse().unwrap(),
            patchlevel: 0,
            abi: "4.0.0".into(),
            rubygems: "4.0.0".parse().unwrap(),
            bundler: None,
            bundler_archive: None,
            build_tools: Vec::new(),
            configuration_inputs: Vec::new(),
        };
        let base = receipt();
        let original = key(&base, "teacup-1.0.0", &runtime).unwrap();
        let mut equivalent = base.clone();
        equivalent.source = "https://GEMS.EXAMPLE".into();
        assert_eq!(
            original,
            key(&equivalent, "teacup-1.0.0", &runtime).unwrap()
        );
        for changed in [
            Receipt {
                checksum: "c".repeat(64),
                ..base.clone()
            },
            Receipt {
                source: "https://other.example/".into(),
                ..base.clone()
            },
            Receipt {
                ruby: "/other/ruby/bin/ruby".into(),
                ..base.clone()
            },
            Receipt {
                version: "3.4.8".into(),
                ..base.clone()
            },
            Receipt {
                platform: "arm64-darwin".into(),
                ..base.clone()
            },
            Receipt {
                abi: "4.1.0".into(),
                ..base.clone()
            },
            Receipt {
                configuration_hash: "d".repeat(64),
                ..base.clone()
            },
            Receipt {
                dependencies: BTreeMap::from([(
                    "leaf".into(),
                    (base.source.clone(), "1.0.0".into(), "e".repeat(64)),
                )]),
                ..base.clone()
            },
        ] {
            assert_ne!(original, key(&changed, "teacup-1.0.0", &runtime).unwrap());
        }
        let mut local = base.clone();
        local.source = PackageSource::Path {
            directory: "/project/one/local".into(),
            glob: None,
        };
        equivalent.source = PackageSource::Path {
            directory: "/project/two/local".into(),
            glob: None,
        };
        assert_eq!(
            key(&local, "teacup-1.0.0", &runtime).unwrap(),
            key(&equivalent, "teacup-1.0.0", &runtime).unwrap()
        );
    }
    #[test]
    fn completed_artifact_trusts_payloads_and_requires_dependency_completion() {
        let runtime = RubyEnvironment::inspect(
            std::env::var_os("RB_GEM_BUILD_RUBY").unwrap_or_else(|| "ruby".into()),
        )
        .unwrap();
        let cache = tempfile::tempdir().unwrap();
        let metadata = rb_gem_archive::GemMetadata {
            id: rb_gem_types::PackageId {
                name: "teacup".into(),
                version: "1.0.0".parse().unwrap(),
                platform: rb_gem_types::Platform::parse("ruby"),
            },
            dependencies: BTreeMap::new(),
            ruby: Default::default(),
            rubygems: Default::default(),
            require_paths: vec!["lib".into()],
            executables: Vec::new(),
            bindir: "bin".into(),
            extensions: Vec::new(),
            files: Vec::new(),
            metadata: BTreeMap::new(),
        };
        let dependency =
            Entry::open(cache.path(), &receipt(), "dependency-1.0.0", &runtime).unwrap();
        dependency.begin().unwrap();
        fs::write(dependency.home().join("payload"), "original").unwrap();
        dependency.publish(&metadata, Vec::new()).unwrap();
        let parent = Entry::open(cache.path(), &receipt(), "parent-1.0.0", &runtime).unwrap();
        parent.begin().unwrap();
        fs::write(parent.home().join("payload"), "original").unwrap();
        parent
            .publish(&metadata, vec![dependency.manifest_path()])
            .unwrap();
        for entry in [&dependency, &parent] {
            fs::create_dir_all(entry.gem()).unwrap();
            fs::write(entry.gem().join(INSTALL_RECEIPT), "receipt").unwrap();
            fs::create_dir_all(entry.home().join(SPECIFICATIONS_DIRECTORY)).unwrap();
            fs::write(
                entry
                    .home()
                    .join(SPECIFICATIONS_DIRECTORY)
                    .join(format!("{}.gemspec", entry.full_name)),
                "specification",
            )
            .unwrap();
        }
        assert!(parent.read().is_ok());
        fs::write(dependency.home().join("payload"), "changed").unwrap();
        assert!(parent.read().is_ok());
        fs::write(parent.home().join("payload"), "changed").unwrap();
        assert!(parent.read().is_ok());
        fs::remove_file(dependency.manifest_path()).unwrap();
        assert!(parent.read().is_err());
    }
}
