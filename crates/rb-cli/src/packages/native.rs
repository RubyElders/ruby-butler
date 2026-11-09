use super::{
    BackendStatus, Driver, PackageDetails, PackageEnvironment, PackageEvents, PreparedInstallation,
};
use anyhow::{Context, Result, ensure};
use rb_core::butler::ButlerRuntime;
use std::path::PathBuf;

pub(super) struct NativeBackend {
    location: Option<crate::invocation::DependencyLocation>,
    process: rb_core::process_context::ProcessContext,
    disabled: bool,
    options: std::cell::OnceCell<rb_packages::PrepareOptions>,
}

impl NativeBackend {
    pub(super) fn new(context: &crate::runtime_helpers::CommandContext) -> Self {
        Self {
            location: context.projects().dependency.clone(),
            process: context.process.clone(),
            disabled: *context.config().no_bundler.get(),
            options: std::cell::OnceCell::new(),
        }
    }

    fn project(&self) -> Result<Option<(PathBuf, Option<PathBuf>)>> {
        let Some(location) = &self.location else {
            return Ok(None);
        };
        if location.explicit {
            let file = location
                .project_file
                .as_ref()
                .or(location.gemfile.as_ref())
                .context("Missing explicit project input")?
                .canonicalize()
                .context("Opening selected project input")?;
            let root = file
                .parent()
                .context("Project input has no directory")?
                .to_owned();
            return Ok(Some((root, location.project_file.as_ref().map(|_| file))));
        }
        Ok(Some((location.root.clone(), location.project_file.clone())))
    }

    fn options(&self, runtime: &ButlerRuntime) -> Result<Option<rb_packages::PrepareOptions>> {
        if self.location.is_none() {
            return Ok(None);
        }
        let ruby = runtime.selected_ruby()?.ruby_executable_path();
        if let Some(options) = self.options.get().filter(|options| options.ruby == ruby) {
            return Ok(Some(options.clone()));
        }
        let Some((root, project_file)) = self.project()? else {
            return Ok(None);
        };
        let mut options = rb_packages::PrepareOptions::with_inputs(
            &root,
            ruby,
            crate::cache::directory(&self.process)?,
            self.process.directory().to_owned(),
            self.process.environment().clone(),
        );
        options.project_file = project_file;
        options.gemfile = self
            .location
            .as_ref()
            .and_then(|location| location.gemfile.clone());
        options.record_state = true;
        for (key, groups) in [
            ("BUNDLE_WITH", &mut options.plan.with),
            ("BUNDLE_WITHOUT", &mut options.plan.without),
        ] {
            if let Some(value) = self.process.var(key).and_then(|value| value.to_str()) {
                groups.extend(
                    value
                        .split([':', ' '])
                        .filter(|group| !group.is_empty())
                        .map(String::from),
                );
            }
        }
        if ["BUNDLE_FROZEN", "BUNDLE_DEPLOYMENT"].iter().any(|key| {
            self.process
                .var(key)
                .and_then(|value| value.to_str())
                .is_some_and(|value| value == "true" || value == "1")
        }) {
            options.plan.mode = rb_packages::LockMode::Locked;
        }
        let _ = self.options.set(options.clone());
        Ok(Some(options))
    }

    fn activate(runtime: &ButlerRuntime, installed: rb_packages::InstalledBundle) -> ButlerRuntime {
        runtime
            .clone()
            .with_package_environment_os(installed.environment_os(), installed.gem_home.join("bin"))
    }
}

impl Driver for NativeBackend {
    fn inspect(&self, runtime: &ButlerRuntime) -> Result<PackageEnvironment> {
        let mut environment = PackageEnvironment {
            runtime: runtime.clone(),
            name: "Native Package",
            details: None,
        };
        if self.disabled {
            return Ok(environment);
        }
        let Some((root, _)) = self.project()? else {
            return Ok(environment);
        };
        let ruby = runtime.selected_ruby()?;
        let engine = match ruby.kind {
            rb_core::ruby::RubyType::CRuby => "ruby",
        };
        let layout = rb_packages::BundleLayout::new(&root, engine, &ruby.version.to_string());
        environment.runtime = runtime.clone().with_package_environment_os(
            std::collections::BTreeMap::from([
                ("GEM_HOME".into(), layout.gem_home.clone().into_os_string()),
                ("GEM_PATH".into(), layout.gem_home.clone().into_os_string()),
                ("BUNDLE_PATH".into(), layout.bundle.clone().into_os_string()),
            ]),
            layout.gem_home.join("bin"),
        );
        environment.details = Some(PackageDetails {
            root,
            gemfile: None,
            configuration: None,
            configured: layout.gem_home.is_dir(),
            installation: layout.gem_home,
            required_ruby: None,
        });
        Ok(environment)
    }

    fn check(&self, runtime: &ButlerRuntime) -> Result<BackendStatus> {
        if self.disabled {
            return Ok(BackendStatus::Ready(Box::new(runtime.clone())));
        }
        let Some(options) = self.options(runtime)? else {
            return Ok(BackendStatus::Ready(Box::new(runtime.clone())));
        };
        Ok(match rb_packages::prepared(&options)? {
            Some(installed) => BackendStatus::Ready(Box::new(Self::activate(runtime, installed))),
            None => BackendStatus::NeedsPreparation,
        })
    }

    fn prepare(
        &self,
        runtime: &ButlerRuntime,
        events: PackageEvents<'_>,
    ) -> Result<PreparedInstallation> {
        ensure!(
            !self.disabled,
            "Package preparation is disabled by --no-bundler"
        );
        let options = self
            .options(runtime)?
            .context("No project file, gems.rb or Gemfile found")?;
        if let Some(installed) = rb_packages::prepared(&options)? {
            return Ok(PreparedInstallation {
                runtime: Self::activate(runtime, installed),
                changed: false,
            });
        }
        let prepared = rb_packages::prepare(options, events.tasks)?;
        let changed = prepared.changed;
        Ok(PreparedInstallation {
            runtime: Self::activate(runtime, prepared.installed),
            changed,
        })
    }
}
