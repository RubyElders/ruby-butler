use crate::{
    InstallOptions, InstalledBundle, LockMode, PackagePlan, PlanOptions, ProjectInput,
    RubyEnvironment, SourceIndex,
};
use anyhow::{Result, ensure};
use rb_task::TaskEvents;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct PrepareOptions {
    pub project: PathBuf,
    pub project_file: Option<PathBuf>,
    pub gemfile: Option<PathBuf>,
    pub record_state: bool,
    pub ruby: PathBuf,
    pub cache: PathBuf,
    pub offline: bool,
    pub plan: PlanOptions,
    pub install: InstallOptions,
}

impl std::fmt::Debug for PrepareOptions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PrepareOptions")
            .field("project", &self.project)
            .field("project_file", &self.project_file)
            .field("record_state", &self.record_state)
            .field("ruby", &self.ruby)
            .field("cache", &self.cache)
            .field("offline", &self.offline)
            .field("plan", &self.plan)
            .field("install", &self.install)
            .finish()
    }
}

impl PrepareOptions {
    pub fn with_inputs(
        project: impl Into<PathBuf>,
        ruby: impl Into<PathBuf>,
        cache: impl Into<PathBuf>,
        directory: PathBuf,
        environment: std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
    ) -> Self {
        let ruby = ruby.into();
        Self {
            project: project.into(),
            project_file: None,
            gemfile: environment
                .get(std::ffi::OsStr::new("BUNDLE_GEMFILE"))
                .filter(|file| !file.is_empty())
                .map(|file| directory.join(file)),
            record_state: false,
            cache: cache.into(),
            offline: false,
            plan: PlanOptions::default(),
            install: InstallOptions::with_inputs(&ruby, directory, environment),
            ruby,
        }
    }

    pub(crate) fn evaluator(&self) -> rb_gemfile::Evaluator {
        rb_gemfile::Evaluator::with_inputs(
            &self.ruby,
            self.install.build.directory().to_owned(),
            self.install.build.inherited_environment().clone(),
        )
    }

    pub fn new(
        project: impl Into<PathBuf>,
        ruby: impl Into<PathBuf>,
        cache: impl Into<PathBuf>,
    ) -> Self {
        Self::with_inputs(
            project,
            ruby,
            cache,
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            std::env::vars_os().collect(),
        )
    }
}

#[derive(Clone)]
pub struct PreparedBundle {
    pub input: ProjectInput,
    pub plan: PackagePlan,
    pub installed: InstalledBundle,
    pub fingerprint: String,
    pub changed: bool,
}

pub fn prepare(options: PrepareOptions, events: TaskEvents) -> Result<PreparedBundle> {
    ensure!(options.install.workers > 0, "Worker count must be positive");
    let project = options.project.clone();
    let _project_lock = crate::task::run("lock project", events.clone(), move |_| {
        crate::prepared_state::lock(&project)
    })?;
    crate::prepared_state::invalidate(&options.project)?;
    let candidate = if options.record_state {
        let options = options.clone();
        crate::task::run("check dependency inputs", events.clone(), move |_| {
            Ok(crate::prepared_state::attempt(
                crate::prepared_state::Candidate::capture(&options),
            ))
        })?
    } else {
        None
    };
    let cache_options = options.clone();
    let runtime_options = options.clone();
    let ruby = options.ruby.clone();
    let runtime = crate::task::run("inspect Ruby", events.clone(), move |_| {
        RubyEnvironment::inspect_with_inputs(
            &ruby,
            runtime_options.install.build.directory(),
            runtime_options.install.build.inherited_environment(),
        )
    })?;
    let project = options.project.clone();
    let ruby = options.ruby.clone();
    let project_file = options.project_file.clone();
    let input_options = options.clone();
    let input = crate::task::run("load dependencies", events.clone(), move |_| {
        ProjectInput::load_with_inputs(
            &project,
            &ruby,
            project_file.as_deref(),
            input_options.gemfile.as_deref(),
            input_options.install.build.directory(),
            input_options.install.build.inherited_environment(),
        )
    })?;
    let source_input = input.clone();
    let source_runtime = runtime.clone();
    let mut plan_options = options.plan.clone();
    plan_options.source_root = Some(input.root.clone());
    let cache = options.cache.clone();
    let offline = options.offline;
    let source_environment = options.install.build.inherited_environment().clone();
    let (plan, index) = crate::task::run("resolve dependencies", events.clone(), move |_| {
        let previous = match std::fs::read_to_string(&source_input.lockfile) {
            Ok(text) => Some(rb_lockfile::Lockfile::parse(&text)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let index = Arc::new(SourceIndex::with_environment(
            &source_input,
            &source_runtime,
            &cache,
            offline,
            previous.as_ref(),
            plan_options.mode,
            &source_environment,
        )?);
        let plan = crate::plan(
            &source_input.manifest,
            &source_runtime,
            previous.as_ref(),
            index.as_ref(),
            &plan_options,
        )?;
        Ok((plan, index))
    })?;
    let observed = if let Some(candidate) = candidate.clone() {
        let options = options.clone();
        let input = input.clone();
        let source_inputs = index.preparation_inputs();
        crate::task::run("check local gem files", events.clone(), move |_| {
            Ok(crate::prepared_state::attempt(candidate.observe(
                &options,
                &input,
                &source_inputs,
            )))
        })?
    } else {
        None
    };
    ensure!(
        options.plan.mode != LockMode::Locked || !plan.lock_changed,
        "Lockfile changed in locked mode"
    );
    let fingerprint =
        crate::freshness::fingerprint(&plan, &input.manifest, &runtime, &options.install)?;
    let identity_file = input
        .root
        .join(crate::constants::STATE_DIRECTORY)
        .join(crate::constants::GEM_HOME_DIRECTORY)
        .join(crate::constants::BUNDLE_FINGERPRINT);
    let previous_identity = std::fs::read_to_string(&identity_file).ok();
    let layout =
        crate::BundleLayout::new(&input.root, &runtime.engine, &runtime.version.to_string());
    let mut installed = if previous_identity.as_deref() == Some(&fingerprint)
        && layout.gem_home.is_dir()
        && index.sources_current(&layout.gem_home, &plan, &runtime)?
    {
        InstalledBundle {
            gem_home: layout.gem_home,
            bundle_path: layout.bundle,
            installed: 0,
            reused: plan
                .packages
                .iter()
                .filter(|package| package.install)
                .count(),
            without: plan.without.iter().cloned().collect(),
            with: plan.with.iter().cloned().collect(),
            gemfile: None,
        }
    } else {
        crate::install(
            &input.root,
            &plan,
            &runtime,
            index.clone(),
            options.install,
            events.clone(),
        )?
    };
    let compatibility_input = input.clone();
    let compatibility_plan = plan.clone();
    let compatibility_bundle = installed.clone();
    installed.gemfile = Some(crate::task::run(
        "prepare Bundler compatibility",
        events.clone(),
        move |_| {
            crate::source_compatibility::write(
                &compatibility_input,
                &compatibility_plan,
                &compatibility_bundle,
                index.as_ref(),
            )
        },
    )?);
    if plan.lock_changed {
        let lockfile = input.lockfile.clone();
        let rendered = plan.lockfile.render()?;
        crate::task::run("write lockfile", events.clone(), move |_| {
            rb_gemfile::write_project(&lockfile, &rendered)?;
            Ok(())
        })?;
    }
    let changed = previous_identity.as_deref() != Some(&fingerprint) || installed.installed > 0;
    rb_gemfile::write_project(&identity_file, &fingerprint)?;
    let prepared = PreparedBundle {
        input,
        plan,
        installed,
        fingerprint,
        changed,
    };
    if let (Some(candidate), Some((paths, before))) = (candidate, observed) {
        let cache_bundle = prepared.clone();
        crate::task::run("record prepared environment", events, move |_| {
            crate::prepared_state::attempt(candidate.clone().publish(
                &cache_options,
                &cache_bundle,
                paths.clone(),
                before.clone(),
            ));
            Ok(())
        })?;
    }
    Ok(prepared)
}
