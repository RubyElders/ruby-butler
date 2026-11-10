use crate::{PrepareOptions, ProjectInput, RubyEnvironment};
use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

#[derive(Serialize)]
pub struct CacheInputs {
    pub complete: bool,
    pub lockfile: PathBuf,
    pub compatibility: String,
    pub fingerprint: Option<String>,
}

enum InputKind {
    Project,
    Gemfile,
}

pub fn inspect_cache(options: &PrepareOptions) -> Result<CacheInputs> {
    inspect(options, InputKind::Project)
}

pub fn inspect_gemfile_cache(options: &PrepareOptions) -> Result<CacheInputs> {
    inspect(options, InputKind::Gemfile)
}

fn inspect(options: &PrepareOptions, kind: InputKind) -> Result<CacheInputs> {
    let environment = options.install.build.inherited_environment();
    let directory = options.install.build.directory();
    let runtime = RubyEnvironment::inspect_with_inputs(&options.ruby, directory, environment)?;
    let input = if matches!(kind, InputKind::Gemfile) {
        ProjectInput::load_gemfile_with_inputs(
            &options.project,
            &options.ruby,
            options.gemfile.as_deref().context("No Gemfile selected")?,
            directory,
            environment,
        )?
    } else {
        ProjectInput::load_with_inputs(
            &options.project,
            &options.ruby,
            options.project_file.as_deref(),
            options.gemfile.as_deref(),
            directory,
            environment,
        )?
    };
    let compatibility = digest(&serde_json::json!({
        "ruby": runtime.executable, "version": runtime.version.to_string(),
        "engine": runtime.engine, "engine_version": runtime.engine_version.to_string(),
        "abi": runtime.abi, "platform": runtime.platform,
        "rubygems": runtime.rubygems.to_string(), "bundler": runtime.bundler.as_ref().map(ToString::to_string),
        "configuration": configuration(&input.root, environment)?,
        "with": options.plan.with, "without": options.plan.without,
        "optional_groups": input.manifest.bundle.optional_groups,
        "build": crate::receipt::configuration_hash(&options.install.build, &runtime, &[])?,
    }))?;
    let mut result = CacheInputs {
        complete: false,
        lockfile: input.lockfile.clone(),
        compatibility,
        fingerprint: None,
    };
    if input
        .manifest
        .bundle
        .source
        .iter()
        .chain(input.manifest.dependencies.values().flat_map(|dependency| {
            std::iter::once(dependency)
                .chain(&dependency.variants)
                .flat_map(|dependency| dependency.source.iter().chain(&dependency.git))
        }))
        .any(|source| credential_url(source))
    {
        return Ok(result);
    }
    let Some(mut names) = input.evaluation_environment.clone() else {
        return Ok(result);
    };
    if names.iter().any(|name| !safe_environment(name)) {
        return Ok(result);
    }
    names.extend(
        crate::receipt::BUILD_ENVIRONMENT
            .iter()
            .map(|name| name.to_string()),
    );
    let mut paths: BTreeSet<_> = input.evaluation_inputs.iter().cloned().collect();
    let mut sources = BTreeSet::new();
    for dependency in input
        .manifest
        .dependencies
        .values()
        .flat_map(|dependency| std::iter::once(dependency).chain(&dependency.variants))
    {
        if dependency.enabled
            && let Some(path) = &dependency.path
        {
            sources.insert((PathBuf::from(path), dependency.glob.clone()));
        }
    }
    let mut cache =
        crate::input::snapshot_cache(&input.root, "cache-inputs.json", options.gemfile.as_deref());
    for (source, glob) in sources {
        for specification in crate::source_specs::read_with_environment(
            &options.ruby,
            &source,
            glob.as_deref(),
            environment,
        )? {
            let Some(observed) = specification.environment.as_ref() else {
                return Ok(result);
            };
            if observed.iter().any(|name| !safe_environment(name)) {
                return Ok(result);
            }
            names.extend(observed.iter().cloned());
            paths.extend(specification.tracked_paths(&source)?);
        }
        for excluded in [".git", ".trees", ".rb", ".bundle"] {
            cache = cache.output_directory(source.join(excluded));
        }
    }
    let anchor = options
        .project_file
        .clone()
        .or(input.gemfile.clone())
        .or_else(|| {
            ["gem.kdl", "gem.toml", "rbproject.kdl", "rbproject.toml"]
                .iter()
                .map(|name| input.root.join(name))
                .find(|path| path.is_file())
        })
        .context("No dependency input")?;
    for name in [
        "gem.toml",
        "gem.kdl",
        "rbproject.toml",
        "rbproject.kdl",
        ".ruby-version",
    ] {
        cache = cache.input(input.root.join(name));
    }
    for path in paths {
        if path.ends_with(".bundle/config")
            || path.ends_with(".rb/config")
            || path.ends_with("auth.toml")
            || path.ends_with("credentials")
        {
            return Ok(result);
        }
        cache = cache.input(path);
    }
    let lock = match fs::read(&input.lockfile) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(result),
        Err(error) => return Err(error.into()),
    };
    if String::from_utf8_lossy(&lock)
        .lines()
        .filter_map(|line| line.trim().strip_prefix("remote: "))
        .any(credential_url)
    {
        return Ok(result);
    }
    result.fingerprint = Some(digest(&(
        cache
            .environment_inputs(names)
            .fingerprint(&options.evaluator(), &anchor)?,
        lock,
    ))?);
    result.complete = true;
    Ok(result)
}

fn credential_url(source: &str) -> bool {
    url::Url::parse(source).is_ok_and(|url| {
        url.password().is_some()
            || (matches!(url.scheme(), "https" | "http") && !url.username().is_empty())
    })
}

fn safe_environment(name: &str) -> bool {
    matches!(
        name,
        "RAILS_ENV"
            | "RACK_ENV"
            | "RUBY_VERSION"
            | "RUBY_ENGINE"
            | "CI"
            | "BUNDLE_WITH"
            | "BUNDLE_WITHOUT"
    ) || crate::receipt::BUILD_ENVIRONMENT.contains(&name)
}

fn configuration(
    project: &std::path::Path,
    environment: &BTreeMap<std::ffi::OsString, std::ffi::OsString>,
) -> Result<BTreeMap<String, String>> {
    let allowed = |name: &str| {
        safe_environment(name)
            || name.starts_with("BUNDLE_BUILD__")
            || matches!(
                name,
                "BUNDLE_FORCE_RUBY_PLATFORM" | "BUNDLE_DISABLE_SHARED_GEMS"
            )
    };
    let mut result = BTreeMap::new();
    let home = environment
        .get(std::ffi::OsStr::new("HOME"))
        .map(PathBuf::from);
    let global = environment
        .get(std::ffi::OsStr::new("BUNDLE_USER_CONFIG"))
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".bundle/config")));
    for path in global
        .into_iter()
        .chain([project.join(".bundle/config"), project.join(".rb/config")])
    {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let settings: Option<BTreeMap<String, serde_yaml_ng::Value>> =
            serde_yaml_ng::from_str(&text)?;
        for (name, value) in settings
            .unwrap_or_default()
            .into_iter()
            .filter(|(name, _)| allowed(name))
        {
            result.insert(name, serde_yaml_ng::to_string(&value)?);
        }
    }
    for (name, value) in environment {
        if let Some(name) = name.to_str().filter(|name| allowed(name)) {
            result.insert(name.to_string(), value.to_string_lossy().into_owned());
        }
    }
    Ok(result)
}

fn digest(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}
