use anyhow::{Context, Result, ensure};
use semver::Version;
use std::{
    path::Path,
    process::{Command, Stdio},
};

pub fn complete(path: &Path) -> bool {
    path.join("bin/ruby").is_file() && path.join("lib/ruby").is_dir()
}

pub(crate) fn validate(
    path: &Path,
    version: &Version,
    directory: &Path,
    environment: &std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
) -> Result<()> {
    ensure!(
        complete(path),
        "Ruby archive is missing its executable or standard library"
    );
    let gem_home = path
        .join("lib/ruby/gems")
        .join(format!("{}.{}.0", version.major, version.minor));
    let output = Command::new(path.join("bin/ruby"))
        .current_dir(directory)
        .env_clear()
        .envs(environment)
        .args([
            "-rrbconfig",
            "-ropenssl",
            "-ryaml",
            "-rreadline",
            "-e",
            "STDOUT.write(RUBY_VERSION)",
        ])
        .env_remove("RUBYOPT")
        .env_remove("RUBYLIB")
        .env("GEM_HOME", &gem_home)
        .env("GEM_PATH", &gem_home)
        .env_remove("RUBYGEMS_GEMDEPS")
        .stdin(Stdio::null())
        .output()
        .context("Unable to start the downloaded Ruby; check operating system compatibility")?;
    ensure!(
        output.status.success(),
        "Downloaded Ruby failed validation: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    ensure!(
        String::from_utf8_lossy(&output.stdout) == version.to_string(),
        "Downloaded Ruby version does not match the release"
    );
    Ok(())
}
