use crate::{config::TrackedConfig, output::Output};
use rb_core::{butler::ButlerError, project::ProjectRuntime};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Serialize)]
pub(crate) struct ScriptReport {
    pub command: String,
    pub description: Option<String>,
}
#[derive(Serialize)]
pub(crate) struct ProjectSummary {
    pub name: Option<String>,
    pub description: Option<String>,
    pub file: PathBuf,
    pub config_filename: String,
    pub scripts: BTreeMap<String, ScriptReport>,
}
impl ProjectSummary {
    pub(crate) fn collect(project: &ProjectRuntime) -> Self {
        Self {
            name: project.metadata.name.clone(),
            description: project.metadata.description.clone(),
            file: project.rbproject_path(),
            config_filename: project.config_filename.clone(),
            scripts: project
                .script_names()
                .into_iter()
                .map(|name| {
                    let script = project.get_script(name).unwrap();
                    (
                        name.to_string(),
                        ScriptReport {
                            command: script.command().to_string(),
                            description: script.description().map(str::to_owned),
                        },
                    )
                })
                .collect(),
        }
    }
}
#[derive(Serialize)]
pub(crate) struct ProjectReport {
    pub project: Option<ProjectSummary>,
    pub configuration: super::config::ConfigurationReport,
}
pub(crate) fn project_command(
    config: &TrackedConfig,
    project: Option<ProjectRuntime>,
    output: &Output,
) -> Result<(), ButlerError> {
    log::info!("Inspecting project configuration");
    let report = ProjectReport {
        project: project.as_ref().map(ProjectSummary::collect),
        configuration: super::config::ConfigurationReport::collect(config),
    };
    output.emit("info.project", &report, || {
        crate::presentation::project::document(&report)
    })
}
