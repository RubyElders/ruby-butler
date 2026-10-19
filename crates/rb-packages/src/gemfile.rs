use anyhow::Result;
use rb_gemfile::{Manifest, Require};

pub(crate) fn render(manifest: &Manifest) -> Result<String> {
    let needs_default = manifest.dependencies.values().any(|dependency| {
        std::iter::once(dependency)
            .chain(&dependency.variants)
            .any(|declaration| {
                declaration.enabled
                    && declaration.source.is_none()
                    && declaration.git.is_none()
                    && declaration.path.is_none()
            })
    });
    let mut output = String::new();
    if let Some(source) = manifest
        .bundle
        .source
        .as_deref()
        .or_else(|| needs_default.then_some(crate::constants::DEFAULT_SOURCE))
    {
        output.push_str(&format!("source {}\n", literal(source)));
    }
    if !manifest.bundle.ruby.is_empty() {
        let mut arguments: Vec<_> = manifest.bundle.ruby.iter().map(|v| literal(v)).collect();
        for (key, value) in [
            ("engine", &manifest.bundle.engine),
            ("engine_version", &manifest.bundle.engine_version),
            ("patchlevel", &manifest.bundle.patchlevel),
        ] {
            if let Some(value) = value {
                arguments.push(format!("{key}: {}", literal(value)));
            }
        }
        output.push_str(&format!("ruby {}\n", arguments.join(", ")));
    }
    for group in &manifest.bundle.optional_groups {
        output.push_str(&format!(
            "group :{}, optional: true do\nend\n",
            literal(group)
        ));
    }
    for (name, dependency) in &manifest.dependencies {
        for declaration in std::iter::once(dependency)
            .chain(&dependency.variants)
            .filter(|d| d.enabled)
        {
            declaration.validate().map_err(anyhow::Error::msg)?;

            let mut arguments = vec![literal(name)];
            arguments.extend(declaration.version.iter().map(|v| literal(v)));
            arguments.push(format!(
                "groups: [{}]",
                declaration
                    .groups
                    .iter()
                    .map(|v| literal(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            if !declaration.platforms.is_empty() {
                arguments.push(format!(
                    "platforms: [{}]",
                    declaration
                        .platforms
                        .iter()
                        .map(|v| literal(v))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if let Some(source) = &declaration.source {
                arguments.push(format!("source: {}", literal(source)));
            }
            for (key, value) in [
                ("git", &declaration.git),
                ("path", &declaration.path),
                ("branch", &declaration.branch),
                ("tag", &declaration.tag),
                ("ref", &declaration.rev),
                ("glob", &declaration.glob),
            ] {
                if let Some(value) = value {
                    arguments.push(format!("{key}: {}", literal(value)));
                }
            }
            if let Some(value) = declaration.submodules {
                arguments.push(format!("submodules: {value}"));
            }
            if let Some(require) = &declaration.require {
                let value = match require {
                    Require::Enabled(v) => v.to_string(),
                    Require::Name(v) => literal(v),
                    Require::Names(v) => format!(
                        "[{}]",
                        v.iter().map(|v| literal(v)).collect::<Vec<_>>().join(", ")
                    ),
                };
                arguments.push(format!("require: {value}"));
            }
            if let Some(force) = declaration.force_ruby_platform {
                arguments.push(format!("force_ruby_platform: {force}"));
            }
            output.push_str(&format!("gem {}\n", arguments.join(", ")));
        }
    }
    Ok(output)
}

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}
