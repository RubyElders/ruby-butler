use crate::validation::{validate_checksum, validate_name};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GemInfo {
    pub version: String,
    pub dependencies: BTreeMap<String, String>,
    pub metadata: BTreeMap<String, String>,
}

pub fn parse(name: &str, body: &str) -> Result<Vec<GemInfo>> {
    validate_name(name)?;
    ensure!(!body.trim().is_empty(), "Empty compact index for {name}");
    let mut lines = body.lines();
    if body.lines().any(|line| line == "---") {
        for line in lines.by_ref() {
            if line == "---" {
                break;
            }
        }
    }
    let mut gems = Vec::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let (version, rest) = line
            .split_once(' ')
            .context("Missing version in compact index")?;
        validate_name(version)?;
        let (dependencies, metadata) = rest
            .split_once('|')
            .context("Missing compact-index metadata")?;
        let mut deps = BTreeMap::new();
        for dep in dependencies.split(',').filter(|d| !d.is_empty()) {
            let (name, constraint) = dep
                .split_once(':')
                .context("Invalid compact-index dependency")?;
            validate_name(name)?;
            // Historical index entries repeat dependencies; combine requirements as Bundler does.
            deps.entry(name.to_string())
                .and_modify(|existing: &mut String| {
                    existing.push_str(", ");
                    existing.push_str(&constraint.replace('&', ", "));
                })
                .or_insert_with(|| constraint.replace('&', ", "));
        }
        let mut fields = BTreeMap::new();
        for field in metadata.split(',') {
            let (key, value) = field
                .split_once(':')
                .context("Invalid compact-index field")?;
            if key == "checksum" {
                validate_checksum(value)?;
            }
            ensure!(
                fields.insert(key.to_string(), value.to_string()).is_none(),
                "Duplicate compact-index field {key}"
            );
        }
        ensure!(fields.contains_key("checksum"), "Missing gem checksum");
        gems.push(GemInfo {
            version: version.to_string(),
            dependencies: deps,
            metadata: fields,
        });
    }
    Ok(gems)
}
