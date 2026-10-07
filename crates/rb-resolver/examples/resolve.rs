use rb_gem_types::{PackageId, Platform};
use rb_resolver::{InMemoryIndex, Package, ResolveRequest, resolve};
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = Package::new(PackageId {
        name: "app".into(),
        version: "1.0".parse()?,
        platform: Platform::parse("ruby"),
    });
    app.dependencies.insert("rack".into(), "~> 3.0".parse()?);
    let rack = Package::new(PackageId {
        name: "rack".into(),
        version: "3.1.0".parse()?,
        platform: Platform::parse("ruby"),
    });
    let request = ResolveRequest::new(
        BTreeMap::from([("app".into(), ">= 0".parse()?)]),
        "3.4".parse()?,
        Platform::parse("x86_64-linux-gnu"),
    );
    for package in resolve(&InMemoryIndex::new([app, rack]), &request)? {
        println!("{}", package.id);
    }
    Ok(())
}
