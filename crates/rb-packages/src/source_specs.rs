use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
pub(crate) struct SourceSpecification {
    pub(crate) gemspec: PathBuf,
    pub(crate) metadata: String,
    pub(crate) specification: String,
    pub(crate) files: Vec<PathBuf>,
    pub(crate) require_paths: Vec<PathBuf>,
    pub(crate) extensions: Vec<PathBuf>,
}

impl SourceSpecification {
    pub(crate) fn payload(&self, root: &Path) -> Result<Vec<PathBuf>> {
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
        for path in &selected {
            ensure!(
                path.components().all(|part| matches!(
                    part,
                    std::path::Component::Normal(_) | std::path::Component::CurDir
                )),
                "Source specification path escapes its package"
            );
        }
        Ok(crate::source_files::files(root)?
            .into_iter()
            .filter(|path| {
                selected.iter().any(|selection| {
                    path == selection || path.starts_with(selection) || selection == Path::new(".")
                })
            })
            .collect())
    }
}

pub(crate) fn read(
    ruby: &Path,
    directory: &Path,
    glob: Option<&str>,
) -> Result<Vec<SourceSpecification>> {
    let helper = tempfile::Builder::new().suffix(".rb").tempfile()?;
    std::fs::write(
        helper.path(),
        include_str!(
            "../../../gems/rb-gem-installer/lib/rb_gem_installer/source_specifications.rb"
        ),
    )?;
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
    let output = Command::new(ruby).env_remove("RUBYOPT").env_remove("RUBYLIB").env_remove("BUNDLE_GEMFILE")
        .args(["-rjson", "-r"]).arg(helper.path()).args(["-e", "request = JSON.parse(File.read(ARGV.fetch(0))); specs = RubyButler::GemInstaller::SourceSpecifications.read(request.fetch('directory'), request.fetch('glob')); File.write(request.fetch('output'), JSON.generate(specs))"])
        .arg(request.path())
        .current_dir(directory).output().context("Evaluating source gemspecs")?;
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
