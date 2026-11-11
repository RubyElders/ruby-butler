use crate::packages::PackageRuntime;
use log::info;
use rb_core::butler::ButlerError;
use rb_core::project::ProjectRuntime;

use super::exec::exec_command;

pub(crate) fn list_available_scripts(
    project: Option<ProjectRuntime>,
    output: &crate::output::Output,
) -> Result<(), ButlerError> {
    info!("Listing available project scripts");
    let project = project.ok_or_else(|| ButlerError::General("No project configuration detected in the current directory hierarchy.\n\nTo define project scripts, create one of these files (in priority order):\n  gem.kdl, gem.toml, rbproject.kdl, rbproject.toml\n\nOr specify a custom location: rb -P path/to/gem.kdl run".to_string()))?;
    let report = crate::commands::info::project::ProjectSummary::collect(&project);
    output.emit("run", &report, || {
        crate::presentation::scripts::document(&report)
    })
}

pub(crate) fn run_command(
    butler_runtime: PackageRuntime,
    script_name: Option<String>,
    args: Vec<String>,
    project_runtime: Option<ProjectRuntime>,
    backend: &crate::packages::PackageBackend,
    report: &mut crate::reporting::PreparationReport,
) -> Result<(), ButlerError> {
    if script_name.is_none() {
        return list_available_scripts(project_runtime, &crate::output::Output::default());
    }

    let script_name = script_name.unwrap();
    info!(
        "Executing project script '{}' with distinguished precision",
        script_name
    );

    let project = match project_runtime {
        Some(p) => p,
        None => {
            return Err(ButlerError::General(format!(
                "No project configuration detected in the current directory hierarchy.\n\nTo use project scripts, create one of these files: rbproject.toml, rb.toml, rb.kdl, gem.toml, gem.kdl\n\nOr specify a custom location with: rb -P path/to/rb.toml run {}",
                script_name
            )));
        }
    };

    if !project.has_script(&script_name) {
        return Err(ButlerError::General(format!(
            "The script '{}' is not defined in your project configuration",
            script_name
        )));
    }

    let command_str = project.get_script_command(&script_name).unwrap();

    info!("Executing script: {} → {}", script_name, command_str);

    let command_parts = parse_command(command_str);

    if command_parts.is_empty() {
        return Err(ButlerError::General(format!(
            "The script '{}' has an empty command",
            script_name
        )));
    }

    let mut full_args = command_parts;
    full_args.extend(args);

    info!("Delegating to exec command with args: {:?}", full_args);

    // Delegate to exec_command for consistent behavior (auto bundle exec, env composition)
    exec_command(butler_runtime, full_args, backend, report)
}

/// Parse a command string into program and arguments
/// This is a simple whitespace-based parser that respects quotes
fn parse_command(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_double_quotes = false;
    let mut in_single_quotes = false;

    for ch in command.chars() {
        match ch {
            '"' if !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
            }
            '\'' if !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
            }
            ' ' if !in_double_quotes && !in_single_quotes => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_simple() {
        assert_eq!(
            parse_command("ruby -v"),
            vec!["ruby".to_string(), "-v".to_string()]
        );
    }

    #[test]
    fn test_parse_command_with_multiple_args() {
        assert_eq!(
            parse_command("gem install bundler --version 2.4.0"),
            vec![
                "gem".to_string(),
                "install".to_string(),
                "bundler".to_string(),
                "--version".to_string(),
                "2.4.0".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_command_with_quotes() {
        assert_eq!(
            parse_command("rails new \"my app\""),
            vec!["rails".to_string(), "new".to_string(), "my app".to_string()]
        );
    }

    #[test]
    fn test_parse_command_with_extra_spaces() {
        assert_eq!(
            parse_command("ruby  -e   \"puts 'hello'\""),
            vec![
                "ruby".to_string(),
                "-e".to_string(),
                "puts 'hello'".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_command_with_single_quotes() {
        assert_eq!(
            parse_command("ruby -e 'puts ARGV.join(\", \")'"),
            vec![
                "ruby".to_string(),
                "-e".to_string(),
                "puts ARGV.join(\", \")".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_command_empty() {
        assert_eq!(parse_command(""), Vec::<String>::new());
    }

    #[test]
    fn test_parse_command_only_spaces() {
        assert_eq!(parse_command("   "), Vec::<String>::new());
    }
}
