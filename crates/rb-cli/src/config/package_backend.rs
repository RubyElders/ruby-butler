use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageBackendKind {
    #[default]
    Bundler,
    Native,
}

impl std::fmt::Display for PackageBackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Bundler => "bundler",
            Self::Native => "native",
        })
    }
}

impl PackageBackendKind {
    pub(crate) fn resolve(
        cli: Option<Self>,
        environment: Option<&str>,
        file: Option<Self>,
    ) -> super::ConfigValue<Self> {
        if let Some(value) = cli {
            super::ConfigValue::from_cli(value)
        } else if let Some(value) =
            environment.and_then(|value| <Self as ValueEnum>::from_str(value, false).ok())
        {
            super::ConfigValue::from_env(value)
        } else if let Some(value) = file {
            super::ConfigValue::from_file(value)
        } else {
            super::ConfigValue::default_value(Self::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigSource;

    #[test]
    fn selection_respects_cli_environment_file_and_default_priority() {
        use PackageBackendKind::{Bundler, Native};
        for (cli, environment, file, expected, source) in [
            (
                Some(Bundler),
                Some("native"),
                Some(Native),
                Bundler,
                ConfigSource::Cli,
            ),
            (
                None,
                Some("native"),
                Some(Bundler),
                Native,
                ConfigSource::EnvVar,
            ),
            (None, None, Some(Native), Native, ConfigSource::ConfigFile),
            (None, None, None, Bundler, ConfigSource::Default),
        ] {
            let selected = PackageBackendKind::resolve(cli, environment, file);
            assert_eq!(selected.value, expected);
            assert_eq!(selected.source, source);
        }
    }
}
