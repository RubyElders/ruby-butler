use super::{PackageBackend, PackageEvents, PackageStatus, PreparedEnvironment};
use anyhow::{Context, Result, ensure};
use rb_core::butler::ButlerRuntime;
use std::path::{Path, PathBuf};

pub(super) struct NativeBackend {
    project_file: Option<PathBuf>,
    disabled: bool,
}

impl NativeBackend {
    pub(super) fn new(project_file: Option<PathBuf>, disabled: bool) -> Self {
        Self {
            project_file,
            disabled,
        }
    }

    fn project(&self, directory: &Path) -> Result<Option<(PathBuf, Option<PathBuf>)>> {
        if let Some(file) = &self.project_file {
            let file = directory
                .join(file)
                .canonicalize()
                .context("Opening selected project file")?;
            return Ok(Some((
                file.parent()
                    .context("Project file has no directory")?
                    .into(),
                Some(file),
            )));
        }
        if let Some(file) = std::env::var_os("BUNDLE_GEMFILE").filter(|file| !file.is_empty()) {
            let file = directory
                .join(file)
                .canonicalize()
                .context("Opening BUNDLE_GEMFILE")?;
            return Ok(Some((
                file.parent().context("Gemfile has no directory")?.into(),
                None,
            )));
        }
        for root in directory.ancestors() {
            for name in ["gem.kdl", "gem.toml", "rbproject.kdl", "rbproject.toml"] {
                let file = root.join(name);
                if file.is_file() {
                    return Ok(Some((root.into(), Some(file))));
                }
            }
            if ["gems.rb", "Gemfile"]
                .iter()
                .any(|name| root.join(name).is_file())
            {
                return Ok(Some((root.into(), None)));
            }
        }
        Ok(None)
    }

    fn options(&self, runtime: &ButlerRuntime) -> Result<Option<rb_packages::PrepareOptions>> {
        let Some((root, project_file)) = self.project(runtime.current_dir())? else {
            return Ok(None);
        };
        let ruby = runtime.selected_ruby()?.ruby_executable_path();
        let mut options = rb_packages::PrepareOptions::new(&root, ruby, root.join(".rb/cache"));
        options.project_file = project_file;
        options.record_state = true;
        for (key, groups) in [
            ("BUNDLE_WITH", &mut options.plan.with),
            ("BUNDLE_WITHOUT", &mut options.plan.without),
        ] {
            if let Ok(value) = std::env::var(key) {
                groups.extend(
                    value
                        .split([':', ' '])
                        .filter(|group| !group.is_empty())
                        .map(String::from),
                );
            }
        }
        if ["BUNDLE_FROZEN", "BUNDLE_DEPLOYMENT"]
            .iter()
            .any(|key| std::env::var(key).is_ok_and(|value| value == "true" || value == "1"))
        {
            options.plan.mode = rb_packages::LockMode::Locked;
        }
        Ok(Some(options))
    }

    fn activate(runtime: &ButlerRuntime, installed: rb_packages::InstalledBundle) -> ButlerRuntime {
        runtime.clone().with_package_environment(
            installed.environment().into_iter().collect(),
            installed.gem_home.join("bin"),
        )
    }
}

impl PackageBackend for NativeBackend {
    fn check(&self, runtime: &ButlerRuntime) -> Result<PackageStatus> {
        if self.disabled {
            return Ok(PackageStatus::Ready(Box::new(runtime.clone())));
        }
        let Some(options) = self.options(runtime)? else {
            return Ok(PackageStatus::Ready(Box::new(runtime.clone())));
        };
        Ok(match rb_packages::prepared(&options)? {
            Some(installed) => PackageStatus::Ready(Box::new(Self::activate(runtime, installed))),
            None => PackageStatus::NeedsPreparation,
        })
    }

    fn prepare(
        &self,
        runtime: &ButlerRuntime,
        events: PackageEvents<'_>,
    ) -> Result<PreparedEnvironment> {
        ensure!(
            !self.disabled,
            "Package preparation is disabled by --no-bundler"
        );
        let options = self
            .options(runtime)?
            .context("No project file, gems.rb or Gemfile found")?;
        let prepared = rb_packages::prepare(options, events.tasks)?;
        let changed = prepared.plan.lock_changed || prepared.installed.installed > 0;
        Ok(PreparedEnvironment {
            runtime: Self::activate(runtime, prepared.installed),
            changed,
        })
    }
}
