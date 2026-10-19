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
    pub(crate) evaluation_inputs: Vec<PathBuf>,
}

impl ProjectInput {
    pub fn load(root: impl AsRef<Path>, ruby: impl AsRef<Path>) -> Result<Self> {
        Self::load_with_project_file(root, ruby, None)
    }

    pub fn load_with_project_file(
        root: impl AsRef<Path>,
        ruby: impl AsRef<Path>,
        project_file: Option<&Path>,
    ) -> Result<Self> {
        let root = dunce::canonicalize(root)?;
        let selected = project_file
            .map(|file| dunce::canonicalize(root.join(file)))
            .transpose()?;
        let is_kdl = selected
            .as_ref()
            .is_some_and(|path| path.extension().is_some_and(|ext| ext == "kdl"));
        let toml = selected
            .clone()
            .filter(|_| !is_kdl)
            .unwrap_or_else(|| root.join("rbproject.toml"));
        let kdl = selected
            .filter(|_| is_kdl)
            .unwrap_or_else(|| root.join("rbproject.kdl"));
        ensure!(
            project_file.is_some() || !(toml.exists() && kdl.exists()),
            "Choose one rbproject format"
        );
        let project = if toml.exists() && !is_kdl {
            Some(fs::read_to_string(toml)?)
        } else {
            None
        };
        let gemfile = std::env::var_os("BUNDLE_GEMFILE")
            .filter(|name| !name.is_empty())
            .map(|name| std::env::current_dir().map(|cwd| cwd.join(name)))
            .transpose()?
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
        let kdl_manifest = if is_kdl || (project_file.is_none() && kdl.exists()) {
            parse_kdl_project(&fs::read_to_string(kdl)?)?
        } else {
            None
        };
        let mut evaluation_inputs = Vec::new();
        let (mut manifest, reused, gemfile) = if let Some(manifest) = kdl_manifest {
            (manifest, false, None)
        } else if has_dependencies {
            (parse_project(project.as_ref().unwrap())?, false, None)
        } else {
            let gemfile = gemfile.context("No rbproject dependencies, gems.rb or Gemfile found")?;
            let state = root.join(STATE_DIRECTORY);
            fs::create_dir_all(&state)?;
            let snapshot = snapshot_cache(&root, SNAPSHOT_FILE);
            let cached = snapshot.evaluate(&Evaluator::new(ruby.as_ref()), &gemfile)?;
            evaluation_inputs = cached.evaluation.inputs.clone();
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
            evaluation_inputs,
        })
    }
}

fn parse_project(input: &str) -> Result<Manifest> {
    let mut value: toml::Value = input.parse()?;
    let table = value.as_table_mut().context("Project must be a table")?;
    table.remove("project");
    table.remove("scripts");
    value
        .try_into()
        .context("Reading project dependency declarations")
}

fn parse_kdl_project(input: &str) -> Result<Option<Manifest>> {
    let mut document: kdl::KdlDocument = input.parse()?;
    document
        .nodes_mut()
        .retain(|node| !matches!(node.name().value(), "project" | "scripts"));
    if document.nodes().is_empty() {
        return Ok(None);
    }
    Ok(Some(Manifest::from_kdl(&document.to_string())?))
}

pub(crate) fn snapshot_cache(root: &Path, filename: &str) -> SnapshotCache {
    let cache = SnapshotCache::new(root.join(STATE_DIRECTORY).join(filename))
        .output_directory(root.join(STATE_DIRECTORY))
        .output(root.join("Gemfile.lock"))
        .output(root.join("gems.locked"));
    let cache = ["log", "tmp"].into_iter().fold(cache, |cache, directory| {
        let path = root.join(directory);
        if path.is_file() {
            cache
        } else {
            cache.output_directory(path)
        }
    });
    if let Some(file) = std::env::var_os("BUNDLE_GEMFILE").filter(|file| !file.is_empty()) {
        let mut filename = std::env::current_dir()
            .unwrap_or_else(|_| root.to_path_buf())
            .join(file)
            .into_os_string();
        filename.push(".lock");
        cache.output(PathBuf::from(filename))
    } else {
        cache
    }
}
