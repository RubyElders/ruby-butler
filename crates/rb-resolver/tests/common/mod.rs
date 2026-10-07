use rb_gem_types::{PackageId, Platform};
use rb_resolver::{Package, ResolveRequest};
use std::collections::BTreeMap;

pub fn package(name: &str, version: &str, dependencies: &[(&str, &str)]) -> Package {
    let mut package = Package::new(PackageId {
        name: name.into(),
        version: version.parse().unwrap(),
        platform: Platform::parse("ruby"),
    });
    package.dependencies = dependencies
        .iter()
        .map(|(name, requirement)| ((*name).into(), requirement.parse().unwrap()))
        .collect();
    package
}

pub fn request(roots: &[(&str, &str)]) -> ResolveRequest {
    ResolveRequest::new(
        roots
            .iter()
            .map(|(name, requirement)| ((*name).into(), requirement.parse().unwrap()))
            .collect(),
        "3.4".parse().unwrap(),
        Platform::parse("x86_64-linux-gnu"),
    )
}

pub fn names(packages: Vec<Package>) -> BTreeMap<String, String> {
    packages
        .into_iter()
        .map(|package| (package.id.name, package.id.version.to_string()))
        .collect()
}
