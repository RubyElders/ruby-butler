use anyhow::{Context, Result};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub(crate) struct Settings {
    layers: Vec<BTreeMap<String, String>>,
}

fn key(value: &str) -> String {
    format!(
        "BUNDLE_{}",
        value.replace('.', "__").replace('-', "___").to_uppercase()
    )
}

fn read(path: Option<PathBuf>) -> Result<BTreeMap<String, String>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(error).context("Cannot read Bundler credentials config"),
    };
    if text.trim().is_empty() {
        return Ok(BTreeMap::new());
    }
    let values: BTreeMap<String, serde_yaml_ng::Value> = serde_yaml_ng::from_str(&text)
        .map_err(|_| anyhow::anyhow!("Invalid Bundler credentials config"))?;
    Ok(values
        .into_iter()
        .filter_map(|(mut name, value)| {
            let value = value.as_str()?.to_owned();
            if (name.to_lowercase().contains("https:") || name.to_lowercase().contains("http:"))
                && !name.ends_with('/')
            {
                name.push('/');
            }
            Some((name.replace('.', "__").replace('-', "___"), value))
        })
        .collect())
}

impl Settings {
    pub(crate) fn from_environment(
        project: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<Self> {
        let setting = |name: &str| {
            environment
                .get(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };
        let local = project
            .join(setting("BUNDLE_APP_CONFIG").unwrap_or_else(|| ".bundle".into()))
            .join("config");
        let global = setting("BUNDLE_CONFIG")
            .or_else(|| setting("BUNDLE_USER_CONFIG"))
            .or_else(|| setting("BUNDLE_USER_HOME").map(|home| home.join("config")))
            .or_else(|| {
                setting("HOME")
                    .or_else(|| setting("USERPROFILE"))
                    .map(|home| home.join(".bundle/config"))
            });
        let ignore = environment.contains_key("BUNDLE_IGNORE_CONFIG");
        Ok(Self {
            layers: vec![
                read((!ignore).then_some(local))?,
                environment,
                read(if ignore { None } else { global })?,
            ],
        })
    }

    pub(crate) fn credentials(&self, source: &str) -> Result<Option<&str>> {
        let url = url::Url::parse(source)?;
        for name in [source, url.host_str().context("Gem source has no host")?] {
            let key = key(name);
            if let Some(value) = self.layers.iter().find_map(|layer| layer.get(&key)) {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_follow_bundler_precedence_and_source_scope() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".bundle")).unwrap();
        fs::write(
            root.path().join(".bundle/config"),
            "BUNDLE_GEMS__EXAMPLE___COM: local\n",
        )
        .unwrap();
        let global = root.path().join("global");
        fs::write(&global, "BUNDLE_GEMS__EXAMPLE___COM: global\nBUNDLE_HTTPS://GEMS__EXAMPLE___COM/TEAM/: user:source-token\n").unwrap();
        let mut env = BTreeMap::from([
            ("BUNDLE_CONFIG".into(), global.to_str().unwrap().into()),
            ("BUNDLE_GEMS__EXAMPLE___COM".into(), "environment".into()),
        ]);
        let settings = Settings::from_environment(root.path(), env.clone()).unwrap();
        assert_eq!(
            settings.credentials("https://gems.example-com/").unwrap(),
            Some("local")
        );
        assert_eq!(
            settings
                .credentials("https://gems.example-com/team/")
                .unwrap(),
            Some("user:source-token")
        );
        assert_eq!(
            settings.credentials("https://other.example/").unwrap(),
            None
        );
        env.insert("BUNDLE_IGNORE_CONFIG".into(), "1".into());
        let settings = Settings::from_environment(root.path(), env).unwrap();
        assert_eq!(
            settings
                .credentials("https://gems.example-com/team/")
                .unwrap(),
            Some("environment")
        );
    }

    #[test]
    fn malformed_config_does_not_disclose_its_contents() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".bundle")).unwrap();
        fs::write(root.path().join(".bundle/config"), "[secret-token").unwrap();
        let error = Settings::from_environment(root.path(), BTreeMap::new())
            .err()
            .unwrap();
        assert_eq!(error.to_string(), "Invalid Bundler credentials config");
    }

    #[test]
    fn config_paths_and_environment_overrides_follow_bundler() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        fs::create_dir_all(home.join(".bundle")).unwrap();
        fs::write(
            home.join(".bundle/config"),
            "BUNDLE_GEMS__EXAMPLE: user:global\n",
        )
        .unwrap();
        let mut env = BTreeMap::from([("HOME".into(), home.to_str().unwrap().into())]);
        let credentials = |env| {
            Settings::from_environment(root.path(), env)
                .unwrap()
                .credentials("https://gems.example/")
                .unwrap()
                .map(str::to_owned)
        };
        assert_eq!(credentials(env.clone()).as_deref(), Some("user:global"));
        for variable in ["BUNDLE_USER_HOME", "BUNDLE_USER_CONFIG", "BUNDLE_CONFIG"] {
            let directory = root.path().join(variable);
            fs::create_dir(&directory).unwrap();
            let file = directory.join("config");
            fs::write(&file, format!("BUNDLE_GEMS__EXAMPLE: {variable}\n")).unwrap();
            env.insert(
                variable.into(),
                if variable == "BUNDLE_USER_HOME" {
                    directory
                } else {
                    file
                }
                .to_str()
                .unwrap()
                .into(),
            );
            assert_eq!(credentials(env.clone()).as_deref(), Some(variable));
        }
        env.insert("BUNDLE_GEMS__EXAMPLE".into(), "user:environment".into());
        assert_eq!(
            credentials(env.clone()).as_deref(),
            Some("user:environment")
        );
        for location in [
            PathBuf::from("custom-config"),
            root.path().join("absolute-config"),
        ] {
            let directory = root.path().join(&location);
            fs::create_dir(&directory).unwrap();
            fs::write(
                directory.join("config"),
                "BUNDLE_GEMS__EXAMPLE: user:local\n",
            )
            .unwrap();
            env.insert(
                "BUNDLE_APP_CONFIG".into(),
                location.to_str().unwrap().into(),
            );
            assert_eq!(credentials(env.clone()).as_deref(), Some("user:local"));
        }
    }
}
