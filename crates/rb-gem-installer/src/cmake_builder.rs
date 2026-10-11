use crate::{BuildEvent, BuildStage, NativeBuilder};
use anyhow::{Context, Result};
use std::{fs, path::Path};

pub(crate) fn build(
    builder: &NativeBuilder,
    path: &Path,
    extension: &Path,
    output: &Path,
    events: &mut impl FnMut(BuildEvent),
) -> Result<()> {
    let directory = path.parent().context("Missing CMake directory")?;
    let build = output.join("cmake");
    let mut command = builder.command(&builder.options.cmake, directory);
    command
        .arg("-S")
        .arg(directory)
        .arg("-B")
        .arg(&build)
        .arg(format!(
            "-DCMAKE_LIBRARY_OUTPUT_DIRECTORY={}",
            output.display()
        ))
        .arg(format!(
            "-DCMAKE_RUNTIME_OUTPUT_DIRECTORY={}",
            output.display()
        ))
        .arg(format!(
            "-DRuby_EXECUTABLE={}",
            builder.ruby.executable.display()
        ))
        .args(&builder.options.arguments);
    builder.run(command, extension, BuildStage::Configure, events)?;
    let mut command = builder.command(&builder.options.cmake, directory);
    command
        .arg("--build")
        .arg(&build)
        .args(["--config", "Release", "--parallel"])
        .arg(builder.options.jobs.to_string());
    builder.run(command, extension, BuildStage::Compile, events)?;
    fs::remove_dir_all(build)?;
    Ok(())
}
