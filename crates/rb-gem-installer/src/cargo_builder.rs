use crate::{BuildEvent, BuildStage, NativeBuilder};
use anyhow::{Context, Result, ensure};
use std::{fs, path::Path};

pub(crate) fn build(
    builder: &NativeBuilder,
    manifest: &Path,
    extension: &Path,
    output: &Path,
    events: &mut impl FnMut(BuildEvent),
) -> Result<()> {
    let directory = manifest.parent().context("Missing Cargo directory")?;
    let mut command = builder.command(&builder.options.cargo, directory);
    command
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(manifest);
    let metadata = builder.run(command, extension, BuildStage::Configure, events)?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&metadata.stdout).context("Reading Cargo metadata")?;
    let package = metadata["packages"]
        .as_array()
        .context("Missing Cargo packages")?
        .iter()
        .find(|package| {
            package["manifest_path"].as_str().is_some_and(|path| {
                Path::new(path).canonicalize().ok().as_deref() == Some(manifest)
            })
        })
        .context("Missing extension Cargo package")?;
    let package_id = package["id"]
        .as_str()
        .context("Missing Cargo package identity")?;
    let target = output.join("cargo");
    let mut command = builder.command(&builder.options.cargo, directory);
    command
        .args([
            "rustc",
            "--lib",
            "--release",
            "--locked",
            "--message-format=json-render-diagnostics",
            "--manifest-path",
        ])
        .arg(manifest)
        .arg("--target-dir")
        .arg(&target)
        .args(&builder.options.arguments)
        .arg("--")
        .arg("--crate-type=cdylib");
    command
        .arg("-L")
        .arg(format!("native={}", builder.ruby.get("libdir")));
    if let Some(linker) = builder.ruby.linker.first() {
        command.arg("-C").arg(format!("linker={linker}"));
    }
    for argument in builder
        .ruby
        .linker
        .iter()
        .skip(1)
        .chain(builder.ruby.libraries.iter())
    {
        command.arg("-C").arg(format!("link-arg={argument}"));
    }
    if builder.ruby.get("target_os").contains("darwin") {
        command
            .arg("-C")
            .arg("link-arg=-Wl,-undefined,dynamic_lookup");
    }
    let compiled = builder.run(command, extension, BuildStage::Compile, events)?;
    let mut found = false;
    for line in compiled.stdout.split(|byte| *byte == b'\n') {
        let Ok(message) = serde_json::from_slice::<serde_json::Value>(line) else {
            continue;
        };
        if message["reason"] != "compiler-artifact" || message["package_id"] != package_id {
            continue;
        }
        let name = message["target"]["name"]
            .as_str()
            .context("Missing Cargo library name")?;
        for file in message["filenames"]
            .as_array()
            .context("Missing Cargo artifact filenames")?
        {
            let path = Path::new(file.as_str().context("Invalid Cargo artifact filename")?);
            if path
                .extension()
                .is_some_and(|suffix| suffix == "so" || suffix == "dylib" || suffix == "dll")
            {
                let canonical = path.canonicalize()?;
                ensure!(
                    canonical.starts_with(target.canonicalize()?),
                    "Cargo artifact is outside build directory"
                );
                let nesting = extension
                    .parent()
                    .unwrap_or(Path::new(""))
                    .components()
                    .skip(1)
                    .collect::<std::path::PathBuf>();
                let destination = output.join(nesting);
                fs::create_dir_all(&destination)?;
                fs::copy(
                    path,
                    destination.join(format!("{name}.{}", builder.ruby.get("DLEXT"))),
                )?;
                found = true;
            }
        }
    }
    ensure!(found, "Cargo did not produce an extension library");
    fs::remove_dir_all(target)?;
    Ok(())
}
