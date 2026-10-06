use crate::Manifest;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process::Command,
};

/// Executes a trusted Gemfile using the selected Ruby and its RubyGems API.
pub struct Evaluator {
    pub(crate) ruby: PathBuf,
    pub(crate) environment: BTreeMap<OsString, OsString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    pub manifest: Manifest,
    pub inputs: Vec<PathBuf>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
}

impl Evaluator {
    pub fn new(ruby: impl Into<PathBuf>) -> Self {
        Self {
            ruby: ruby.into(),
            environment: BTreeMap::new(),
        }
    }

    pub fn env(mut self, name: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.environment
            .insert(name.as_ref().to_owned(), value.as_ref().to_owned());
        self
    }

    pub fn evaluate(&self, gemfile: impl AsRef<Path>) -> io::Result<Evaluation> {
        let gemfile = gemfile_path(gemfile.as_ref())?;
        let directory = tempfile::tempdir()?;
        let result = directory.path().join("manifest.json");
        for (name, source) in SOURCES {
            std::fs::write(directory.path().join(name), source)?;
        }
        let ruby = self.ruby_executable()?;
        let output = Command::new(ruby)
            .envs(&self.environment)
            .env_remove("RUBYOPT")
            .arg(directory.path().join("evaluate.rb"))
            .arg(&gemfile)
            .arg(&result)
            .current_dir(gemfile.parent().unwrap())
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "Gemfile evaluation failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let mut evaluation: Evaluation =
            serde_json::from_slice(&std::fs::read(result).map_err(|error| {
                io::Error::other(format!("Ruby did not produce a Gemfile manifest: {error}"))
            })?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        evaluation.stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        evaluation.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        evaluation.inputs = evaluation
            .inputs
            .iter()
            .map(dunce::canonicalize)
            .collect::<io::Result<_>>()?;
        for dependency in evaluation.manifest.dependencies.values() {
            dependency.validate().map_err(io::Error::other)?;
        }
        Ok(evaluation)
    }

    pub(crate) fn ruby_executable(&self) -> io::Result<PathBuf> {
        let ruby = which::which_in(
            &self.ruby,
            self.environment
                .get(OsStr::new("PATH"))
                .cloned()
                .or_else(|| std::env::var_os("PATH")),
            std::env::current_dir()?,
        )
        .map_err(|error| io::Error::new(io::ErrorKind::NotFound, error))?;
        dunce::canonicalize(ruby)
    }
}

pub(crate) fn gemfile_path(path: &Path) -> io::Result<PathBuf> {
    let path = dunce::canonicalize(path)?;
    if !std::fs::metadata(&path)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Gemfile must be a file",
        ));
    }
    Ok(path)
}

pub(crate) const SOURCES: &[(&str, &str)] = &[
    ("evaluate.rb", include_str!("../assets/evaluate.rb")),
    ("gemfile.rb", include_str!("../assets/gemfile.rb")),
    ("sources.rb", include_str!("../assets/sources.rb")),
    ("gemspec.rb", include_str!("../assets/gemspec.rb")),
    (
        "compatibility.rb",
        include_str!("../assets/compatibility.rb"),
    ),
];
