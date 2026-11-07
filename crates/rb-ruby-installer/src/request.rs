use anyhow::{Result, bail};
use semver::Version;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Request {
    text: String,
    selection: Selection,
}

#[derive(Clone, Debug)]
enum Selection {
    Latest,
    Prefix(Vec<u64>),
    Requirement(rb_gem_types::Requirement),
}

impl FromStr for Request {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        let text = text.trim();
        if text.is_empty() {
            bail!("Ruby version must not be empty");
        }
        let prefix = text
            .split('.')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()
            .filter(|parts| (1..=3).contains(&parts.len()));
        let selection = match prefix {
            Some(prefix) => Selection::Prefix(prefix),
            None if text == "latest" => Selection::Latest,
            None => Selection::Requirement(text.parse()?),
        };
        Ok(Self {
            text: text.into(),
            selection,
        })
    }
}

impl Request {
    pub fn matches(&self, version: &Version) -> bool {
        if !version.pre.is_empty() || !version.build.is_empty() {
            return false;
        }
        match &self.selection {
            Selection::Latest => true,
            Selection::Prefix(prefix) => {
                [version.major, version.minor, version.patch].starts_with(prefix)
            }
            Selection::Requirement(requirement) => {
                requirement.matches(&version.to_string().parse().expect("stable Ruby version"))
            }
        }
    }

    pub fn exact_version(&self) -> Option<Version> {
        match &self.selection {
            Selection::Prefix(parts) if parts.len() == 3 => {
                Some(Version::new(parts[0], parts[1], parts[2]))
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}
