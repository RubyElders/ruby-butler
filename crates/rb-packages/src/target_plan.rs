use crate::RubyEnvironment;
use anyhow::{Result, ensure};
use rb_gem_types::Requirement;
use rb_lockfile::Dependency;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct TargetPlan {
    pub(crate) roots: BTreeMap<String, Requirement>,
    pub(crate) install_roots: BTreeSet<String>,
    pub(crate) sources: BTreeMap<String, rb_gemfile::Dependency>,
    pub(crate) force_ruby: BTreeSet<String>,
    pub(crate) declarations: BTreeMap<String, Dependency>,
}

impl TargetPlan {
    pub(crate) fn new(
        manifest: &rb_gemfile::Manifest,
        runtime: &RubyEnvironment,
        options: &crate::PlanOptions,
    ) -> Result<Self> {
        let mut roots: BTreeMap<String, Requirement> = BTreeMap::new();
        let mut install_roots = BTreeSet::new();
        let mut sources = BTreeMap::new();
        let mut force_ruby = BTreeSet::new();
        let mut declarations = BTreeMap::new();
        for (name, dependency) in &manifest.dependencies {
            for declaration in std::iter::once(dependency).chain(&dependency.variants) {
                declaration.validate().map_err(anyhow::Error::msg)?;
                if !declaration.enabled {
                    continue;
                }
                let applies = platform_applies(&declaration.platforms, runtime)?;

                let constraint: Requirement = declaration.version.join(", ").parse()?;
                let value = if let Some(previous) = roots.get(name) {
                    format!("{previous}, {constraint}").parse()?
                } else {
                    constraint.clone()
                };
                if let Some(previous) = declarations.get(name) {
                    let previous: &Dependency = previous;
                    ensure!(
                        previous.requirement == constraint,
                        "Different requirements across declarations for {name} are not supported"
                    );
                }
                declarations.insert(
                    name.clone(),
                    Dependency {
                        requirement: constraint,
                        pinned: declaration.source.is_some()
                            || declaration.git.is_some()
                            || declaration.path.is_some(),
                    },
                );
                if !applies {
                    continue;
                }
                roots.insert(name.clone(), value);
                if let Some(previous) = sources.insert(name.clone(), declaration.clone()) {
                    ensure!(
                        previous.git == declaration.git
                            && previous.path == declaration.path
                            && previous.source == declaration.source
                            && crate::package_source::git_options(&previous)
                                == crate::package_source::git_options(declaration),
                        "Conflicting sources for {name}"
                    );
                }
                if declaration.force_ruby_platform == Some(true) {
                    force_ruby.insert(name.clone());
                }
                if declaration.groups.iter().any(|g| {
                    !options.without.contains(g)
                        && (!manifest.bundle.optional_groups.contains(g)
                            || options.with.contains(g))
                }) {
                    install_roots.insert(name.clone());
                }
            }
        }
        Ok(Self {
            roots,
            install_roots,
            sources,
            force_ruby,
            declarations,
        })
    }
}

fn platform_applies(platforms: &[String], runtime: &RubyEnvironment) -> Result<bool> {
    if platforms.is_empty() {
        return Ok(true);
    }
    for selector in platforms {
        let (name, version) = selector
            .rsplit_once('_')
            .filter(|(_, suffix)| suffix.chars().all(|c| c.is_ascii_digit()))
            .map_or((selector.as_str(), None), |(name, version)| {
                (name, Some(version))
            });
        let platform = runtime.target();
        let windows = platform.os.starts_with("mingw")
            || platform.os.starts_with("mswin")
            || platform.os == "cygwin";
        let applies = match name {
            "ruby" => {
                !windows
                    && matches!(
                        runtime.engine.as_str(),
                        "ruby" | "rbx" | "maglev" | "truffleruby"
                    )
            }
            "mri" => !windows && runtime.engine == "ruby",
            "jruby" => runtime.engine == "jruby",
            "truffleruby" => runtime.engine == "truffleruby",
            "rbx" => !windows && runtime.engine == "rbx",
            "maglev" => runtime.engine == "maglev",
            "windows" | "mingw" | "x64_mingw" | "mswin" | "mswin64" => windows,
            _ => anyhow::bail!("Unsupported Gemfile platform selector {selector}"),
        };
        let version_matches = version.is_none_or(|version| {
            runtime
                .version
                .to_string()
                .split('.')
                .take(if version.len() == 1 { 1 } else { 2 })
                .collect::<String>()
                == version
        });
        if applies && version_matches {
            return Ok(true);
        }
    }
    Ok(false)
}
