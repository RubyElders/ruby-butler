use crate::{BuildEvent, BuildStage, NativeBuilder};
use anyhow::{Context, Result};
use std::{ffi::OsString, path::Path};

pub(crate) fn build(
    builder: &NativeBuilder,
    path: &Path,
    extension: &Path,
    output: &Path,
    events: &mut impl FnMut(BuildEvent),
) -> Result<()> {
    let directory = path.parent().context("Missing build directory")?;
    if path.file_name().is_some_and(|name| name == "configure") {
        let mut command = builder.command("sh", directory);
        command
            .arg("./configure")
            .arg(format!(
                "--prefix=./{}",
                output
                    .file_name()
                    .context("Missing temporary output name")?
                    .to_string_lossy()
            ))
            .args(&builder.options.arguments);
        builder.run(command, extension, BuildStage::Configure, events)?;
        make(builder, directory, output, extension, events)
    } else {
        let mut command = builder.ruby_command(directory);
        command
            .arg(path.file_name().context("Missing extension filename")?)
            .args(&builder.options.arguments);
        builder.run(command, extension, BuildStage::Configure, events)?;
        if directory.join("Makefile").is_file() {
            make(builder, directory, output, extension, events)?;
        }
        Ok(())
    }
}

fn make(
    builder: &NativeBuilder,
    directory: &Path,
    output: &Path,
    extension: &Path,
    events: &mut impl FnMut(BuildEvent),
) -> Result<()> {
    let program = builder
        .options
        .make
        .clone()
        .or_else(|| {
            builder
                .options
                .environment
                .get(std::ffi::OsStr::new("MAKE"))
                .cloned()
        })
        .or_else(|| std::env::var_os("MAKE"))
        .unwrap_or_else(|| {
            OsString::from(if builder.ruby.get("target_os").contains("mswin") {
                "nmake"
            } else {
                "make"
            })
        });
    let is_nmake = program.to_string_lossy().contains("nmake");
    for (stage, target) in [
        (BuildStage::Compile, None),
        (BuildStage::Install, Some("install")),
    ] {
        let mut command = builder.command(&program, directory);
        if !is_nmake {
            command
                .arg(format!("-j{}", builder.options.jobs))
                .arg("DESTDIR=");
        }
        let relative = format!(
            "./{}",
            output
                .file_name()
                .context("Missing temporary output name")?
                .to_string_lossy()
        );
        command
            .arg(format!("sitearchdir={relative}"))
            .arg(format!("sitelibdir={relative}"));
        if let Some(target) = target {
            command.arg(target);
        }
        builder.run(command, extension, stage, events)?;
    }
    Ok(())
}
