use crate::runtime_helpers::CommandContext;
use anyhow::{Context, Result};
use rb_core::ruby::GemfileDetector;
use rb_ruby_installer::Request;
use std::{fs, path::Path};

pub(crate) fn resolve(context: &CommandContext, version: Option<&str>) -> Result<Request> {
    let configured = context.config().ruby_version_for_runtime();
    if let Some(version) = version.or(configured.as_deref()) {
        return version.parse();
    }
    let cwd = context.process.directory();
    let file = context.projects().ruby_input.clone();
    let root = file.as_ref().and_then(|file| file.parent()).unwrap_or(cwd);
    let version_file = root.join(".ruby-version");
    if version_file.is_file() {
        return fs::read_to_string(&version_file)?
            .trim()
            .parse()
            .with_context(|| format!("Invalid {}", version_file.display()));
    }
    if let Some(file) = &file {
        if matches!(
            file.extension().and_then(|extension| extension.to_str()),
            Some("toml" | "kdl")
        ) {
            if let Some(requirement) = manifest_requirement(file)? {
                return requirement.parse();
            }
            for name in ["gems.rb", "Gemfile"] {
                if let Some(requirement) = GemfileDetector::literal_requirement(&root.join(name))? {
                    return requirement.parse();
                }
            }
        } else if let Some(requirement) = GemfileDetector::literal_requirement(file)? {
            return requirement.parse();
        }
    }
    "latest".parse()
}

fn manifest_requirement(path: &Path) -> Result<Option<String>> {
    let contents = fs::read_to_string(path)?;
    if path.extension().is_some_and(|extension| extension == "kdl") {
        let document: kdl::KdlDocument = contents.parse()?;
        let node = document
            .get("bundle")
            .and_then(|node| node.children())
            .and_then(|children| children.get("ruby"));
        return node
            .filter(|node| !node.entries().is_empty())
            .map(|node| {
                node.entries()
                    .iter()
                    .map(|entry| {
                        entry
                            .value()
                            .as_string()
                            .context("bundle ruby requires strings")
                    })
                    .collect::<Result<Vec<_>>>()
                    .map(|parts| parts.join(", "))
            })
            .transpose();
    }
    let value: toml::Value = contents.parse()?;
    let Some(value) = value.get("bundle").and_then(|bundle| bundle.get("ruby")) else {
        return Ok(None);
    };
    if let Some(value) = value.as_str() {
        return Ok(Some(value.into()));
    }
    let values = value
        .as_array()
        .context("bundle.ruby must be a string or array of strings")?;
    if values.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        values
            .iter()
            .map(|value| value.as_str().context("bundle.ruby requires strings"))
            .collect::<Result<Vec<_>>>()?
            .join(", "),
    ))
}
