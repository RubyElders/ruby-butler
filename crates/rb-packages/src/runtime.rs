use anyhow::{Context, Result, ensure};
use rb_gem_types::{Platform, Version};
use serde::Deserialize;
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
    pub build_tools: Vec<rb_microgems::InstalledGem>,
    pub configuration_inputs: Vec<PathBuf>,
}

impl RubyEnvironment {
    pub fn inspect(ruby: impl AsRef<Path>) -> Result<Self> {
        let helper = tempfile::Builder::new().suffix(".rb").tempfile()?;
        std::fs::write(
            helper.path(),
            include_str!("../../../gems/rb-gem-installer/lib/rb_gem_installer/runtime.rb"),
        )?;
        let output = Command::new(ruby.as_ref())
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

    pub fn target(&self) -> Platform {
        Platform::parse(&self.platform)
    }
}
