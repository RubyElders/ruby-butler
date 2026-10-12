use crate::{BuildOptions, BuildStage, build_process};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const FILES: &[(&str, &str)] = &[
    (
        "lib/rb_gem_installer/source_specifications.rb",
        include_str!(
            "../../../gems/rb-gem-installer/lib/rb_gem_installer/source_specifications.rb"
        ),
    ),
    (
        "lib/rb_gem_installer/runtime.rb",
        include_str!("../../../gems/rb-gem-installer/lib/rb_gem_installer/runtime.rb"),
    ),
    (
        "lib/rb_gem_installer/specification.rb",
        include_str!("../../../gems/rb-gem-installer/lib/rb_gem_installer/specification.rb"),
    ),
    (
        "lib/rb_gem_installer.rb",
        include_str!("../../../gems/rb-gem-installer/lib/rb_gem_installer.rb"),
    ),
    (
        "lib/rb_gem_installer/ruby_config.rb",
        include_str!("../../../gems/rb-gem-installer/lib/rb_gem_installer/ruby_config.rb"),
    ),
    (
        "exe/rb-ruby-config",
        include_str!("../../../gems/rb-gem-installer/exe/rb-ruby-config"),
    ),
];

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RubyConfig {
    pub(crate) executable: PathBuf,
    pub(crate) version: String,
    pub(crate) config: BTreeMap<String, String>,
    pub(crate) linker: Vec<String>,
    pub(crate) libraries: Vec<String>,
}

impl RubyConfig {
    pub(crate) fn read(options: &BuildOptions) -> Result<Self> {
        let library = tempfile::Builder::new()
            .prefix("rb-ruby-config-")
            .tempdir()?;
        for (name, source) in FILES {
            let path = library.path().join(name);
            fs::create_dir_all(path.parent().context("Missing Ruby helper directory")?)?;
            fs::write(path, source)?;
        }
        let mut command = Command::new(&options.ruby);
        command
            .args(["--disable=gems", "-I"])
            .arg(library.path().join("lib"))
            .arg(library.path().join("exe/rb-ruby-config"));
        command
            .envs(&options.environment)
            .env_remove("RUBYOPT")
            .env_remove("RUBYLIB")
            .env_remove("RUBYGEMS_GEMDEPS");
        let output = build_process::run(
            command,
            Path::new(""),
            BuildStage::Configure,
            options.timeout,
            &mut |_| {},
        )?;
        serde_json::from_slice(&output.stdout).context("Reading Ruby build configuration")
    }

    pub(crate) fn get(&self, key: &str) -> &str {
        self.config.get(key).map_or("", String::as_str)
    }
}
