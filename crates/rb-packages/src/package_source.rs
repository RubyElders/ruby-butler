use anyhow::{Context, Result, ensure};
use rb_lockfile::{Source, SourceKind};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PackageSource {
    Registry(String),
    Git {
        remote: String,
        revision: String,
        options: BTreeMap<String, String>,
    },
    Path {
        directory: PathBuf,
        glob: Option<String>,
    },
}

impl From<String> for PackageSource {
    fn from(value: String) -> Self {
        Self::Registry(value)
    }
}
impl From<&str> for PackageSource {
    fn from(value: &str) -> Self {
        Self::Registry(value.into())
    }
}

impl PackageSource {
    pub fn lock_source(&self) -> Source {
        match self {
            Self::Registry(url) => Source::new(SourceKind::Registry, url),
            Self::Git {
                remote,
                revision,
                options,
            } => {
                let mut source = Source::new(SourceKind::Git, remote);
                source.options = options.clone();
                source.options.insert("revision".into(), revision.clone());
                source
            }
            Self::Path { directory, glob } => {
                let mut source = Source::new(SourceKind::Path, directory.to_string_lossy());
                if let Some(glob) = glob {
                    source.options.insert("glob".into(), glob.clone());
                }
                source
            }
        }
    }

    pub(crate) fn from_lock(source: &Source, remote: &str, root: Option<&Path>) -> Result<Self> {
        Ok(match source.kind {
            SourceKind::Registry => Self::Registry(crate::provider::source_url(remote)?),
            SourceKind::Path => Self::Path {
                directory: {
                    let path = root
                        .map(Path::to_owned)
                        .unwrap_or(std::env::current_dir()?)
                        .join(remote);
                    dunce::canonicalize(&path).unwrap_or(path)
                },
                glob: normalized_glob(source.options.get("glob").map(String::as_str)),
            },
            SourceKind::Git => {
                let mut options = source.options.clone();
                let revision = options
                    .remove("revision")
                    .context("Git lock has no revision")?;
                if options
                    .get("glob")
                    .is_some_and(|value| value == crate::constants::DEFAULT_GEMSPEC_GLOB)
                {
                    options.remove("glob");
                }
                Self::Git {
                    remote: remote.trim_end_matches('/').into(),
                    revision,
                    options,
                }
            }
        })
    }

    pub(crate) fn matches(
        &self,
        dependency: &rb_gemfile::Dependency,
        default: Option<&str>,
    ) -> Result<bool> {
        Ok(match self {
            Self::Registry(url) => {
                dependency.git.is_none()
                    && dependency.path.is_none()
                    && *url
                        == crate::provider::source_url(
                            dependency
                                .source
                                .as_deref()
                                .or(default)
                                .unwrap_or(crate::constants::DEFAULT_SOURCE),
                        )?
            }
            Self::Git {
                remote, options, ..
            } => {
                dependency
                    .git
                    .as_ref()
                    .is_some_and(|git| git.trim_end_matches('/') == remote)
                    && *options == git_options(dependency)
            }
            Self::Path { directory, glob } => {
                dependency
                    .path
                    .as_ref()
                    .is_some_and(|path| Path::new(path) == directory)
                    && glob == &normalized_glob(dependency.glob.as_deref())
            }
        })
    }

    pub(crate) fn validate(&self) -> Result<()> {
        match self {
            Self::Registry(url) => ensure!(
                crate::provider::source_url(url)? == *url,
                "Registry source must be normalized"
            ),
            Self::Git {
                remote, revision, ..
            } => {
                ensure!(
                    !remote.is_empty()
                        && !remote.starts_with('-')
                        && !remote.chars().any(char::is_control),
                    "Invalid Git remote"
                );
                ensure!(
                    crate::constants::GIT_OBJECT_ID_LENGTHS.contains(&revision.len())
                        && revision.bytes().all(|b| b.is_ascii_hexdigit()),
                    "Invalid Git revision"
                );
            }
            Self::Path { directory, .. } => {
                ensure!(directory.is_absolute(), "Path source must be absolute")
            }
        }
        Ok(())
    }
}

pub(crate) fn git_options(dependency: &rb_gemfile::Dependency) -> BTreeMap<String, String> {
    let mut options = BTreeMap::new();
    for (name, value) in [
        ("branch", &dependency.branch),
        ("tag", &dependency.tag),
        ("ref", &dependency.rev),
        ("glob", &dependency.glob),
    ] {
        if let Some(value) = value
            && (name != "glob" || value != crate::constants::DEFAULT_GEMSPEC_GLOB)
        {
            options.insert(name.into(), value.clone());
        }
    }
    if dependency.submodules == Some(true) {
        options.insert("submodules".into(), "true".into());
    }
    options
}

pub(crate) fn normalized_glob(glob: Option<&str>) -> Option<String> {
    glob.filter(|value| *value != crate::constants::DEFAULT_GEMSPEC_GLOB)
        .map(str::to_owned)
}
