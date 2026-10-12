use crate::{
    BuildEvent, BuildOptions, BuildReport, BuildStage, build_outputs, build_process, cargo_builder,
    constants::{BUILD_COMPLETE_MARKER, NATIVE_DIRECTORY},
    payload_path,
    ruby_config::RubyConfig,
};
use anyhow::{Context, Result, ensure};
use rb_gem_archive::GemMetadata;
use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    time::Instant,
};

#[derive(Clone, Debug)]
pub struct NativeBuilder {
    pub(crate) options: BuildOptions,
    pub(crate) ruby: RubyConfig,
    compatibility: Option<(PathBuf, PathBuf, String, String)>,
}

impl NativeBuilder {
    pub fn new(options: BuildOptions) -> Result<Self> {
        ensure!(options.jobs > 0, "Build jobs must be positive");
        ensure!(!options.timeout.is_zero(), "Build timeout must be positive");
        std::env::join_paths(&options.load_paths).context("Invalid dependency load path")?;
        let ruby = RubyConfig::read(&options)?;
        Ok(Self {
            options,
            ruby,
            compatibility: None,
        })
    }

    pub fn build(
        &self,
        metadata: &GemMetadata,
        root: impl AsRef<Path>,
        mut events: impl FnMut(BuildEvent),
    ) -> Result<BuildReport> {
        let root = root.as_ref().canonicalize()?;
        let mut configured = self.clone();
        let paths = if metadata.require_paths.is_empty() {
            vec!["lib".to_owned()]
        } else {
            metadata.require_paths.clone()
        };
        let paths = paths
            .iter()
            .map(|path| {
                let path = Path::new(path);
                if path.components().all(|part| part == Component::CurDir) {
                    Ok(root.clone())
                } else {
                    payload_path::normalize(path).map(|path| root.join(path))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        configured.options.load_paths.splice(0..0, paths);
        std::env::join_paths(&configured.options.load_paths).context("Invalid gem load path")?;
        let mut gems = configured.options.dependencies.clone();
        gems.retain(|gem| {
            !(gem.name == metadata.id.name
                && gem.version == metadata.id.version.to_string()
                && gem.platform == metadata.id.platform.to_string())
        });
        gems.push(rb_microgems::InstalledGem {
            name: metadata.id.name.clone(),
            version: metadata.id.version.to_string(),
            platform: metadata.id.platform.to_string(),
            root: root.clone(),
            require_paths: metadata.require_paths.clone(),
            metadata: metadata.metadata.clone(),
            executables: metadata.executables.clone(),
            bindir: metadata.bindir.clone(),
        });
        let compatibility = rb_microgems::Environment::new(&gems, &configured.options.arguments)?;
        configured.compatibility = Some((
            compatibility.load_path().to_path_buf(),
            compatibility.manifest_path().to_path_buf(),
            serde_json::to_string(&[&metadata.id.name, &metadata.id.version.to_string()])?,
            serde_json::to_string(&configured.options.load_paths)?,
        ));
        configured.build_declared(metadata, &root, &mut events)
    }

    fn build_declared(
        &self,
        metadata: &GemMetadata,
        root: &Path,
        events: &mut impl FnMut(BuildEvent),
    ) -> Result<BuildReport> {
        ensure!(
            metadata.ruby.matches(&self.ruby.version.parse()?),
            "Gem requires Ruby {}, selected Ruby is {}",
            metadata.ruby,
            self.ruby.version
        );
        let started = Instant::now();
        let mut report = BuildReport {
            ruby_version: self.ruby.version.clone(),
            architecture: self.ruby.get("arch").into(),
            extensions: Vec::new(),
            artifacts: Vec::new(),
            extension_directory: None,
            elapsed: Default::default(),
        };
        if metadata.extensions.is_empty() {
            return Ok(report);
        }
        let native_relative = PathBuf::from(NATIVE_DIRECTORY)
            .join(self.ruby.get("arch"))
            .join(self.ruby.get("ruby_version"));
        let native_relative = payload_path::relative(&native_relative)?;
        report.extension_directory = Some(native_relative.clone());
        let destination = root.join(&native_relative);
        ensure!(
            !destination.exists(),
            "Native build destination already exists"
        );
        fs::create_dir_all(&destination)?;
        let lib = self
            .options
            .load_paths
            .first()
            .context("Missing gem load path")?;
        fs::create_dir_all(lib)?;
        ensure!(
            lib.canonicalize()?.starts_with(root),
            "Gem load path escapes installation"
        );
        for extension in &metadata.extensions {
            let relative = payload_path::normalize(Path::new(extension))?;
            let path = root
                .join(&relative)
                .canonicalize()
                .with_context(|| format!("Missing extension {extension}"))?;
            ensure!(
                path.starts_with(root) && path.is_file(),
                "Extension is outside installation or is not a file"
            );
            let directory = path.parent().context("Missing extension directory")?;
            let output = tempfile::Builder::new()
                .prefix(".rb-build-")
                .tempdir_in(directory)?;
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .context("Non-UTF8 builder name")?;
            match name {
                "extconf.rb" | "configure" => {
                    crate::make_builder::build(self, &path, &relative, output.path(), events)?
                }
                "Rakefile" | "rakefile" | "mkrf_conf.rb" => {
                    crate::rake_builder::build(self, &path, &relative, output.path(), events)?
                }
                "CMakeLists.txt" => {
                    crate::cmake_builder::build(self, &path, &relative, output.path(), events)?
                }
                "Cargo.toml" => {
                    cargo_builder::build(self, &path, &relative, output.path(), events)?
                }
                _ => anyhow::bail!("Unsupported extension builder: {extension}"),
            }
            let files = build_outputs::copy(output.path(), &destination)
                .with_context(|| format!("Collecting build outputs for {extension}"))?;
            build_outputs::copy(output.path(), lib)?;
            report
                .artifacts
                .extend(files.into_iter().map(|file| native_relative.join(file)));
            report.extensions.push(relative);
        }
        for path in self
            .options
            .load_paths
            .iter()
            .take(metadata.require_paths.len().max(1))
        {
            build_outputs::native_files(root, path, self.ruby.get("DLEXT"), &mut report.artifacts)?;
        }
        for extension in &metadata.extensions {
            let path = root.join(payload_path::normalize(Path::new(extension))?);
            let directory = path.parent().context("Missing extension directory")?;
            build_outputs::native_files(
                root,
                directory,
                self.ruby.get("DLEXT"),
                &mut report.artifacts,
            )?;
        }
        report.artifacts.sort();
        report.artifacts.dedup();
        fs::write(destination.join(BUILD_COMPLETE_MARKER), b"")?;
        report.elapsed = started.elapsed();
        Ok(report)
    }

    pub(crate) fn command(
        &self,
        program: impl AsRef<std::ffi::OsStr>,
        directory: &Path,
    ) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(directory)
            .env_remove("RUBYLIB")
            .envs(&self.options.environment);
        command
            .env_remove("RUBYGEMS_GEMDEPS")
            .env_remove("RUBY_BOX")
            .env_remove("DESTDIR");
        command
            .env("RUBY", &self.ruby.executable)
            .env("RUBYOPT", "--disable=gems");
        let mut make_flags = self
            .options
            .environment
            .get(std::ffi::OsStr::new("MAKEFLAGS"))
            .cloned()
            .or_else(|| std::env::var_os("MAKEFLAGS"))
            .unwrap_or_default();
        if !make_flags.is_empty() {
            make_flags.push(" ");
        }
        make_flags.push(format!("-j{}", self.options.jobs));
        command
            .env("MAKEFLAGS", make_flags)
            .env("NJOBS", self.options.jobs.to_string())
            .env("CARGO_BUILD_JOBS", self.options.jobs.to_string())
            .env("CMAKE_BUILD_PARALLEL_LEVEL", self.options.jobs.to_string())
            .env("GRPC_RUBY_BUILD_PROCS", self.options.jobs.to_string());
        for (key, value) in &self.ruby.config {
            command.env(format!("RBCONFIG_{key}"), value);
        }
        let mut paths = self.options.load_paths.clone();
        if let Some((directory, manifest, current, load_paths)) = &self.compatibility {
            paths.clear();
            paths.push(directory.clone());
            command
                .env("RB_MICROGEMS_MANIFEST", manifest)
                .env("RB_MICROGEMS_CURRENT", current)
                .env("RB_MICROGEMS_LOAD_PATHS", load_paths)
                .env("RUBYOPT", "--disable=gems -rmicrogems/compatibility");
        }
        if let Ok(paths) = std::env::join_paths(&paths) {
            command.env("RUBYLIB", paths);
        }
        command
    }

    pub(crate) fn ruby_command(&self, directory: &Path) -> Command {
        let mut command = self.command(&self.ruby.executable, directory);
        command.arg("--disable=gems");
        command
    }

    pub(crate) fn run(
        &self,
        command: Command,
        extension: &Path,
        stage: BuildStage,
        events: &mut impl FnMut(BuildEvent),
    ) -> Result<build_process::ProcessOutput> {
        let directory = command.get_current_dir().map(Path::to_path_buf);
        build_process::run(command, extension, stage, self.options.timeout, events)
            .map_err(
                |error| match crate::build_diagnostics::mkmf(directory.as_deref()) {
                    Some(log) => error.context(format!("mkmf.log:\n{log}")),
                    None => error,
                },
            )
            .with_context(|| format!("Building {} ({stage:?})", extension.display()))
    }
}
