use crate::InstalledGem;
use serde::Serialize;
use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
};

const FILES: &[(&str, &str)] = &[
    (
        "microgems.rb",
        include_str!("../../../gems/rb-microgems/lib/microgems.rb"),
    ),
    (
        "microgems/command.rb",
        include_str!("../../../gems/rb-microgems/lib/microgems/command.rb"),
    ),
    (
        "microgems/ext/builder.rb",
        include_str!("../../../gems/rb-microgems/lib/microgems/ext/builder.rb"),
    ),
    (
        "microgems/package.rb",
        include_str!("../../../gems/rb-microgems/lib/microgems/package.rb"),
    ),
    (
        "microgems/platform.rb",
        include_str!("../../../gems/rb-microgems/lib/microgems/platform.rb"),
    ),
    (
        "microgems/requirement.rb",
        include_str!("../../../gems/rb-microgems/lib/microgems/requirement.rb"),
    ),
    (
        "microgems/specification.rb",
        include_str!("../../../gems/rb-microgems/lib/microgems/specification.rb"),
    ),
    (
        "microgems/version.rb",
        include_str!("../../../gems/rb-microgems/lib/microgems/version.rb"),
    ),
    (
        "microgems/compatibility.rb",
        include_str!("../../../gems/rb-microgems/lib/microgems/compatibility.rb"),
    ),
    (
        "rubygems.rb",
        include_str!("../../../gems/rb-microgems/lib/rubygems.rb"),
    ),
    (
        "rubygems/command.rb",
        include_str!("../../../gems/rb-microgems/lib/rubygems/command.rb"),
    ),
    (
        "rubygems/ext.rb",
        include_str!("../../../gems/rb-microgems/lib/rubygems/ext.rb"),
    ),
    (
        "rubygems/ext/builder.rb",
        include_str!("../../../gems/rb-microgems/lib/rubygems/ext/builder.rb"),
    ),
    (
        "rubygems/package.rb",
        include_str!("../../../gems/rb-microgems/lib/rubygems/package.rb"),
    ),
    (
        "rubygems/platform.rb",
        include_str!("../../../gems/rb-microgems/lib/rubygems/platform.rb"),
    ),
    (
        "rubygems/requirement.rb",
        include_str!("../../../gems/rb-microgems/lib/rubygems/requirement.rb"),
    ),
    (
        "rubygems/specification.rb",
        include_str!("../../../gems/rb-microgems/lib/rubygems/specification.rb"),
    ),
    (
        "rubygems/version.rb",
        include_str!("../../../gems/rb-microgems/lib/rubygems/version.rb"),
    ),
];

pub struct Environment {
    directory: tempfile::TempDir,
    manifest: PathBuf,
}

#[derive(Serialize)]
struct Manifest<'a> {
    gems: &'a [InstalledGem],
    arguments: Vec<&'a str>,
}

impl Environment {
    pub fn new(gems: &[InstalledGem], arguments: &[OsString]) -> io::Result<Self> {
        let arguments = arguments
            .iter()
            .map(|value| {
                value.to_str().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Build argument is not UTF-8")
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        let directory = tempfile::Builder::new().prefix("rb-microgems-").tempdir()?;
        for (name, source) in FILES {
            let path = directory.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, source)?;
        }
        let manifest = directory.path().join("manifest.json");
        std::fs::write(
            &manifest,
            serde_json::to_vec(&Manifest { gems, arguments }).map_err(io::Error::other)?,
        )?;
        Ok(Self {
            directory,
            manifest,
        })
    }

    pub fn load_path(&self) -> &Path {
        self.directory.path()
    }
    pub fn manifest_path(&self) -> &Path {
        &self.manifest
    }
}
