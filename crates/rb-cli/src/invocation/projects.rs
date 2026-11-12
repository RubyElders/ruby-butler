use rb_core::project::ProjectRuntime;
use std::{
    cell::OnceCell,
    path::{Path, PathBuf},
};

const PROJECT_FILES: &[&str] = &["gem.kdl", "gem.toml", "rbproject.kdl", "rbproject.toml"];

#[derive(Clone)]
pub(crate) struct DependencyLocation {
    pub root: PathBuf,
    pub project_file: Option<PathBuf>,
    pub gemfile: Option<PathBuf>,
    pub explicit: bool,
}

pub(crate) struct ProjectLocations {
    pub script_file: Option<PathBuf>,
    pub dependency: Option<DependencyLocation>,
    pub ruby_input: Option<PathBuf>,
    pub bundler_gemfile: Option<PathBuf>,
    project: OnceCell<Result<Option<ProjectRuntime>, String>>,
}

impl ProjectLocations {
    pub fn discover(
        directory: &Path,
        project_file: Option<&Path>,
        gemfile_override: Option<&std::ffi::OsStr>,
    ) -> Self {
        let mut script_file = None;
        let mut dependency = None;
        let mut ruby_input = None;
        let mut bundler_gemfile = None;
        for root in directory.ancestors() {
            let project = PROJECT_FILES
                .iter()
                .map(|name| root.join(name))
                .find(|file| file.is_file());
            let gemfile = ["gems.rb", "Gemfile"]
                .iter()
                .map(|name| root.join(name))
                .find(|file| file.is_file());
            if script_file.is_none() {
                script_file = project.clone();
            }
            if dependency.is_none() && (project.is_some() || gemfile.is_some()) {
                dependency = Some(DependencyLocation {
                    root: root.into(),
                    project_file: project.clone(),
                    gemfile: gemfile.clone(),
                    explicit: false,
                });
            }
            if ruby_input.is_none() {
                ruby_input = project.or(gemfile.clone()).or_else(|| {
                    let file = root.join(".ruby-version");
                    file.is_file().then_some(file)
                });
            }
            if bundler_gemfile.is_none() {
                bundler_gemfile = gemfile;
            }
        }
        let gemfile_override = gemfile_override
            .filter(|file| !file.is_empty())
            .map(|file| directory.join(file));
        if let Some(file) = project_file {
            let file = directory.join(file);
            script_file = Some(file.clone());
            if let Some(gemfile) = &gemfile_override {
                bundler_gemfile = Some(gemfile.clone());
            }
            ruby_input = Some(file.clone());
            dependency = Some(DependencyLocation {
                root: file.parent().unwrap_or(directory).into(),
                project_file: Some(file),
                gemfile: gemfile_override.clone(),
                explicit: true,
            });
        } else if let Some(file) = gemfile_override {
            bundler_gemfile = Some(file.clone());
            ruby_input = Some(file.clone());
            dependency = Some(DependencyLocation {
                root: file.parent().unwrap_or(directory).into(),
                project_file: None,
                gemfile: Some(file),
                explicit: true,
            });
        }
        Self {
            script_file,
            dependency,
            ruby_input,
            bundler_gemfile,
            project: OnceCell::new(),
        }
    }

    pub fn script_project(&self) -> Result<Option<ProjectRuntime>, String> {
        self.project
            .get_or_init(|| {
                self.script_file
                    .as_ref()
                    .map(ProjectRuntime::from_file)
                    .transpose()
                    .map_err(|error| error.to_string())
            })
            .clone()
    }
}
