use crate::{
    InstallOptions, InstalledBundle, LockMode, PackagePlan, PlanOptions, ProjectInput,
    RubyEnvironment, SourceIndex,
};
use anyhow::{Result, ensure};
use rb_task::TaskEvents;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Debug)]
pub struct PrepareOptions {
    pub project: PathBuf,
    pub project_file: Option<PathBuf>,
    pub record_state: bool,
    pub ruby: PathBuf,
    pub cache: PathBuf,
    pub offline: bool,
    pub plan: PlanOptions,
    pub install: InstallOptions,
}

impl PrepareOptions {
    pub fn new(
        project: impl Into<PathBuf>,
        ruby: impl Into<PathBuf>,
        cache: impl Into<PathBuf>,
    ) -> Self {
        let ruby = ruby.into();
        Self {
            project: project.into(),
            project_file: None,
            record_state: false,
            cache: cache.into(),
            offline: false,
            plan: PlanOptions::default(),
            install: InstallOptions::new(&ruby),
            ruby,
        }
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
    let ruby = options.ruby.clone();
    let runtime = crate::task::run("inspect Ruby", events.clone(), move |_| {
        RubyEnvironment::inspect(&ruby)
    })?;
    let project = options.project.clone();
    let ruby = options.ruby.clone();
    let project_file = options.project_file.clone();
    let input = crate::task::run("load dependencies", events.clone(), move |_| {
        ProjectInput::load_with_project_file(&project, &ruby, project_file.as_deref())
    })?;
    let observed = if let Some(candidate) = candidate.clone() {
        let options = options.clone();
        let input = input.clone();
        crate::task::run("track preparation inputs", events.clone(), move |_| {
            Ok(crate::prepared_state::attempt(
                candidate.observe(&options, &input),
            ))
        })?
    } else {
        None
    };
    let source_input = input.clone();
    let source_runtime = runtime.clone();
    let mut plan_options = options.plan.clone();
    plan_options.source_root = Some(input.root.clone());
    let cache = options.cache.clone();
    let offline = options.offline;
    let (plan, index) = crate::task::run("resolve dependencies", events.clone(), move |_| {
        let previous = match std::fs::read_to_string(&source_input.lockfile) {
            Ok(text) => Some(rb_lockfile::Lockfile::parse(&text)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let index = Arc::new(SourceIndex::new(
            &source_input,
            &source_runtime,
            &cache,
            offline,
            previous.as_ref(),
            plan_options.mode,
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
