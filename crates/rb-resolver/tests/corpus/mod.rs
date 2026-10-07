use rb_gem_types::{PackageId, Platform, Requirement};
use rb_resolver::{Package, ResolveRequest};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::PathBuf};

#[derive(Deserialize)]
pub struct Case {
    pub name: String,
    pub repository: String,
}

#[derive(Deserialize)]
pub struct Identity {
    pub name: String,
    pub version: String,
    pub platform: String,
}

impl Identity {
    pub fn id(&self) -> PackageId {
        PackageId {
            name: self.name.clone(),
            version: self.version.parse().unwrap(),
            platform: Platform::parse(&self.platform),
        }
    }
}

#[derive(Deserialize)]
pub struct Expected {
    pub ruby: String,
    pub bundler: String,
    pub target: String,
    pub packages: Vec<Identity>,
}

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corpus")
}

pub fn cases() -> Vec<Case> {
    serde_json::from_str(&fs::read_to_string(root().join("cases.json")).unwrap()).unwrap()
}

pub fn expected(case: &Case) -> Expected {
    serde_json::from_str(
        &fs::read_to_string(root().join(&case.name).join("expected.json")).unwrap(),
    )
    .unwrap()
}

pub fn request(case: &Case, expected: &Expected) -> ResolveRequest {
    let manifest: rb_gemfile::Manifest = toml::from_str(
        &fs::read_to_string(root().join(&case.name).join("rbproject.toml")).unwrap(),
    )
    .unwrap();
    let default_source = manifest.bundle.source.as_deref();
    let roots = manifest
        .dependencies
        .into_iter()
        .filter(|(_, dependency)| {
            dependency.enabled
                && dependency.groups.iter().any(|group| group == "default")
                && dependency.platforms.is_empty()
        })
        .map(|(name, dependency)| {
            assert!(dependency.git.is_none() && dependency.path.is_none());
            assert_eq!(
                dependency
                    .source
                    .as_deref()
                    .or(default_source)
                    .map(|source| source.trim_end_matches('/')),
                Some("https://rubygems.org")
            );
            let requirement = if dependency.version.is_empty() {
                ">= 0".into()
            } else {
                dependency.version.join(", ")
            };
            (name, requirement.parse().unwrap())
        })
        .collect();
    let mut request = ResolveRequest::new(
        roots,
        expected.ruby.parse().unwrap(),
        Platform::parse(&expected.target),
    );
    request.roots.insert(
        "bundler".into(),
        format!("= {}", expected.bundler).parse().unwrap(),
    );
    request
}

pub fn packages() -> Vec<Package> {
    let mut packages = Vec::new();
    for file in fs::read_dir(root().join("index")).unwrap() {
        let path = file.unwrap().path();
        if path.extension().and_then(|value| value.to_str()) != Some("info") {
            continue;
        }
        let name = path.file_stem().unwrap().to_str().unwrap();
        for record in rb_compact_index::parse(name, &fs::read_to_string(&path).unwrap()).unwrap() {
            let (version, platform) = record
                .version
                .split_once('-')
                .unwrap_or((&record.version, "ruby"));
            packages.push(Package {
                id: PackageId {
                    name: name.into(),
                    version: version.parse().unwrap(),
                    platform: Platform::parse(platform),
                },
                ruby: record
                    .metadata
                    .get("ruby")
                    .map_or(">= 0", String::as_str)
                    .parse()
                    .unwrap(),
                dependencies: record
                    .dependencies
                    .into_iter()
                    .map(|(name, requirement)| (name, requirement.parse().unwrap()))
                    .collect(),
            });
        }
    }
    packages
}

pub fn validate(request: &ResolveRequest, packages: &[Package]) {
    let selected: BTreeMap<_, _> = packages
        .iter()
        .map(|package| (&package.id.name, package))
        .collect();
    assert_eq!(selected.len(), packages.len());
    for (name, requirement) in &request.roots {
        assert!(
            requirement.matches(&selected[name].id.version),
            "root {name}"
        );
    }
    for package in packages {
        assert!(
            package.ruby.matches(&request.ruby),
            "Ruby constraint for {}",
            package.id
        );
        assert!(
            package.id.platform.matches(&request.target),
            "platform for {}",
            package.id
        );
        for (name, requirement) in &package.dependencies {
            assert!(
                requirement.matches(&selected[name].id.version),
                "{} depends on {name} ({requirement})",
                package.id
            );
        }
    }
    let mut reachable = std::collections::BTreeSet::new();
    let mut pending: Vec<_> = request.roots.keys().collect();
    while let Some(name) = pending.pop() {
        if reachable.insert(name) {
            pending.extend(selected[name].dependencies.keys());
        }
    }
    assert_eq!(reachable, selected.keys().copied().collect());
}

pub fn requirement(value: &str) -> Requirement {
    value.parse().unwrap()
}
