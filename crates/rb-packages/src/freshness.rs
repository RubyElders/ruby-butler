use crate::{InstallOptions, PackagePlan, RubyEnvironment, receipt::Receipt};
use anyhow::Result;
use rb_gemfile::Manifest;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub fn fingerprint(
    plan: &PackagePlan,
    manifest: &Manifest,
    runtime: &RubyEnvironment,
    options: &InstallOptions,
) -> Result<String> {
    let mut packages = BTreeMap::new();
    for package in &plan.packages {
        let record = &package.record;
        packages.insert(
            record.package.id.clone(),
            serde_json::json!({
                "receipt": Receipt::new(record, runtime, &options.build, &[])?,
                "install": package.install,
                "dependencies": record.package.dependencies,
                "ruby": record.package.ruby,
                "rubygems": record.rubygems,
            }),
        );
    }
    let activation: BTreeMap<_, _> = manifest
        .dependencies
        .iter()
        .map(|(name, dependency)| {
            let mut declarations: Vec<_> = std::iter::once(dependency)
                .chain(&dependency.variants)
                .filter(|value| value.enabled)
                .map(|value| {
                    serde_json::to_string(&serde_json::json!({
                        "groups": value.groups.iter().collect::<BTreeSet<_>>(),
                        "platforms": value.platforms.iter().collect::<BTreeSet<_>>(),
                        "require": value.require,
                    }))
                })
                .collect::<std::result::Result<_, _>>()?;
            declarations.sort();
            declarations.dedup();
            Ok((name, declarations))
        })
        .collect::<Result<_>>()?;
    let bytes = serde_json::to_vec(&serde_json::json!({
        "version": 1,
        "packages": packages.into_iter().map(|(id, value)| (id.to_string(), value)).collect::<BTreeMap<_,_>>(),
        "activation": activation,
        "optional_groups": manifest.bundle.optional_groups.iter().collect::<BTreeSet<_>>(),
        "with": plan.with,
        "without": plan.without,
        "runtime": (&runtime.engine, &runtime.engine_version, runtime.patchlevel, &runtime.rubygems, &runtime.bundler),
    }))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
