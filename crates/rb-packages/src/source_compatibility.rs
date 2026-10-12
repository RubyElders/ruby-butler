use crate::{InstalledBundle, PackageIndex, PackagePlan, PackageSource, ProjectInput};
use anyhow::{Context, Result};
use std::path::PathBuf;

pub(crate) fn write(
    input: &ProjectInput,
    plan: &PackagePlan,
    bundle: &InstalledBundle,
    index: &dyn PackageIndex,
) -> Result<PathBuf> {
    if !plan
        .packages
        .iter()
        .any(|p| matches!(p.record.source, PackageSource::Path { .. }))
    {
        return Ok(input.bundler_gemfile.clone());
    }
    let mut manifest = input.manifest.clone();
    let mut lock = plan.lockfile.clone();
    let mut paths = std::collections::BTreeMap::new();
    for package in &plan.packages {
        if matches!(package.record.source, PackageSource::Path { .. }) {
            let runtime = index
                .compatibility_path(&package.record, &bundle.gem_home)?
                .context("Path source has no compatibility snapshot")?;
            paths.insert(
                package.record.source.clone(),
                runtime.to_string_lossy().into_owned(),
            );
        }
    }
    fn rewrite(
        dependency: &mut rb_gemfile::Dependency,
        paths: &std::collections::BTreeMap<PackageSource, String>,
    ) -> Result<()> {
        if let Some(path) = &mut dependency.path {
            let source = PackageSource::Path {
                directory: PathBuf::from(&*path),
                glob: crate::package_source::normalized_glob(dependency.glob.as_deref()),
            };
            *path = paths.get(&source).context("Missing path snapshot")?.clone();
        }
        for variant in &mut dependency.variants {
            rewrite(variant, paths)?;
        }
        Ok(())
    }
    for dependency in manifest.dependencies.values_mut() {
        rewrite(dependency, &paths)?;
    }
    for source in &mut lock.sources {
        if source.kind == rb_lockfile::SourceKind::Path {
            let identity = PackageSource::from_lock(
                source,
                source
                    .remotes
                    .first()
                    .context("Path lock has no location")?,
                Some(&input.root),
            )?;
            source.remotes = vec![
                paths
                    .get(&identity)
                    .context("Missing locked path snapshot")?
                    .clone(),
            ];
        }
    }
    let gemfile = input
        .root
        .join(crate::constants::STATE_DIRECTORY)
        .join(crate::constants::RUNTIME_DIRECTORY)
        .join(crate::constants::GEMFILE_FILE);
    rb_gemfile::write_project(&gemfile, &crate::gemfile::render(&manifest)?)?;
    rb_gemfile::write_project(gemfile.with_file_name("Gemfile.lock"), &lock.render()?)?;
    Ok(gemfile)
}
