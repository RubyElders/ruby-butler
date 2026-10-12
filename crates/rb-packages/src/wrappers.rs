use anyhow::{Result, ensure};
use std::{fs, path::Path};

pub(crate) fn wrappers(home: &Path, metadata: &rb_gem_archive::GemMetadata) -> Result<()> {
    fs::create_dir_all(home.join(crate::constants::BIN_DIRECTORY))?;
    for executable in &metadata.executables {
        ensure!(
            !executable.is_empty()
                && Path::new(executable).components().count() == 1
                && executable
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')),
            "Invalid executable name"
        );
        let name = serde_json::to_string(&metadata.id.name)?;
        let version = serde_json::to_string(&metadata.id.version.to_string())?;
        let executable_literal = serde_json::to_string(executable)?;
        let script = format!(
            "#!/usr/bin/env ruby\nrequire 'rubygems'\nload Gem.activate_bin_path({name}, {executable_literal}, '= ' + {version})\n"
        );
        let path = home.join(crate::constants::BIN_DIRECTORY).join(executable);
        fs::write(&path, script)?;
        #[cfg(windows)]
        fs::write(
            home.join(crate::constants::BIN_DIRECTORY)
                .join(format!("{executable}.bat")),
            "@ECHO OFF\r\nruby.exe \"%~dpn0\" %*\r\n",
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}
