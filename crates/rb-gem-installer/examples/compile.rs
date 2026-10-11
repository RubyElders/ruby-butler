use rb_gem_installer::{BuildEvent, BuildOptions, NativeBuilder};
use std::{io::Write, time::Duration};

fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let archive = arguments
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected archive path"))?;
    let destination = arguments
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected destination path"))?;
    let ruby = arguments.next().unwrap_or_else(|| "ruby".into());
    let mut options = BuildOptions::new(ruby);
    options.arguments = arguments.collect();
    if let Ok(seconds) = std::env::var("RB_GEM_BUILD_TIMEOUT") {
        options.timeout = Duration::from_secs(seconds.parse()?);
    }
    if let Ok(jobs) = std::env::var("RB_GEM_BUILD_JOBS") {
        options.jobs = jobs.parse()?;
    }
    if let Some(paths) = std::env::var_os("RB_GEM_BUILD_LOAD_PATHS") {
        options.load_paths = std::env::split_paths(&paths).collect();
    }
    if let Some(path) = std::env::var_os("RB_GEM_BUILD_DEPENDENCIES") {
        options.dependencies = serde_json::from_slice(&std::fs::read(path)?)?;
    }
    let builder = NativeBuilder::new(options)?;
    let (metadata, report) = rb_gem_installer::install_with_builder(
        std::fs::File::open(archive)?,
        destination,
        &builder,
        |event| {
            if let BuildEvent::Output { bytes, .. } = event {
                let _ = std::io::stderr().write_all(&bytes);
            }
        },
    )?;
    println!(
        "Installed {} with {} build outputs",
        metadata.id,
        report.artifacts.len()
    );
    Ok(())
}
