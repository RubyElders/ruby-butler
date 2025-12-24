use crate::butler::runtime_provider::RuntimeProvider;
use log::{debug, info};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub mod detector;

pub use detector::RbprojectDetector;

/// Represents a script definition in rbproject.toml
/// Supports both simple string format and detailed object format
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum ScriptDefinition {
    /// Simple format: script = "command"
    Simple(String),
    /// Detailed format: script = { command = "...", description = "..." }
    Detailed {
        command: String,
        #[serde(default)]
        description: Option<String>,
    },
}

impl ScriptDefinition {
    /// Get the command string
    pub fn command(&self) -> &str {
        match self {
            ScriptDefinition::Simple(cmd) => cmd,
            ScriptDefinition::Detailed { command, .. } => command,
        }
    }

    /// Get the optional description
    pub fn description(&self) -> Option<&str> {
        match self {
            ScriptDefinition::Simple(_) => None,
            ScriptDefinition::Detailed { description, .. } => description.as_deref(),
        }
    }
}

/// Project metadata from [project] section
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct ProjectMetadata {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct RbprojectConfig {
    #[serde(default)]
    project: ProjectMetadata,
    #[serde(default)]
    scripts: HashMap<String, ScriptDefinition>,
}

/// Parse KDL format project configuration
fn parse_kdl(content: &str, filename: &str) -> io::Result<RbprojectConfig> {
    let document: kdl::KdlDocument = content.parse().map_err(|e| {
        debug!("Failed to parse KDL: {}", e);
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {}: {}", filename, e),
        )
    })?;

    let mut metadata = ProjectMetadata::default();
    let mut scripts = HashMap::new();

    // Parse project node
    if let Some(project_node) = document.get("project") {
        if let Some(name_node) = project_node.children().and_then(|c| c.get("name"))
            && let Some(name_val) = name_node.entries().first()
            && let Some(name_str) = name_val.value().as_string()
        {
            metadata.name = Some(name_str.to_string());
        }
        if let Some(desc_node) = project_node.children().and_then(|c| c.get("description"))
            && let Some(desc_val) = desc_node.entries().first()
            && let Some(desc_str) = desc_val.value().as_string()
        {
            metadata.description = Some(desc_str.to_string());
        }
    }

    // Parse scripts node
    if let Some(scripts_node) = document.get("scripts")
        && let Some(children) = scripts_node.children()
    {
        for child in children.nodes() {
            let script_name = child.name().value().to_string();

            // Check if it's a simple string or detailed format
            if let Some(command_entry) = child.entries().first() {
                if let Some(command_str) = command_entry.value().as_string() {
                    // Simple format: script "command"
                    scripts.insert(
                        script_name.clone(),
                        ScriptDefinition::Simple(command_str.to_string()),
                    );
                }
            } else if let Some(script_children) = child.children() {
                // Detailed format with command and description nodes
                let mut command = None;
                let mut description = None;

                for prop in script_children.nodes() {
                    match prop.name().value() {
                        "command" => {
                            if let Some(cmd) =
                                prop.entries().first().and_then(|e| e.value().as_string())
                            {
                                command = Some(cmd.to_string());
                            }
                        }
                        "description" => {
                            if let Some(desc) =
                                prop.entries().first().and_then(|e| e.value().as_string())
                            {
                                description = Some(desc.to_string());
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(cmd) = command {
                    scripts.insert(
                        script_name.clone(),
                        ScriptDefinition::Detailed {
                            command: cmd,
                            description,
                        },
                    );
                }
            }
        }
    }

    Ok(RbprojectConfig {
        project: metadata,
        scripts,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRuntime {
    /// Root directory containing the project config file
    pub root: PathBuf,
    /// Name of the config file (rbproject.toml or gem.toml)
    pub config_filename: String,
    /// Project metadata
    pub metadata: ProjectMetadata,
    /// Scripts defined in the [scripts] section
    pub scripts: HashMap<String, ScriptDefinition>,
}

impl ProjectRuntime {
    /// Create a new ProjectRuntime from a directory containing a project config file
    pub fn new(
        root: impl AsRef<Path>,
        config_filename: impl Into<String>,
        metadata: ProjectMetadata,
        scripts: HashMap<String, ScriptDefinition>,
    ) -> Self {
        let root = root.as_ref().to_path_buf();
        let config_filename = config_filename.into();

        debug!("Creating ProjectRuntime for root: {}", root.display());
        debug!("Config file: {}", config_filename);
        if let Some(ref name) = metadata.name {
            debug!("Project name: {}", name);
        }
        debug!("Scripts defined: {:?}", scripts.keys().collect::<Vec<_>>());

        Self {
            root,
            config_filename,
            metadata,
            scripts,
        }
    }

    /// Load ProjectRuntime from a project config file
    pub fn from_file(config_path: impl AsRef<Path>) -> io::Result<Self> {
        let config_path = config_path.as_ref();

        // Extract the filename from the path
        let config_filename = config_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid config file path"))?
            .to_string();

        debug!("Loading project config from: {}", config_path.display());

        let content = fs::read_to_string(config_path)?;
        debug!("Read {} bytes from {}", content.len(), config_filename);

        // Parse based on file extension
        let config: RbprojectConfig = if config_filename.ends_with(".kdl") {
            parse_kdl(&content, &config_filename)?
        } else {
            // Default to TOML parsing
            toml::from_str(&content).map_err(|e| {
                debug!("Failed to parse TOML: {}", e);
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to parse {}: {}", config_filename, e),
                )
            })?
        };

        let root = config_path
            .parent()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Could not determine parent directory",
                )
            })?
            .to_path_buf();

        if let Some(ref name) = config.project.name {
            info!("Loaded project '{}' from {}", name, config_filename);
        } else {
            info!("Loaded {}", config_filename);
        }

        let script_names: Vec<&str> = config.scripts.keys().map(|s| s.as_str()).collect();
        info!(
            "Found {} script(s): {}",
            config.scripts.len(),
            if script_names.is_empty() {
                "none".to_string()
            } else {
                script_names.join(", ")
            }
        );

        for (name, script_def) in &config.scripts {
            if let Some(desc) = script_def.description() {
                debug!("Script '{}': {} ({})", name, script_def.command(), desc);
            } else {
                debug!("Script '{}': {}", name, script_def.command());
            }
        }

        Ok(Self::new(
            root,
            config_filename,
            config.project,
            config.scripts,
        ))
    }

    /// Returns the full path to the project config file
    pub fn rbproject_path(&self) -> PathBuf {
        self.root.join(&self.config_filename)
    }

    /// Check if a script with the given name exists
    pub fn has_script(&self, name: &str) -> bool {
        self.scripts.contains_key(name)
    }

    /// Get the script definition by name
    pub fn get_script(&self, name: &str) -> Option<&ScriptDefinition> {
        self.scripts.get(name)
    }

    /// Get the command string for a script by name
    pub fn get_script_command(&self, name: &str) -> Option<&str> {
        self.scripts.get(name).map(|s| s.command())
    }

    /// Get the description for a script by name
    pub fn get_script_description(&self, name: &str) -> Option<&str> {
        self.scripts.get(name).and_then(|s| s.description())
    }

    /// Get all script names
    pub fn script_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.scripts.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }
}

impl RuntimeProvider for ProjectRuntime {
    fn bin_dir(&self) -> Option<PathBuf> {
        // Project runtime doesn't add a bin directory to PATH
        None
    }

    fn gem_dir(&self) -> Option<PathBuf> {
        // Project runtime doesn't add a gem directory
        None
    }

    fn compose_version_detector(&self) -> crate::ruby::CompositeDetector {
        use crate::ruby::version_detector::{GemfileDetector, RubyVersionFileDetector};

        // Project environment: check .ruby-version first, then Gemfile
        crate::ruby::CompositeDetector::new(vec![
            Box::new(RubyVersionFileDetector),
            Box::new(GemfileDetector),
        ])
    }

    fn compose_gem_path_detector(
        &self,
    ) -> crate::gems::gem_path_detector::CompositeGemPathDetector {
        use crate::gems::gem_path_detector::{
            BundlerIsolationDetector, CustomGemBaseDetector, UserGemsDetector,
        };

        // Project environment: standard priority
        crate::gems::gem_path_detector::CompositeGemPathDetector::new(vec![
            Box::new(CustomGemBaseDetector),
            Box::new(BundlerIsolationDetector),
            Box::new(UserGemsDetector),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use tempfile::TempDir;

    fn create_rbproject_file(dir: &Path, content: &str) -> io::Result<PathBuf> {
        let rbproject_path = dir.join("rbproject.toml");
        fs::write(&rbproject_path, content)?;
        Ok(rbproject_path)
    }

    #[test]
    fn new_creates_project_runtime() {
        let temp_dir = TempDir::new().unwrap();
        let mut scripts = HashMap::new();
        scripts.insert(
            "test".to_string(),
            ScriptDefinition::Simple("rspec".to_string()),
        );
        scripts.insert(
            "lint".to_string(),
            ScriptDefinition::Simple("rubocop".to_string()),
        );

        let metadata = ProjectMetadata::default();
        let project =
            ProjectRuntime::new(temp_dir.path(), "rbproject.toml", metadata, scripts.clone());

        assert_eq!(project.root, temp_dir.path());
        assert_eq!(project.scripts, scripts);
    }

    #[test]
    fn from_file_parses_simple_scripts() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_content = r#"
[scripts]
test = "rspec"
lint = "rubocop"
server = "rails server -p 3000"
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), toml_content)?;

        let project = ProjectRuntime::from_file(&rbproject_path)?;

        assert_eq!(project.root, temp_dir.path());
        assert_eq!(project.scripts.len(), 3);
        assert_eq!(project.get_script_command("test"), Some("rspec"));
        assert_eq!(project.get_script_command("lint"), Some("rubocop"));
        assert_eq!(
            project.get_script_command("server"),
            Some("rails server -p 3000")
        );
        assert_eq!(project.get_script_description("test"), None);

        Ok(())
    }

    #[test]
    fn from_file_handles_empty_scripts_section() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_content = r#"
[scripts]
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), toml_content)?;

        let project = ProjectRuntime::from_file(&rbproject_path)?;

        assert_eq!(project.scripts.len(), 0);

        Ok(())
    }

    #[test]
    fn from_file_handles_missing_scripts_section() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_content = r#"
[other_section]
key = "value"
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), toml_content)?;

        let project = ProjectRuntime::from_file(&rbproject_path)?;

        assert_eq!(project.scripts.len(), 0);

        Ok(())
    }

    #[test]
    fn from_file_returns_error_for_invalid_toml() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let invalid_toml = r#"
[scripts
this is not valid toml
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), invalid_toml)?;

        let result = ProjectRuntime::from_file(&rbproject_path);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);

        Ok(())
    }

    #[test]
    fn rbproject_path_returns_correct_path() {
        let temp_dir = TempDir::new().unwrap();
        let project = ProjectRuntime::new(
            temp_dir.path(),
            "rbproject.toml",
            ProjectMetadata::default(),
            HashMap::new(),
        );

        let expected_path = temp_dir.path().join("rbproject.toml");
        assert_eq!(project.rbproject_path(), expected_path);
    }

    #[test]
    fn has_script_returns_true_for_existing_script() {
        let temp_dir = TempDir::new().unwrap();
        let mut scripts = HashMap::new();
        scripts.insert(
            "test".to_string(),
            ScriptDefinition::Simple("rspec".to_string()),
        );
        let project = ProjectRuntime::new(
            temp_dir.path(),
            "rbproject.toml",
            ProjectMetadata::default(),
            scripts,
        );

        assert!(project.has_script("test"));
        assert!(!project.has_script("nonexistent"));
    }

    #[test]
    fn get_script_command_returns_command_string() {
        let temp_dir = TempDir::new().unwrap();
        let mut scripts = HashMap::new();
        scripts.insert(
            "test".to_string(),
            ScriptDefinition::Simple("rspec".to_string()),
        );
        scripts.insert(
            "lint".to_string(),
            ScriptDefinition::Simple("rubocop -a".to_string()),
        );
        let project = ProjectRuntime::new(
            temp_dir.path(),
            "rbproject.toml",
            ProjectMetadata::default(),
            scripts,
        );

        assert_eq!(project.get_script_command("test"), Some("rspec"));
        assert_eq!(project.get_script_command("lint"), Some("rubocop -a"));
        assert_eq!(project.get_script_command("nonexistent"), None);
    }

    #[test]
    fn script_names_returns_sorted_list() {
        let temp_dir = TempDir::new().unwrap();
        let mut scripts = HashMap::new();
        scripts.insert(
            "server".to_string(),
            ScriptDefinition::Simple("rails server".to_string()),
        );
        scripts.insert(
            "test".to_string(),
            ScriptDefinition::Simple("rspec".to_string()),
        );
        scripts.insert(
            "lint".to_string(),
            ScriptDefinition::Simple("rubocop".to_string()),
        );
        let project = ProjectRuntime::new(
            temp_dir.path(),
            "rbproject.toml",
            ProjectMetadata::default(),
            scripts,
        );

        let names = project.script_names();

        assert_eq!(names, vec!["lint", "server", "test"]);
    }

    #[test]
    fn runtime_provider_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let project = ProjectRuntime::new(
            temp_dir.path(),
            "rbproject.toml",
            ProjectMetadata::default(),
            HashMap::new(),
        );

        assert_eq!(project.bin_dir(), None);
        assert_eq!(project.gem_dir(), None);
    }

    #[test]
    fn from_file_with_complex_commands() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_content = r#"
[scripts]
test = "rspec --format documentation"
server = "rails server -b 0.0.0.0 -p 3000"
console = "rails console"
deploy = "cap production deploy"
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), toml_content)?;

        let project = ProjectRuntime::from_file(&rbproject_path)?;

        assert_eq!(project.scripts.len(), 4);
        assert_eq!(
            project.get_script_command("test"),
            Some("rspec --format documentation")
        );
        assert_eq!(
            project.get_script_command("server"),
            Some("rails server -b 0.0.0.0 -p 3000")
        );

        Ok(())
    }

    #[test]
    fn from_file_parses_detailed_scripts() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_content = r#"
[scripts]
test = { command = "rspec", description = "Run test suite" }
lint = { command = "rubocop", description = "Run linter" }
server = { command = "rails server -p 3000" }
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), toml_content)?;

        let project = ProjectRuntime::from_file(&rbproject_path)?;

        assert_eq!(project.scripts.len(), 3);
        assert_eq!(project.get_script_command("test"), Some("rspec"));
        assert_eq!(
            project.get_script_description("test"),
            Some("Run test suite")
        );
        assert_eq!(project.get_script_command("lint"), Some("rubocop"));
        assert_eq!(project.get_script_description("lint"), Some("Run linter"));
        assert_eq!(
            project.get_script_command("server"),
            Some("rails server -p 3000")
        );
        assert_eq!(project.get_script_description("server"), None);

        Ok(())
    }

    #[test]
    fn from_file_parses_mixed_scripts() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_content = r#"
[scripts]
test = "rspec"
lint = { command = "rubocop", description = "Check code quality" }
server = "rails server"
deploy = { command = "cap production deploy", description = "Deploy to production" }
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), toml_content)?;

        let project = ProjectRuntime::from_file(&rbproject_path)?;

        assert_eq!(project.scripts.len(), 4);
        // Simple format
        assert_eq!(project.get_script_command("test"), Some("rspec"));
        assert_eq!(project.get_script_description("test"), None);
        // Detailed format
        assert_eq!(project.get_script_command("lint"), Some("rubocop"));
        assert_eq!(
            project.get_script_description("lint"),
            Some("Check code quality")
        );
        // Simple format
        assert_eq!(project.get_script_command("server"), Some("rails server"));
        assert_eq!(project.get_script_description("server"), None);
        // Detailed format
        assert_eq!(
            project.get_script_command("deploy"),
            Some("cap production deploy")
        );
        assert_eq!(
            project.get_script_description("deploy"),
            Some("Deploy to production")
        );

        Ok(())
    }

    #[test]
    fn script_definition_simple_variant() {
        let def = ScriptDefinition::Simple("test command".to_string());
        assert_eq!(def.command(), "test command");
        assert_eq!(def.description(), None);
    }

    #[test]
    fn script_definition_detailed_variant_with_description() {
        let def = ScriptDefinition::Detailed {
            command: "test command".to_string(),
            description: Some("Test description".to_string()),
        };
        assert_eq!(def.command(), "test command");
        assert_eq!(def.description(), Some("Test description"));
    }

    #[test]
    fn script_definition_detailed_variant_without_description() {
        let def = ScriptDefinition::Detailed {
            command: "test command".to_string(),
            description: None,
        };
        assert_eq!(def.command(), "test command");
        assert_eq!(def.description(), None);
    }

    #[test]
    fn from_file_parses_scripts_with_colons() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_content = r#"
[scripts]
test = "rspec"
"test:watch" = "guard"
"lint:fix" = { command = "rubocop -a", description = "Auto-fix linting issues" }
"db:migrate" = "rails db:migrate"
"deploy:production" = { command = "cap production deploy" }
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), toml_content)?;

        let project = ProjectRuntime::from_file(&rbproject_path)?;

        assert_eq!(project.scripts.len(), 5);
        assert_eq!(project.get_script_command("test"), Some("rspec"));
        assert_eq!(project.get_script_command("test:watch"), Some("guard"));
        assert_eq!(project.get_script_command("lint:fix"), Some("rubocop -a"));
        assert_eq!(
            project.get_script_description("lint:fix"),
            Some("Auto-fix linting issues")
        );
        assert_eq!(
            project.get_script_command("db:migrate"),
            Some("rails db:migrate")
        );
        assert_eq!(
            project.get_script_command("deploy:production"),
            Some("cap production deploy")
        );
        assert_eq!(project.get_script_description("deploy:production"), None);

        Ok(())
    }

    #[test]
    fn from_file_parses_project_metadata() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_content = r#"
[project]
name = "My Elegant Project"
description = "A sophisticated Ruby application managed with distinction"

[scripts]
test = "rspec"
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), toml_content)?;

        let project = ProjectRuntime::from_file(&rbproject_path)?;

        assert_eq!(
            project.metadata.name,
            Some("My Elegant Project".to_string())
        );
        assert_eq!(
            project.metadata.description,
            Some("A sophisticated Ruby application managed with distinction".to_string())
        );

        Ok(())
    }

    #[test]
    fn from_file_handles_missing_project_metadata() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_content = r#"
[scripts]
test = "rspec"
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), toml_content)?;

        let project = ProjectRuntime::from_file(&rbproject_path)?;

        assert_eq!(project.metadata.name, None);
        assert_eq!(project.metadata.description, None);

        Ok(())
    }

    #[test]
    fn from_file_handles_partial_project_metadata() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let toml_content = r#"
[project]
name = "Project Without Description"

[scripts]
test = "rspec"
"#;
        let rbproject_path = create_rbproject_file(temp_dir.path(), toml_content)?;

        let project = ProjectRuntime::from_file(&rbproject_path)?;

        assert_eq!(
            project.metadata.name,
            Some("Project Without Description".to_string())
        );
        assert_eq!(project.metadata.description, None);

        Ok(())
    }

    // KDL format tests
    #[test]
    fn from_file_parses_simple_kdl_scripts() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let kdl_content = r#"
scripts {
    test "rspec"
    lint "rubocop"
    server "rails server -p 3000"
}
"#;
        let kdl_path = temp_dir.path().join("rb.kdl");
        fs::write(&kdl_path, kdl_content)?;

        let project = ProjectRuntime::from_file(&kdl_path)?;

        assert_eq!(project.root, temp_dir.path());
        assert_eq!(project.config_filename, "rb.kdl");
        assert_eq!(project.scripts.len(), 3);
        assert_eq!(project.get_script_command("test"), Some("rspec"));
        assert_eq!(project.get_script_command("lint"), Some("rubocop"));
        assert_eq!(
            project.get_script_command("server"),
            Some("rails server -p 3000")
        );

        Ok(())
    }

    #[test]
    fn from_file_parses_detailed_kdl_scripts() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let kdl_content = r#"
scripts {
    test {
        command "rspec"
        description "Run test suite"
    }
    lint {
        command "rubocop"
        description "Run linter"
    }
    server {
        command "rails server -p 3000"
    }
}
"#;
        let kdl_path = temp_dir.path().join("rb.kdl");
        fs::write(&kdl_path, kdl_content)?;

        let project = ProjectRuntime::from_file(&kdl_path)?;

        assert_eq!(project.scripts.len(), 3);
        assert_eq!(project.get_script_command("test"), Some("rspec"));
        assert_eq!(
            project.get_script_description("test"),
            Some("Run test suite")
        );
        assert_eq!(project.get_script_command("lint"), Some("rubocop"));
        assert_eq!(project.get_script_description("lint"), Some("Run linter"));
        assert_eq!(
            project.get_script_command("server"),
            Some("rails server -p 3000")
        );
        assert_eq!(project.get_script_description("server"), None);

        Ok(())
    }

    #[test]
    fn from_file_parses_kdl_with_project_metadata() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let kdl_content = r#"
project {
    name "My KDL Project"
    description "A project configured with KDL"
}

scripts {
    test "rspec"
    lint "rubocop"
}
"#;
        let kdl_path = temp_dir.path().join("gem.kdl");
        fs::write(&kdl_path, kdl_content)?;

        let project = ProjectRuntime::from_file(&kdl_path)?;

        assert_eq!(project.config_filename, "gem.kdl");
        assert_eq!(project.metadata.name, Some("My KDL Project".to_string()));
        assert_eq!(
            project.metadata.description,
            Some("A project configured with KDL".to_string())
        );
        assert_eq!(project.scripts.len(), 2);

        Ok(())
    }

    #[test]
    fn from_file_handles_empty_kdl_scripts() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let kdl_content = r#"
scripts {
}
"#;
        let kdl_path = temp_dir.path().join("rb.kdl");
        fs::write(&kdl_path, kdl_content)?;

        let project = ProjectRuntime::from_file(&kdl_path)?;

        assert_eq!(project.scripts.len(), 0);

        Ok(())
    }

    #[test]
    fn from_file_handles_kdl_without_project_section() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let kdl_content = r#"
scripts {
    test "rspec"
}
"#;
        let kdl_path = temp_dir.path().join("rb.kdl");
        fs::write(&kdl_path, kdl_content)?;

        let project = ProjectRuntime::from_file(&kdl_path)?;

        assert_eq!(project.metadata.name, None);
        assert_eq!(project.metadata.description, None);
        assert_eq!(project.scripts.len(), 1);

        Ok(())
    }

    #[test]
    fn from_file_returns_error_for_invalid_kdl() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let invalid_kdl = r#"
scripts {
    test "missing closing quote
}
"#;
        let kdl_path = temp_dir.path().join("rb.kdl");
        fs::write(&kdl_path, invalid_kdl)?;

        let result = ProjectRuntime::from_file(&kdl_path);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);

        Ok(())
    }
}
