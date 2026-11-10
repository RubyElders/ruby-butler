mod projects;
pub(crate) use projects::{DependencyLocation, ProjectLocations};

use crate::config::TrackedConfig;
use rb_core::butler::{ButlerError, ButlerRuntime};
use std::path::PathBuf;

pub(crate) struct SelectedRuby {
    pub runtime: rb_core::RubyRuntime,
    pub installations: Vec<rb_core::RubyRuntime>,
}

/// Context information for command execution and error handling
pub struct CommandContext {
    config: TrackedConfig,
    project_file: Option<PathBuf>,
    origin: rb_core::process_context::ProcessContext,
    pub(crate) process: rb_core::process_context::ProcessContext,
    pub(crate) ruby: Option<SelectedRuby>,
    discovered_ruby: std::cell::OnceCell<Option<SelectedRuby>>,
    inventory: std::cell::OnceCell<Vec<rb_core::RubyRuntime>>,
    projects: std::cell::OnceCell<ProjectLocations>,
    pub(crate) completion_matches: Option<clap::ArgMatches>,
}

impl CommandContext {
    pub(crate) fn for_completion(&self, line: &str, point: &str) -> Result<Self, ButlerError> {
        use clap::FromArgMatches;
        let matches = crate::completion::completed_matches_in(
            crate::completion::before_cursor(line, point),
            self.origin.home_directory(),
        );
        let mut config = crate::config::RbConfig::from_arg_matches(&matches)
            .map_err(|error| ButlerError::General(error.to_string()))?;
        let config_file = matches.get_one::<PathBuf>("config_file").cloned();
        let project_file = matches.get_one::<PathBuf>("project_file").cloned();
        let explicit = clap::parser::ValueSource::CommandLine;
        if matches.value_source("no_bundler") != Some(explicit) {
            config.no_bundler = None;
        }
        let selected_file = matches.value_source("config_file") == Some(explicit);
        let file = if selected_file {
            crate::config::loader::load_config_in(config_file, &self.origin)
                .map_err(|error| ButlerError::General(error.to_string()))?
        } else {
            self.config.to_rb_config()
        };
        let config = TrackedConfig::from_merged_in(&config, &file, &self.origin);
        let mut context = Self::from_inputs(
            config,
            project_file.or_else(|| self.project_file.clone()),
            self.origin.clone(),
        );
        context.completion_matches = Some(matches);
        Ok(context)
    }

    pub fn new(config: TrackedConfig, project_file: Option<PathBuf>) -> std::io::Result<Self> {
        Ok(Self::from_inputs(
            config,
            project_file,
            rb_core::process_context::ProcessContext::capture()?,
        ))
    }

    pub fn from_inputs(
        config: TrackedConfig,
        project_file: Option<PathBuf>,
        inputs: rb_core::process_context::ProcessContext,
    ) -> Self {
        let process = inputs.at(config.work_dir.get());
        let project_file = project_file.map(|file| process.directory().join(file));
        Self {
            config,
            project_file,
            origin: inputs,
            process,
            ruby: None,
            discovered_ruby: std::cell::OnceCell::new(),
            inventory: std::cell::OnceCell::new(),
            projects: std::cell::OnceCell::new(),
            completion_matches: None,
        }
    }

    pub(crate) fn projects(&self) -> &ProjectLocations {
        self.projects.get_or_init(|| {
            ProjectLocations::discover(
                self.process.directory(),
                self.project_file.as_deref(),
                self.process.var("BUNDLE_GEMFILE"),
            )
        })
    }

    pub(crate) fn script_project(
        &self,
    ) -> Result<Option<rb_core::project::ProjectRuntime>, ButlerError> {
        match self.projects().script_project() {
            Ok(project) => Ok(project),
            Err(error) if self.project_file.is_some() => Err(ButlerError::General(format!(
                "The specified project configuration could not be loaded from {}:\n{error}",
                self.project_file.as_ref().unwrap().display()
            ))),
            Err(error) => {
                log::warn!("Error detecting project config: {error}");
                Ok(None)
            }
        }
    }

    pub fn config(&self) -> &TrackedConfig {
        &self.config
    }
    pub fn project_file(&self) -> &Option<PathBuf> {
        &self.project_file
    }

    pub fn process(&self) -> &rb_core::process_context::ProcessContext {
        &self.process
    }

    pub(crate) fn validate_working_directory(&self) -> Result<(), ButlerError> {
        if !self.process.directory().is_dir() {
            return Err(ButlerError::General(format!(
                "Failed to change to directory '{}': directory does not exist",
                self.process.directory().display()
            )));
        }
        Ok(())
    }

    pub(crate) fn runtime(&self, skip_bundler: bool) -> Result<ButlerRuntime, ButlerError> {
        let selection = match &self.ruby {
            Some(selection) => Some(selection),
            None => self.discover_ruby()?.as_ref(),
        };
        let Some(selection) = selection else {
            return Ok(ButlerRuntime::empty_in(
                self.process.directory().join(self.config.rubies_dir.get()),
                self.process.clone(),
            ));
        };
        let bundler = if skip_bundler {
            None
        } else {
            self.projects().bundler_gemfile.as_ref().map(|file| {
                rb_core::BundlerRuntime::new(
                    file.parent().unwrap(),
                    selection.runtime.version.clone(),
                )
                .with_gemfile(file)
            })
        };
        Ok(ButlerRuntime::compose_selected(
            self.process.clone(),
            self.config.rubies_dir.get().clone(),
            self.config.ruby_version_for_runtime(),
            Some(self.config.gem_home.get().clone()),
            selection.installations.clone(),
            selection.runtime.clone(),
            bundler,
        ))
    }

    pub(crate) fn ruby_inventory(&self) -> Result<&Vec<rb_core::RubyRuntime>, ButlerError> {
        if let Some(inventory) = self.inventory.get() {
            return Ok(inventory);
        }
        let directory = self.process.directory().join(self.config.rubies_dir.get());
        let inventory = rb_core::ruby::RubyRuntimeDetector::discover(&directory).map_err(
            |error| match error {
                rb_core::ruby::RubyDiscoveryError::DirectoryNotFound(_) => {
                    ButlerError::RubiesDirectoryNotFound(self.config.rubies_dir.get().clone())
                }
                error => ButlerError::General(error.to_string()),
            },
        )?;
        let _ = self.inventory.set(inventory);
        Ok(self.inventory.get().unwrap())
    }

    fn discover_ruby(&self) -> Result<&Option<SelectedRuby>, ButlerError> {
        if let Some(selection) = self.discovered_ruby.get() {
            return Ok(selection);
        }
        let installations = self.ruby_inventory()?.clone();
        let selection = if installations.is_empty() {
            None
        } else {
            let request = crate::ruby_install::request::resolve(self, None)
                .map_err(|error| ButlerError::General(format!("{error:#}")))?;
            let matching = installations
                .iter()
                .find(|ruby| request.matches(&ruby.version));
            if matching.is_none() && self.config.ruby_version_for_runtime().is_some() {
                return Err(ButlerError::NoSuitableRuby(format!(
                    "Requested Ruby version {request} not found"
                )));
            }
            let runtime = matching.unwrap_or(&installations[0]).clone();
            Some(SelectedRuby {
                runtime,
                installations,
            })
        };
        let _ = self.discovered_ruby.set(selection);
        Ok(self.discovered_ruby.get().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RbConfig;
    use rb_core::{RubyRuntime, process_context::ProcessContext};
    use std::collections::BTreeMap;

    #[test]
    fn selected_ruby_is_composed_without_scanning_its_directory() {
        let directory = tempfile::tempdir().unwrap();
        let inputs = ProcessContext::new(directory.path().into(), BTreeMap::new());
        let requested = RbConfig {
            rubies_dir: Some(directory.path().join("not-present")),
            ..Default::default()
        };
        let config = TrackedConfig::from_merged_in(&requested, &RbConfig::default(), &inputs);
        let mut context = CommandContext::from_inputs(config, None, inputs);
        let ruby = RubyRuntime::new(
            rb_core::ruby::RubyType::CRuby,
            semver::Version::new(3, 4, 10),
            directory.path().join("selected-ruby"),
        );
        std::fs::write(directory.path().join(".ruby-version"), "invalid-version").unwrap();
        context.ruby = Some(SelectedRuby {
            runtime: ruby.clone(),
            installations: vec![ruby.clone()],
        });
        assert_eq!(
            context.runtime(false).unwrap().selected_ruby().unwrap(),
            &ruby
        );
    }

    #[test]
    fn inspection_reuses_inventory_and_requirement_only_within_its_context() {
        let directory = tempfile::tempdir().unwrap();
        let rubies = directory.path().join("rubies");
        std::fs::create_dir_all(rubies.join("ruby-3.4.10")).unwrap();
        std::fs::write(directory.path().join(".ruby-version"), "3.4.10").unwrap();
        let inputs = ProcessContext::new(directory.path().into(), BTreeMap::new());
        let requested = RbConfig {
            rubies_dir: Some(rubies.clone()),
            ..Default::default()
        };
        let config = TrackedConfig::from_merged_in(&requested, &RbConfig::default(), &inputs);
        let first = CommandContext::from_inputs(config.clone(), None, inputs.clone());
        assert_eq!(
            first
                .runtime(false)
                .unwrap()
                .selected_ruby()
                .unwrap()
                .version
                .to_string(),
            "3.4.10"
        );
        std::fs::remove_dir_all(rubies).unwrap();
        std::fs::write(directory.path().join(".ruby-version"), "invalid-version").unwrap();
        assert_eq!(
            first
                .runtime(false)
                .unwrap()
                .selected_ruby()
                .unwrap()
                .version
                .to_string(),
            "3.4.10"
        );
        let next = CommandContext::from_inputs(config, None, inputs);
        assert!(next.runtime(false).is_err());
    }

    #[test]
    fn gemfile_does_not_shadow_parent_script_settings() {
        let root = tempfile::tempdir().unwrap();
        let child = root.path().join("child");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(
            root.path().join("rbproject.toml"),
            "[project]\nname = 'scripts'\n[scripts]\nprobe = 'ruby -v'\n",
        )
        .unwrap();
        std::fs::write(child.join("Gemfile"), "gem 'rake'\n").unwrap();
        let inputs = ProcessContext::new(child.clone(), BTreeMap::new());
        let config =
            TrackedConfig::from_merged_in(&RbConfig::default(), &RbConfig::default(), &inputs);
        let context = CommandContext::from_inputs(config, None, inputs);
        let child = child.canonicalize().unwrap();
        assert_eq!(context.projects().dependency.as_ref().unwrap().root, child);
        assert_eq!(context.projects().ruby_input, Some(child.join("Gemfile")));
        assert_eq!(
            context.script_project().unwrap().unwrap().script_names(),
            ["probe"]
        );
    }

    #[test]
    fn project_settings_are_loaded_once_per_context() {
        let directory = tempfile::tempdir().unwrap();
        let project = directory.path().join("rbproject.toml");
        std::fs::write(&project, "[project]\nname = 'first'\n").unwrap();
        let inputs = ProcessContext::new(directory.path().into(), BTreeMap::new());
        let config =
            TrackedConfig::from_merged_in(&RbConfig::default(), &RbConfig::default(), &inputs);
        let first = CommandContext::from_inputs(config.clone(), None, inputs.clone());
        assert_eq!(
            first
                .script_project()
                .unwrap()
                .unwrap()
                .metadata
                .name
                .as_deref(),
            Some("first")
        );
        std::fs::write(&project, "[project]\nname = 'second'\n").unwrap();
        assert_eq!(
            first
                .script_project()
                .unwrap()
                .unwrap()
                .metadata
                .name
                .as_deref(),
            Some("first")
        );
        let second = CommandContext::from_inputs(config, None, inputs);
        assert_eq!(
            second
                .script_project()
                .unwrap()
                .unwrap()
                .metadata
                .name
                .as_deref(),
            Some("second")
        );
    }

    #[test]
    fn static_completion_does_not_discover_projects_or_rubies() {
        let root = tempfile::tempdir().unwrap();
        for backend in [
            crate::config::PackageBackendKind::Bundler,
            crate::config::PackageBackendKind::Native,
        ] {
            let inputs = ProcessContext::new(root.path().into(), BTreeMap::new());
            let requested = RbConfig {
                package_backend: Some(backend),
                rubies_dir: Some(root.path().join("missing")),
                ..Default::default()
            };
            let config = TrackedConfig::from_merged_in(&requested, &RbConfig::default(), &inputs);
            let context = CommandContext::from_inputs(config, None, inputs);
            let completion = context.for_completion("rb sy", "5").unwrap();
            crate::runtime_helpers::bash_complete_in(&completion, "rb sy", "5").unwrap();
            assert!(completion.projects.get().is_none());
            assert!(completion.inventory.get().is_none());
            assert!(completion.discovered_ruby.get().is_none());
            assert!(!root.path().join(".rb").exists());
        }
    }
}
