use crate::{PackageRecord, RubyEnvironment};
use anyhow::{Context, Result};
use rb_gem_installer::BuildOptions;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct Receipt {
    pub(crate) checksum: String,
    pub(crate) archive_version: String,
    pub(crate) source: crate::PackageSource,
    pub(crate) ruby: PathBuf,
    pub(crate) version: String,
    pub(crate) platform: String,
    pub(crate) abi: String,
    pub(crate) configuration_hash: String,
    pub(crate) dependencies: BTreeMap<String, (crate::PackageSource, String, String)>,
}

impl Receipt {
    pub(crate) fn new(
        record: &PackageRecord,
        runtime: &RubyEnvironment,
        build: &BuildOptions,
    ) -> Result<Self> {
        Ok(Self {
            checksum: record.checksum.clone(),
            archive_version: record.archive_version.clone(),
            source: record.source.clone(),
            ruby: runtime.executable.clone(),
            version: runtime.version.to_string(),
            platform: runtime.platform.clone(),
            abi: runtime.abi.clone(),
            configuration_hash: configuration_hash(build)?,
            dependencies: BTreeMap::new(),
        })
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct InstalledReceipt {
    pub(crate) receipt: Receipt,
    pub(crate) gem: rb_microgems::InstalledGem,
    pub(crate) files: Vec<PathBuf>,
    pub(crate) build_artifacts: Vec<PathBuf>,
    pub(crate) extension: Option<PathBuf>,
}

fn configuration_hash(build: &BuildOptions) -> Result<String> {
    let mut environment: BTreeMap<_, _> = [
        "CC",
        "CXX",
        "CFLAGS",
        "CXXFLAGS",
        "CPPFLAGS",
        "LDFLAGS",
        "LIBS",
        "MAKE",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "PKG_CONFIG_PATH",
        "SDKROOT",
    ]
    .into_iter()
    .filter_map(|key| std::env::var_os(key).map(|value| (key.to_owned(), value)))
    .collect();
    for (key, value) in &build.environment {
        environment.insert(
            key.to_str()
                .context("Environment name is not UTF-8")?
                .to_owned(),
            value.clone(),
        );
    }
    let environment: BTreeMap<_, _> = environment
        .iter()
        .map(|(key, value)| {
            Ok((
                key,
                value.to_str().context("Environment value is not UTF-8")?,
            ))
        })
        .collect::<Result<_>>()?;
    let arguments: Vec<_> = build
        .arguments
        .iter()
        .map(|value| value.to_str().context("Build argument is not UTF-8"))
        .collect::<Result<_>>()?;
    let bytes = serde_json::to_vec(
        &serde_json::json!({"arguments":arguments,"environment":environment,"load_paths":build.load_paths,"dependencies":build.dependencies,"make":build.make,"cargo":build.cargo,"cmake":build.cmake}),
    )?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
