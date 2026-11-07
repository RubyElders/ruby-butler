use crate::config::locator::{EnvReader, StdEnvReader};
use anyhow::{Context, Result};
use std::path::PathBuf;

pub(crate) fn directory() -> Result<PathBuf> {
    locate(&StdEnvReader, home::home_dir())
}

fn locate(environment: &dyn EnvReader, home: Option<PathBuf>) -> Result<PathBuf> {
    #[cfg(windows)]
    let variable = "LOCALAPPDATA";
    #[cfg(not(windows))]
    let variable = "XDG_CACHE_HOME";
    if let Ok(value) = environment.var(variable) {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            return Ok(path.join("rb"));
        }
    }
    let home = home.context("Unable to locate the user cache directory")?;
    #[cfg(target_os = "macos")]
    let base = home.join("Library/Caches");
    #[cfg(windows)]
    let base = home.join("AppData/Local");
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let base = home.join(".cache");
    Ok(base.join("rb"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Environment(Option<String>);
    impl EnvReader for Environment {
        fn var(&self, _: &str) -> Result<String, std::env::VarError> {
            self.0.clone().ok_or(std::env::VarError::NotPresent)
        }
    }

    #[test]
    fn absolute_cache_override_does_not_require_a_home() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().to_str().unwrap();
        assert_eq!(
            locate(&Environment(Some(path.into())), None).unwrap(),
            root.path().join("rb")
        );
    }

    #[test]
    fn relative_and_empty_overrides_use_the_home_fallback() {
        let root = tempfile::tempdir().unwrap();
        let fallback = locate(&Environment(None), Some(root.path().into())).unwrap();
        for value in ["", "relative/cache"] {
            assert_eq!(
                locate(&Environment(Some(value.into())), Some(root.path().into())).unwrap(),
                fallback
            );
        }
        assert!(locate(&Environment(None), None).is_err());
    }
}
