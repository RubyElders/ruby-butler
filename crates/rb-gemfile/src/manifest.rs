use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use toml::Value;

/// Normalized declarations, not resolved versions or an installation lockfile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Manifest {
    pub bundle: Bundle,
    pub dependencies: BTreeMap<String, Dependency>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gemspecs: Vec<Package>,
}

fn remove_null_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(table) => {
            table.retain(|key, value| {
                !value.is_null()
                    || !matches!(
                        key.as_str(),
                        "source"
                            | "engine"
                            | "engine_version"
                            | "patchlevel"
                            | "lockfile"
                            | "require"
                            | "git"
                            | "path"
                            | "branch"
                            | "tag"
                            | "rev"
                            | "glob"
                            | "submodules"
                            | "force_ruby_platform"
                    )
            });
            for value in table.values_mut() {
                remove_null_fields(value);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                remove_null_fields(value);
            }
        }
        _ => {}
    }
}

impl<'de> Deserialize<'de> for Manifest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            #[serde(default)]
            bundle: Bundle,
            #[serde(deserialize_with = "crate::format::deserialize_dependencies")]
            dependencies: BTreeMap<String, Dependency>,
            #[serde(default)]
            gemspecs: Vec<Package>,
        }
        let mut value = serde_json::Value::deserialize(deserializer)?;
        remove_null_fields(&mut value);
        let root = Value::try_from(value)
            .map_err(serde::de::Error::custom)?
            .as_table()
            .cloned()
            .ok_or_else(|| serde::de::Error::custom("manifest must be a table"))?;
        let root = crate::sources::normalize_document(root).map_err(serde::de::Error::custom)?;
        let wire: Wire = Value::Table(root)
            .try_into()
            .map_err(serde::de::Error::custom)?;
        Ok(Self {
            bundle: wire.bundle,
            dependencies: wire.dependencies,
            gemspecs: wire.gemspecs,
        })
    }
}

impl Manifest {
    /// Reads dependency declarations and package metadata from KDL.
    pub fn from_kdl(input: &str) -> std::io::Result<Self> {
        crate::kdl::parse(input)?
            .try_into()
            .map_err(std::io::Error::other)
    }

    pub fn to_kdl(&self) -> std::io::Result<String> {
        let value = crate::project::render_normalized(self).map_err(std::io::Error::other)?;
        crate::kdl::render(value)
    }

    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        crate::project::render(self)
    }
}

/// Local package metadata needed without evaluating its gemspec again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub path: String,
    pub required_ruby_version: Vec<String>,
    pub required_rubygems_version: Vec<String>,
    pub runtime_dependencies: BTreeMap<String, Vec<String>>,
    pub development_dependencies: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Bundle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(with = "requirements")]
    pub ruby: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patchlevel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lockfile: Option<String>,
    pub optional_groups: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Dependency {
    #[serde(with = "requirements")]
    pub version: Vec<String>,
    pub groups: Vec<String>,
    pub platforms: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require: Option<Require>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(alias = "ref")]
    pub rev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glob: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submodules: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_ruby_platform: Option<bool>,
    pub enabled: bool,
    /// Additional declarations of this gem with distinct activation settings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<Dependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Require {
    Enabled(bool),
    Name(String),
    Names(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plugin {
    pub name: String,
    #[serde(with = "requirements")]
    pub version: Vec<String>,
}

impl Default for Dependency {
    fn default() -> Self {
        Self {
            version: vec![">= 0".into()],
            groups: vec!["default".into()],
            platforms: Vec::new(),
            require: None,
            source: None,
            git: None,
            path: None,
            branch: None,
            tag: None,
            rev: None,
            glob: None,
            submodules: None,
            force_ruby_platform: None,
            enabled: true,
            variants: Vec::new(),
        }
    }
}

mod requirements {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(value: &[String], serializer: S) -> Result<S::Ok, S::Error> {
        if let [requirement] = value {
            requirement.serialize(serializer)
        } else {
            value.serialize(serializer)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<String>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Requirements {
            One(String),
            Many(Vec<String>),
        }
        Ok(match Requirements::deserialize(deserializer)? {
            Requirements::One(value) => vec![value],
            Requirements::Many(values) => values,
        })
    }
}

impl Dependency {
    pub fn validate(&self) -> Result<(), String> {
        if self.version.is_empty() || self.version.iter().any(|value| value.trim().is_empty()) {
            return Err("Version requirements must not be empty".into());
        }
        if self
            .groups
            .iter()
            .chain(&self.platforms)
            .any(|value| value.is_empty())
        {
            return Err("Groups and platforms must not contain empty names".into());
        }
        if [&self.git, &self.path, &self.source]
            .iter()
            .filter(|value| value.is_some())
            .count()
            > 1
        {
            return Err("Git, path and registry sources are mutually exclusive".into());
        }
        let selectors = [&self.branch, &self.tag, &self.rev];
        if selectors.iter().filter(|value| value.is_some()).count() > 1 {
            return Err("Specify only one of branch, tag or rev".into());
        }
        if self.git.is_none()
            && (selectors.iter().any(|value| value.is_some()) || self.submodules.is_some())
        {
            return Err("Git options require a Git source".into());
        }
        for value in [
            &self.git,
            &self.path,
            &self.source,
            &self.branch,
            &self.tag,
            &self.rev,
        ]
        .into_iter()
        .flatten()
        {
            if value.is_empty() || value.chars().any(char::is_control) {
                return Err("Source locations and selectors must not be empty or contain control characters".into());
            }
        }
        for variant in &self.variants {
            variant.validate()?;
        }
        Ok(())
    }
}
