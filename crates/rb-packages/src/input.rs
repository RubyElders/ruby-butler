use crate::constants::{MANIFEST_FILE, SNAPSHOT_FILE, STATE_DIRECTORY};
use anyhow::{Context, Result, ensure};
use rb_gemfile::{Evaluator, Manifest, SnapshotCache};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct ProjectInput {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub gemfile: Option<PathBuf>,
    pub lockfile: PathBuf,
    pub bundler_gemfile: PathBuf,
    pub reused: bool,
}

impl ProjectInput {
    pub fn load(root: impl AsRef<Path>, ruby: impl AsRef<Path>) -> Result<Self> {
        let root = dunce::canonicalize(root)?;
        let toml = root.join("rbproject.toml");
        let kdl = root.join("rbproject.kdl");
        ensure!(
            !(toml.exists() && kdl.exists()),
            "Choose one rbproject format"
        );
        let project = if toml.exists() {
            Some(fs::read_to_string(toml)?)
        } else {
            None
        };
        let gemfile = std::env::var_os("BUNDLE_GEMFILE")
            .map(|name| root.join(name))
            .or_else(|| {
                ["gems.rb", "Gemfile"]
                    .into_iter()
                    .map(|name| root.join(name))
                    .find(|path| path.is_file())
            });
        let has_dependencies = project
            .as_ref()
            .map(|s| s.parse::<toml::Value>())
            .transpose()?
            .as_ref()
            .is_some_and(|v| v.get("dependencies").is_some() || v.get("sources").is_some());
        let (mut manifest, reused, gemfile) = if kdl.exists() {
            (Manifest::from_kdl(&fs::read_to_string(kdl)?)?, false, None)
        } else if has_dependencies {
            (parse_project(project.as_ref().unwrap())?, false, None)
        } else {
            let gemfile = gemfile.context("No rbproject dependencies, gems.rb or Gemfile found")?;
            let state = root.join(STATE_DIRECTORY);
            fs::create_dir_all(&state)?;
            let snapshot = SnapshotCache::new(state.join(SNAPSHOT_FILE)).output_directory(&state);
            let cached = snapshot.evaluate(&Evaluator::new(ruby.as_ref()), &gemfile)?;
            let document = if let Some(settings) = project {
                cached.evaluation.manifest.merge_project(&settings)?
            } else {
                cached.evaluation.manifest.to_toml()?
            };
            rb_gemfile::write_project(
                state.join(MANIFEST_FILE),
                &format!("{}{document}", rb_gemfile::GENERATED_HEADER),
            )?;
            (parse_project(&document)?, cached.reused, Some(gemfile))
        };
        for dependency in manifest.dependencies.values_mut() {
            for path in std::iter::once(&mut dependency.path)
                .chain(
                    dependency
                        .variants
                        .iter_mut()
                        .map(|variant| &mut variant.path),
                )
                .flatten()
            {
                *path = dunce::canonicalize(root.join(&*path))?
                    .to_string_lossy()
                    .into_owned();
            }
        }
        ensure!(
            manifest.bundle.lockfile.is_none(),
            "Custom lockfile paths are not supported by Bundler compatibility preparation"
        );
        let bundler_gemfile = if let Some(path) = &gemfile {
            path.clone()
        } else {
            let generated = root.join(STATE_DIRECTORY).join("Gemfile");
            rb_gemfile::write_project(&generated, &crate::gemfile::render(&manifest)?)?;
            generated
        };
        let lockfile = manifest
            .bundle
            .lockfile
            .as_ref()
            .map(|path| root.join(path))
            .unwrap_or_else(|| {
                if gemfile
                    .as_ref()
                    .is_some_and(|p| p.file_name().is_some_and(|n| n == "gems.rb"))
                {
                    root.join("gems.locked")
                } else if let Some(gemfile) = &gemfile {
                    let mut name = gemfile.as_os_str().to_owned();
                    name.push(".lock");
                    PathBuf::from(name)
                } else {
                    {
                        let mut path = bundler_gemfile.as_os_str().to_owned();
                        path.push(".lock");
                        PathBuf::from(path)
                    }
                }
            });
        Ok(Self {
            root,
            manifest,
            gemfile,
            lockfile,
            bundler_gemfile,
            reused,
        })
    }
}

fn parse_project(input: &str) -> Result<Manifest> {
    let mut value: toml::Value = input.parse()?;
    value
        .as_table_mut()
        .context("Project must be a table")?
        .remove("project");
    value
        .try_into()
        .context("Reading project dependency declarations")
}
