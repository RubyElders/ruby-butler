use crate::GemMetadata;
use anyhow::{Context, Result, ensure};
use rb_gem_types::{PackageId, Platform, Requirement};
use serde_yaml_ng::Value;
use std::collections::BTreeMap;

pub(crate) fn untag(mut value: &Value) -> &Value {
    while let Value::Tagged(tagged) = value {
        value = &tagged.value;
    }
    value
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    untag(value).get(key).map(untag).unwrap_or(&Value::Null)
}

fn version(value: &Value) -> Result<&str> {
    untag(value)
        .as_str()
        .or_else(|| field(value, "version").as_str())
        .context("Missing gem version")
}

fn strings(value: &Value) -> Result<Vec<String>> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    untag(value)
        .as_sequence()
        .context("Expected metadata array")?
        .iter()
        .map(|value| {
            Ok(untag(value)
                .as_str()
                .context("Expected metadata string")?
                .into())
        })
        .collect()
}

fn requirement(value: &Value) -> Result<Requirement> {
    if value.is_null() {
        return Ok(Requirement::default());
    }
    if let Some(text) = untag(value).as_str() {
        return Ok(text.parse()?);
    }
    let pairs = field(value, "requirements")
        .as_sequence()
        .context("Missing gem requirements")?;
    ensure!(!pairs.is_empty(), "Empty gem requirements");
    let constraints = pairs
        .iter()
        .map(|pair| {
            let pair = untag(pair)
                .as_sequence()
                .context("Invalid requirement pair")?;
            ensure!(pair.len() == 2, "Invalid requirement pair length");
            Ok(format!(
                "{} {}",
                untag(&pair[0])
                    .as_str()
                    .context("Missing requirement operator")?,
                version(&pair[1])?
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(constraints.join(", ").parse()?)
}

fn name(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c)),
        "Invalid gem name"
    );
    Ok(())
}

pub(crate) fn parse(bytes: &[u8]) -> Result<GemMetadata> {
    let document: Value = serde_yaml_ng::from_slice(bytes).context("Reading gem metadata")?;
    let spec = untag(&document);
    ensure!(spec.is_mapping(), "Gem metadata must be a mapping");
    let gem_name = field(spec, "name").as_str().context("Missing gem name")?;
    name(gem_name)?;
    let platform = field(spec, "platform");
    let platform = if platform.is_null() {
        "ruby".into()
    } else if let Some(text) = platform.as_str() {
        text.into()
    } else {
        ensure!(platform.is_mapping(), "Invalid gem platform");
        for key in ["cpu", "os", "version"] {
            let component = field(platform, key);
            ensure!(
                component.is_null() || component.as_str().is_some(),
                "Invalid platform component: {key}"
            );
        }
        ensure!(
            field(platform, "os").as_str().is_some(),
            "Missing platform OS"
        );
        ["cpu", "os", "version"]
            .into_iter()
            .filter_map(|key| field(platform, key).as_str())
            .collect::<Vec<_>>()
            .join("-")
    };
    let mut dependencies = BTreeMap::<String, String>::new();
    let declared = field(spec, "dependencies");
    if !declared.is_null() {
        for dependency in declared
            .as_sequence()
            .context("Expected dependency array")?
        {
            let kind = field(dependency, "type");
            let kind = if kind.is_null() {
                "runtime"
            } else {
                kind.as_str()
                    .context("Invalid dependency type")?
                    .trim_start_matches(':')
            };
            ensure!(
                matches!(kind, "runtime" | "development"),
                "Unknown dependency type"
            );
            if kind == "development" {
                continue;
            }
            let dep_name = field(dependency, "name")
                .as_str()
                .context("Missing dependency name")?;
            name(dep_name)?;
            let constraint = requirement(field(dependency, "requirement"))?.to_string();
            dependencies
                .entry(dep_name.into())
                .and_modify(|value| {
                    value.push_str(", ");
                    value.push_str(&constraint);
                })
                .or_insert(constraint);
        }
    }
    let metadata = field(spec, "metadata");
    let metadata = if metadata.is_null() {
        BTreeMap::new()
    } else {
        metadata
            .as_mapping()
            .context("Expected metadata map")?
            .iter()
            .map(|(key, value)| {
                Ok((
                    untag(key).as_str().context("Invalid metadata key")?.into(),
                    untag(value)
                        .as_str()
                        .context("Invalid metadata value")?
                        .into(),
                ))
            })
            .collect::<Result<_>>()?
    };
    Ok(GemMetadata {
        id: PackageId {
            name: gem_name.into(),
            version: version(field(spec, "version"))?.parse()?,
            platform: Platform::parse(&platform),
        },
        dependencies: dependencies
            .into_iter()
            .map(|(name, value)| Ok((name, value.parse()?)))
            .collect::<Result<_>>()?,
        ruby: requirement(field(spec, "required_ruby_version"))?,
        rubygems: requirement(field(spec, "required_rubygems_version"))?,
        require_paths: if field(spec, "require_paths").is_null() {
            vec!["lib".into()]
        } else {
            strings(field(spec, "require_paths"))?
        },
        executables: strings(field(spec, "executables"))?,
        bindir: if field(spec, "bindir").is_null() {
            "bin".into()
        } else {
            field(spec, "bindir")
                .as_str()
                .context("Invalid bindir")?
                .into()
        },
        extensions: strings(field(spec, "extensions"))?,
        files: strings(field(spec, "files"))?,
        metadata,
    })
}
