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
    pub(crate) inherited_environment: BTreeMap<OsString, OsString>,
    pub(crate) directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    pub manifest: Manifest,
    pub inputs: Vec<PathBuf>,
    pub environment: Option<std::collections::BTreeSet<String>>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
}

impl Evaluator {
    pub fn new(ruby: impl Into<PathBuf>) -> Self {
        Self::with_inputs(
            ruby,
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            std::env::vars_os().collect(),
        )
    }

    pub fn with_inputs(
        ruby: impl Into<PathBuf>,
        directory: PathBuf,
        inherited_environment: BTreeMap<OsString, OsString>,
    ) -> Self {
        #[cfg(windows)]
        let inherited_environment = inherited_environment
            .into_iter()
            .map(|(key, value)| {
                let key = key
                    .to_str()
                    .map(|key| OsString::from(key.to_ascii_uppercase()))
                    .unwrap_or(key);
                (key, value)
            })
            .collect();
        Self {
            ruby: ruby.into(),
            directory,
            inherited_environment,
            environment: BTreeMap::new(),
        }
    }

    pub(crate) fn effective_environment(&self) -> BTreeMap<OsString, OsString> {
        let mut environment = self.inherited_environment.clone();
        environment.extend(self.environment.clone());
        environment.remove(OsStr::new("RUBYOPT"));
        environment
    }

    pub fn env(mut self, name: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        let name = name.as_ref().to_owned();
        #[cfg(windows)]
        let name = name
            .to_str()
            .map(|name| OsString::from(name.to_ascii_uppercase()))
            .unwrap_or(name);
        self.environment.insert(name, value.as_ref().to_owned());
        self
    }

    pub fn evaluate(&self, gemfile: impl AsRef<Path>) -> io::Result<Evaluation> {
        let gemfile = gemfile_path(&self.directory.join(gemfile.as_ref()))?;
        let directory = tempfile::tempdir()?;
        let result = directory.path().join("manifest.json");
        for (name, source) in SOURCES {
            let path = directory.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, source)?;
        }
        let ruby = self.ruby_executable()?;
        let output = Command::new(ruby)
            .env_clear()
            .envs(self.effective_environment())
            .arg(directory.path().join("exe/rb-evaluate-gemfile"))
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
        let environment = self.effective_environment();
        let search_path = environment
            .get(OsStr::new("PATH"))
            .map(|path| {
                std::env::join_paths(
                    std::env::split_paths(path).map(|part| self.directory.join(part)),
                )
            })
            .transpose()
            .map_err(io::Error::other)?;
        let ruby = which::which_in(&self.ruby, search_path, &self.directory)
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
    (
        "lib/rb_gemfile/file_inputs.rb",
        include_str!("../../../gems/rb-gemfile/lib/rb_gemfile/file_inputs.rb"),
    ),
    (
        "lib/rb_gemfile/environment_inputs.rb",
        include_str!("../../../gems/rb-gemfile/lib/rb_gemfile/environment_inputs.rb"),
    ),
    (
        "exe/rb-evaluate-gemfile",
        include_str!("../../../gems/rb-gemfile/exe/rb-evaluate-gemfile"),
    ),
    (
        "lib/rb_gemfile.rb",
        include_str!("../../../gems/rb-gemfile/lib/rb_gemfile.rb"),
    ),
    (
        "lib/rb_gemfile/version.rb",
        include_str!("../../../gems/rb-gemfile/lib/rb_gemfile/version.rb"),
    ),
    (
        "lib/rb_gemfile/gemfile.rb",
        include_str!("../../../gems/rb-gemfile/lib/rb_gemfile/gemfile.rb"),
    ),
    (
        "lib/rb_gemfile/sources.rb",
        include_str!("../../../gems/rb-gemfile/lib/rb_gemfile/sources.rb"),
    ),
    (
        "lib/rb_gemfile/gemspec.rb",
        include_str!("../../../gems/rb-gemfile/lib/rb_gemfile/gemspec.rb"),
    ),
    (
        "lib/rb_gemfile/compatibility.rb",
        include_str!("../../../gems/rb-gemfile/lib/rb_gemfile/compatibility.rb"),
    ),
];
