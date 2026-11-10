use anyhow::{Context, Result, ensure};
use rb_gem_types::{Platform, Version};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RubyEnvironment {
    pub executable: PathBuf,
    pub version: Version,
    pub platform: String,
    pub engine: String,
    pub engine_version: Version,
    pub patchlevel: i32,
    pub abi: String,
    pub rubygems: Version,
    pub bundler: Option<Version>,
    #[serde(default)]
    pub bundler_archive: Option<PathBuf>,
    pub build_tools: Vec<rb_microgems::InstalledGem>,
    pub configuration_inputs: Vec<PathBuf>,
}

impl RubyEnvironment {
    pub fn inspect(ruby: impl AsRef<Path>) -> Result<Self> {
        Self::inspect_with_inputs(
            ruby,
            &std::env::current_dir()?,
            &std::env::vars_os().collect(),
        )
    }

    pub fn inspect_with_inputs(
        ruby: impl AsRef<Path>,
        directory: &Path,
        environment: &std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
    ) -> Result<Self> {
        let helper = tempfile::Builder::new().suffix(".rb").tempfile()?;
        std::fs::write(
            helper.path(),
            include_str!("../../../gems/rb-gem-installer/lib/rb_gem_installer/runtime.rb"),
        )?;
        let output = Command::new(ruby.as_ref())
            .current_dir(directory)
            .env_clear()
            .envs(environment)
            .env_remove("RUBYOPT")
            .env_remove("RUBYLIB")
            .args(["-rjson", "-r"])
            .arg(helper.path())
            .args([
                "-e",
                "puts JSON.generate(RubyButler::GemInstaller::Runtime.read)",
            ])
            .output()
            .context("Inspecting Ruby")?;
        ensure!(
            output.status.success(),
            "Ruby inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).context("Reading Ruby environment")
    }

    pub(crate) fn bundler_checksum(&self) -> Result<Option<String>> {
        if self
            .bundler
            .as_ref()
            .is_none_or(|version| version < &"4.0.0".parse().unwrap())
        {
            return Ok(None);
        }
        let Some(path) = &self.bundler_archive else {
            return Ok(None);
        };
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(format!("{:x}", Sha256::digest(bytes)))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("Reading cached Bundler archive"),
        }
    }

    pub fn target(&self) -> Platform {
        Platform::parse(&self.platform)
    }
}
