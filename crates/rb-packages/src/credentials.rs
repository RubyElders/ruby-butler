use crate::{
    bundler_credentials,
    constants::{AUTH_CONFIG_DIRECTORY, AUTH_CONFIG_FILE},
    provider::source_url,
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandCredential {
    command: Vec<String>,
    username: String,
}

#[derive(Default, Deserialize)]
struct Config {
    #[serde(default)]
    auth: BTreeMap<String, CommandCredential>,
}

pub(crate) struct Credentials {
    directory: Option<PathBuf>,
    commands: BTreeMap<String, CommandCredential>,
    bundler: bundler_credentials::Settings,
}

fn config_path(environment: &BTreeMap<String, String>) -> Option<PathBuf> {
    let value = |name: &str| {
        environment
            .get(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let base = value("XDG_CONFIG_HOME").or_else(|| {
        if cfg!(windows) {
            value("APPDATA")
        } else {
            value("HOME").map(|home| home.join(".config"))
        }
    })?;
    Some(base.join(AUTH_CONFIG_DIRECTORY).join(AUTH_CONFIG_FILE))
}

impl Credentials {
    pub(crate) fn load(project: &Path) -> Result<Self> {
        Self::from_environment(
            project,
            std::env::vars_os()
                .filter_map(|(key, value)| {
                    Some((key.into_string().ok()?, value.into_string().ok()?))
                })
                .collect(),
        )
    }

    fn from_environment(project: &Path, environment: BTreeMap<String, String>) -> Result<Self> {
        let path = config_path(&environment);
        let config = if let Some(path) = &path {
            match fs::read_to_string(path) {
                Ok(text) => toml::from_str::<Config>(&text)
                    .map_err(|_| anyhow::anyhow!("Invalid rb authentication config"))?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Config::default(),
                Err(error) => return Err(error).context("Cannot read rb authentication config"),
            }
        } else {
            Config::default()
        };
        let mut commands = BTreeMap::new();
        for (source, credential) in config.auth {
            ensure!(
                !credential.command.is_empty() && !credential.command[0].is_empty(),
                "Credential command must not be empty"
            );
            ensure!(
                !credential.username.is_empty() && !credential.username.contains([':', '\r', '\n']),
                "Invalid credential username"
            );
            let source =
                source_url(&source).map_err(|_| anyhow::anyhow!("Invalid credential source"))?;
            ensure!(
                commands.insert(source, credential).is_none(),
                "Duplicate credential source"
            );
        }
        Ok(Self {
            directory: path.and_then(|path| path.parent().map(Path::to_path_buf)),
            commands,
            bundler: bundler_credentials::Settings::from_environment(project, environment)?,
        })
    }

    pub(crate) fn credentials(&self, source: &str) -> Result<Option<String>> {
        let source = source_url(source)?;
        let Some(credential) = self.commands.get(&source) else {
            return Ok(self.bundler.credentials(&source)?.map(str::to_owned));
        };
        let output = Command::new(&credential.command[0])
            .args(&credential.command[1..])
            .current_dir(
                self.directory
                    .as_ref()
                    .context("Missing authentication config directory")?,
            )
            .stdin(Stdio::null())
            .output()
            .map_err(|_| anyhow::anyhow!("Cannot run credential command"))?;
        ensure!(output.status.success(), "Credential command failed");
        let token = std::str::from_utf8(&output.stdout)
            .map_err(|_| anyhow::anyhow!("Credential command returned invalid UTF-8"))?
            .trim_end_matches(['\r', '\n']);
        ensure!(
            !token.is_empty() && !token.contains(['\r', '\n']),
            "Credential command must return one nonempty token line"
        );
        Ok(Some(format!("{}:{token}", credential.username)))
    }
}

#[cfg(test)]
mod tests;
