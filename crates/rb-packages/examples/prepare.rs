use rb_packages::{PrepareOptions, prepare};
use rb_task::{TaskEvent, TaskEventSink, TaskEvents};
use std::{path::PathBuf, sync::Arc};

struct Output;
impl TaskEventSink for Output {
    fn event(&self, event: TaskEvent) {
        match event {
            TaskEvent::Started { worker, label, .. } => eprintln!("worker {worker}: {label}"),
            TaskEvent::Output { worker, line, .. } => eprintln!("worker {worker}: {line}"),
            _ => {}
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let project = PathBuf::from(arguments.next().unwrap_or_else(|| ".".into()));
    let ruby = PathBuf::from(arguments.next().unwrap_or_else(|| "ruby".into()));
    let cache = PathBuf::from(
        arguments
            .next()
            .unwrap_or_else(|| project.join(".rb/cache").into()),
    );
    let mut options = PrepareOptions::new(project, ruby, cache);
    if let Ok(seconds) = std::env::var("RB_GEM_BUILD_TIMEOUT") {
        options.install.build.timeout = std::time::Duration::from_secs(seconds.parse()?);
    }
    if let Ok(jobs) = std::env::var("RB_GEM_BUILD_JOBS") {
        options.install.build.jobs = jobs.parse()?;
    }
    if let Some(path) = std::env::var_os("RB_GEM_BUILD_DEPENDENCIES") {
        options.install.build.dependencies = serde_json::from_slice(&std::fs::read(path)?)?;
    }
    options.offline = std::env::var_os("RB_PACKAGES_OFFLINE").is_some();
    options.plan.mode = if std::env::var_os("RB_PACKAGES_LOCKED").is_some() {
        rb_packages::LockMode::Locked
    } else {
        rb_packages::LockMode::Prefer
    };
    options
        .install
        .build
        .environment
        .insert("CCACHE_DISABLE".into(), "1".into());
    let events = TaskEvents::default();
    events.subscribe(Arc::new(Output));
    let result = prepare(options, events)?;
    println!(
        "{} installed, {} reused in {}",
        result.installed.installed,
        result.installed.reused,
        result.installed.gem_home.display()
    );
    for (key, value) in result.installed.environment() {
        println!("{key}={value}");
    }
    Ok(())
}
