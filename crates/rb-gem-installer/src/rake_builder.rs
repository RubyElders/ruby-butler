use crate::{BuildEvent, BuildStage, NativeBuilder};
use anyhow::{Context, Result};
use std::path::Path;

pub(crate) fn build(
    builder: &NativeBuilder,
    path: &Path,
    extension: &Path,
    output: &Path,
    events: &mut impl FnMut(BuildEvent),
) -> Result<()> {
    let directory = path.parent().context("Missing rake directory")?;
    let name = path.file_name().context("Missing rake filename")?;
    let configure = name == "mkrf_conf.rb";
    if configure {
        let mut command = builder.ruby_command(directory);
        command.arg(name).args(&builder.options.arguments);
        builder.run(command, extension, BuildStage::Configure, events)?;
    }
    let mut command = builder.ruby_command(directory);
    command.args(["-e", "require 'rake'; Rake.application.run", "--", "-f"]);
    command.arg(if configure {
        std::ffi::OsStr::new("Rakefile")
    } else {
        name
    });
    let output = output.display().to_string().replace('\\', "/");
    command
        .arg(format!("RUBYARCHDIR={output}"))
        .arg(format!("RUBYLIBDIR={output}"))
        .args(&builder.options.arguments);
    builder.run(command, extension, BuildStage::Compile, events)?;
    Ok(())
}
