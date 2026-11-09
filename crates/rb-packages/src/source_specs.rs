use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
pub(crate) struct SourceSpecification {
    pub(crate) gemspec: PathBuf,
    pub(crate) inputs: Vec<PathBuf>,
    pub(crate) metadata: String,
    pub(crate) specification: String,
    pub(crate) files: Vec<PathBuf>,
    pub(crate) require_paths: Vec<PathBuf>,
    pub(crate) extensions: Vec<PathBuf>,
}

impl SourceSpecification {
    fn selections(&self) -> Result<Vec<PathBuf>> {
        let mut selected = if self.files.is_empty() {
            self.require_paths.clone()
        } else {
            self.files.clone()
        };
        for extension in &self.extensions {
            selected.push(
                extension
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .unwrap_or(extension)
                    .to_owned(),
            );
        }
        for path in selected.iter().chain(&self.require_paths) {
            ensure!(
                path.components().all(|part| matches!(
                    part,
                    std::path::Component::Normal(_) | std::path::Component::CurDir
                )),
                "Source specification path escapes its package"
            );
        }
        Ok(selected)
    }

    pub(crate) fn payload(&self, root: &Path) -> Result<Vec<PathBuf>> {
        crate::source_files::selected(root, &self.selections()?)
    }

    pub(crate) fn tracked_paths(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let directory = root.join(self.gemspec.parent().unwrap());
        let mut paths = self.inputs.clone();
        paths.push(root.join(&self.gemspec));
        paths.extend(self.selections()?.iter().map(|path| directory.join(path)));
        paths.extend(
            self.require_paths
                .iter()
                .filter(|path| path.as_path() != Path::new("."))
                .map(|path| directory.join(path)),
        );
        Ok(paths)
    }
}

#[cfg(test)]
pub(crate) fn read(
    ruby: &Path,
    directory: &Path,
    glob: Option<&str>,
) -> Result<Vec<SourceSpecification>> {
    read_with_environment(ruby, directory, glob, &std::env::vars_os().collect())
}

pub(crate) fn read_with_environment(
    ruby: &Path,
    directory: &Path,
    glob: Option<&str>,
    environment: &std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
) -> Result<Vec<SourceSpecification>> {
    let helper = tempfile::tempdir()?;
    for (path, source) in [
        (
            "lib/rb_gemfile/file_inputs.rb",
            include_str!("../../../gems/rb-gemfile/lib/rb_gemfile/file_inputs.rb"),
        ),
        (
            "lib/rb_gem_installer/source_specifications.rb",
            include_str!(
                "../../../gems/rb-gem-installer/lib/rb_gem_installer/source_specifications.rb"
            ),
        ),
    ] {
        let path = helper.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, source)?;
    }
    let result = tempfile::NamedTempFile::new()?;
    let request = tempfile::NamedTempFile::new()?;
    std::fs::write(
        request.path(),
        serde_json::to_vec(&serde_json::json!({
            "directory": directory,
            "glob": glob.unwrap_or(crate::constants::DEFAULT_GEMSPEC_GLOB),
            "output": result.path(),
        }))?,
    )?;
    let output = Command::new(ruby)
        .env_clear()
        .envs(environment)
        .env_remove("RUBYOPT")
        .env_remove("RUBYLIB")
        .env_remove("BUNDLE_GEMFILE")
        .arg("-r").arg(helper.path().join("lib/rb_gemfile/file_inputs.rb"))
        .arg("-r").arg(helper.path().join("lib/rb_gem_installer/source_specifications.rb"))
        .args(["-e", "RubyButler::GemInstaller::SourceSpecifications.export(ARGV.fetch(0), RubyButler::FileInputs.new)"])
        .arg(request.path())
        .current_dir(directory)
        .output()
        .context("Evaluating source gemspecs")?;
    ensure!(
        output.status.success(),
        "Source gemspec evaluation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let specs: Vec<SourceSpecification> = serde_json::from_slice(&std::fs::read(result.path())?)
        .context("Reading source gemspec response")?;
    ensure!(!specs.is_empty(), "Source contains no matching gemspecs");
    Ok(specs)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};

    #[test]
    fn wildcard_expansion_cannot_shift_helper_arguments() {
        let root = tempfile::tempdir().unwrap();
        for name in ["one", "two"] {
            fs::write(root.path().join(format!("{name}.gemspec")), format!("Gem::Specification.new {{ |s| s.name='{name}'; s.version='1.0'; s.summary='Fixture'; s.authors=['Fixture'] }}\n")).unwrap();
        }
        let ruby = root.path().join("ruby-shim");
        fs::write(
            &ruby,
            include_str!("../tests/fixtures/expand_ruby_arguments.rb"),
        )
        .unwrap();
        fs::set_permissions(&ruby, fs::Permissions::from_mode(0o755)).unwrap();
        let specs = read(&ruby, root.path(), None).unwrap();
        assert_eq!(specs.len(), 2);
        assert_eq!(
            specs
                .iter()
                .map(|spec| spec.gemspec.clone())
                .collect::<Vec<_>>(),
            [PathBuf::from("one.gemspec"), PathBuf::from("two.gemspec")]
        );
    }
}
