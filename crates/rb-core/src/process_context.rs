use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    sync::Arc,
};

/// Immutable process inputs shared by one invocation and its child operations.
#[derive(Clone)]
pub struct ProcessContext {
    directory: PathBuf,
    environment: Arc<BTreeMap<OsString, OsString>>,
    home: Option<PathBuf>,
}

impl ProcessContext {
    pub fn capture() -> std::io::Result<Self> {
        let mut inputs = Self::new(std::env::current_dir()?, std::env::vars_os().collect());
        if inputs.home.is_none() {
            inputs.home = home::home_dir();
        }
        Ok(inputs)
    }

    pub fn new(directory: PathBuf, environment: BTreeMap<OsString, OsString>) -> Self {
        #[cfg(windows)]
        let environment: BTreeMap<OsString, OsString> = environment
            .into_iter()
            .map(|(key, value)| {
                let key = key
                    .to_str()
                    .map(|key| OsString::from(key.to_ascii_uppercase()))
                    .unwrap_or(key);
                (key, value)
            })
            .collect();
        #[cfg(windows)]
        let home_variable = "USERPROFILE";
        #[cfg(not(windows))]
        let home_variable = "HOME";
        let home = environment
            .get(OsStr::new(home_variable))
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        Self {
            directory,
            environment: Arc::new(environment),
            home,
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.environment
    }

    pub fn var(&self, name: impl AsRef<OsStr>) -> Option<&OsStr> {
        #[cfg(windows)]
        let name = name
            .as_ref()
            .to_str()
            .map(|name| OsString::from(name.to_ascii_uppercase()))
            .unwrap_or_else(|| name.as_ref().to_owned());
        #[cfg(not(windows))]
        let name = name.as_ref();
        self.environment
            .get::<OsStr>(name.as_ref())
            .map(OsString::as_os_str)
    }

    pub fn home_directory(&self) -> Option<PathBuf> {
        self.home.clone()
    }

    pub fn at(&self, directory: impl AsRef<Path>) -> Self {
        let directory = self.directory.join(directory);
        let directory = directory.canonicalize().unwrap_or(directory);
        Self {
            directory,
            environment: self.environment.clone(),
            home: self.home.clone(),
        }
    }
}

impl std::fmt::Debug for ProcessContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProcessContext")
            .field("directory", &self.directory)
            .finish_non_exhaustive()
    }
}
