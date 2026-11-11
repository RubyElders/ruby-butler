use crate::Cli;
use clap::CommandFactory;
use std::path::PathBuf;

pub(crate) struct CompletionEnvironment {
    pub bin_dirs: Vec<PathBuf>,
    pub project: Option<rb_core::project::ProjectRuntime>,
    pub directory: PathBuf,
    pub ruby_versions: Vec<String>,
}

pub(crate) fn before_cursor<'a>(line: &'a str, point: &str) -> &'a str {
    let mut end = point.parse::<usize>().unwrap_or(line.len()).min(line.len());
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    &line[..end]
}

#[cfg(test)]
pub(crate) fn completed_matches(line: &str) -> clap::ArgMatches {
    completed_matches_in(line, None)
}

pub(crate) fn completed_matches_in(line: &str, home: Option<PathBuf>) -> clap::ArgMatches {
    let mut lexer = shlex::Shlex::new(line);
    let mut words = lexer.by_ref().collect::<Vec<_>>();
    if !lexer.had_error && !line.ends_with(char::is_whitespace) {
        words.pop();
    }
    let mut command = Cli::command()
        .ignore_errors(true)
        .allow_external_subcommands(true);
    let ids = command
        .get_arguments()
        .map(|arg| arg.get_id().clone())
        .collect::<Vec<_>>();
    for id in ids {
        command = command.mut_arg(id, |arg| arg.env(None::<&str>));
    }
    for id in ["rubies_dir", "gem_home", "work_dir"] {
        let home = home.clone();
        command = command.mut_arg(id, |arg| {
            arg.value_parser(move |path: &str| {
                let expanded = home.as_ref().and_then(|home| {
                    if path == "~" {
                        Some(home.clone())
                    } else {
                        path.strip_prefix("~/").map(|suffix| home.join(suffix))
                    }
                });
                Ok::<PathBuf, String>(expanded.unwrap_or_else(|| PathBuf::from(path)))
            })
        });
    }
    command
        .disable_help_flag(true)
        .disable_version_flag(true)
        .try_get_matches_from(words)
        .unwrap_or_else(|_| Cli::command().ignore_errors(true).get_matches_from(["rb"]))
}

fn suggest_directories(current: &str, directory: &std::path::Path) {
    let current_path = std::path::Path::new(current);

    let (search_dir, prefix) = if current.is_empty() {
        (std::path::PathBuf::from("."), "")
    } else if current.ends_with('/') || current.ends_with(std::path::MAIN_SEPARATOR) {
        (current_path.to_path_buf(), "")
    } else {
        match current_path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => {
                let prefix = current_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                (parent.to_path_buf(), prefix)
            }
            _ => (std::path::PathBuf::from("."), current),
        }
    };

    let Ok(entries) = std::fs::read_dir(directory.join(&search_dir)) else {
        return;
    };

    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if !file_type.is_dir() {
            continue;
        }

        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };

        if name.starts_with('.') && !prefix.starts_with('.') {
            continue;
        }

        if !name.starts_with(prefix) {
            continue;
        }

        let candidate_path =
            if current.ends_with('/') || current.ends_with(std::path::MAIN_SEPARATOR) {
                format!("{}{}/", current, name)
            } else if let Some(parent) = current_path.parent() {
                if parent.as_os_str().is_empty() || parent == std::path::Path::new(".") {
                    format!("{}/", name)
                } else {
                    format!("{}/{}/", parent.display(), name)
                }
            } else {
                format!("{}/", name)
            };

        candidates.push(candidate_path);
    }

    candidates.sort();
    for candidate in candidates {
        println!("{}", candidate);
    }
}

fn suggest_files(current: &str, directory: &std::path::Path) {
    let current_path = std::path::Path::new(current);

    let (search_dir, prefix) = if current.is_empty() {
        (std::path::PathBuf::from("."), "")
    } else if current.ends_with('/') || current.ends_with(std::path::MAIN_SEPARATOR) {
        (current_path.to_path_buf(), "")
    } else {
        match current_path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => {
                let prefix = current_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                (parent.to_path_buf(), prefix)
            }
            _ => (std::path::PathBuf::from("."), current),
        }
    };

    let Ok(entries) = std::fs::read_dir(directory.join(&search_dir)) else {
        return;
    };

    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };

        if name.starts_with('.') && !prefix.starts_with('.') {
            continue;
        }

        if !name.starts_with(prefix) {
            continue;
        }

        let candidate_path =
            if current.ends_with('/') || current.ends_with(std::path::MAIN_SEPARATOR) {
                if file_type.is_dir() {
                    format!("{}{}/", current, name)
                } else {
                    format!("{}{}", current, name)
                }
            } else if let Some(parent) = current_path.parent() {
                if parent.as_os_str().is_empty() || parent == std::path::Path::new(".") {
                    if file_type.is_dir() {
                        format!("{}/", name)
                    } else {
                        name.to_string()
                    }
                } else if file_type.is_dir() {
                    format!("{}/{}/", parent.display(), name)
                } else {
                    format!("{}/{}", parent.display(), name)
                }
            } else if file_type.is_dir() {
                format!("{}/", name)
            } else {
                name.to_string()
            };

        candidates.push(candidate_path);
    }

    candidates.sort();
    for candidate in candidates {
        println!("{}", candidate);
    }
}

/// Generate dynamic completions based on current line and cursor position
pub(crate) fn generate_completions(
    line: &str,
    cursor_pos: &str,
    environment: &CompletionEnvironment,
    matches: &clap::ArgMatches,
) {
    let line = before_cursor(line, cursor_pos);

    let tokens =
        shlex::split(line).unwrap_or_else(|| line.split_whitespace().map(String::from).collect());
    let words: Vec<&str> = tokens.iter().map(String::as_str).collect();

    if words.is_empty() || words.len() == 1 {
        print_commands("");
        return;
    }

    let (current_word, prev_word) = if line.ends_with(' ') {
        ("", words.last().copied())
    } else {
        (
            words.last().copied().unwrap_or(""),
            words.get(words.len().saturating_sub(2)).copied(),
        )
    };

    let subcommand = matches.subcommand();
    let program_started = subcommand.is_some_and(|(name, args)| match name {
        "exec" => args.get_many::<String>("args").is_some(),
        "run" => args.get_one::<String>("script").is_some(),
        _ => false,
    });
    if program_started {
        return;
    }

    if let Some(prev) = prev_word {
        let command = active_command(matches);
        if let Some(values) = command
            .get_arguments()
            .filter(|arg| arg.get_action().takes_values())
            .find(|arg| {
                arg.get_long()
                    .is_some_and(|name| prev == format!("--{name}"))
                    || arg
                        .get_short()
                        .is_some_and(|name| prev == format!("-{name}"))
            })
            .and_then(|arg| arg.get_value_parser().possible_values())
        {
            for value in values
                .filter(|value| !value.is_hide_set() && value.get_name().starts_with(current_word))
            {
                println!("{}", value.get_name());
            }
            return;
        }
        if prev == "-r" || prev == "--ruby" {
            suggest_ruby_versions(&environment.ruby_versions, current_word);
            return;
        }
        if prev == "-R" || prev == "--rubies-dir" {
            suggest_directories(current_word, &environment.directory);
            return;
        }
        if prev == "-C" || prev == "--work-dir" {
            suggest_directories(current_word, &environment.directory);
            return;
        }
        if prev == "-G" || prev == "--gem-home" {
            suggest_directories(current_word, &environment.directory);
            return;
        }
        if prev == "-c" || prev == "--config" {
            suggest_files(current_word, &environment.directory);
            return;
        }
        if prev == "-P" || prev == "--project" {
            suggest_files(current_word, &environment.directory);
            return;
        }
        if prev == "shell-integration" {
            if "bash".starts_with(current_word) {
                println!("bash");
            }
            return;
        }
    }

    if current_word.starts_with('-') {
        print_flags(&active_command(matches));
        return;
    }

    match subcommand {
        None => print_commands(current_word),
        Some(("run", _)) => suggest_script_names(current_word, environment.project.as_ref()),
        Some(("exec", _)) => suggest_binstubs(current_word, &environment.bin_dirs),
        _ => print_nested_commands(matches, current_word),
    }
}

fn active_command(matches: &clap::ArgMatches) -> clap::Command {
    let mut command = Cli::command();
    command.build();
    let mut matches = matches;
    while let Some((name, child)) = matches.subcommand() {
        let Some(selected) = command.find_subcommand(name) else {
            break;
        };
        command = selected.clone();
        matches = child;
    }
    command
}

fn print_nested_commands(matches: &clap::ArgMatches, prefix: &str) {
    let command = Cli::command();
    let mut active = &command;
    let mut matches = matches;
    while let Some((name, child)) = matches.subcommand() {
        let Some(command) = active.find_subcommand(name) else {
            return;
        };
        active = command;
        matches = child;
    }
    print_command_candidates(active, prefix);
}

fn print_commands(prefix: &str) {
    print_command_candidates(&Cli::command(), prefix);
}

fn print_command_candidates(command: &clap::Command, prefix: &str) {
    for subcommand in command.get_subcommands() {
        if subcommand.is_hide_set() {
            continue;
        }

        let name = subcommand.get_name();
        if name.starts_with(prefix) {
            println!("{}", name);
        }

        for alias in subcommand.get_visible_aliases() {
            if alias.starts_with(prefix) {
                println!("{}", alias);
            }
        }
    }
}

fn print_flags(command: &clap::Command) {
    for arg in command.get_arguments() {
        if arg.is_positional() || arg.is_hide_set() {
            continue;
        }

        if let Some(short) = arg.get_short() {
            println!("-{}", short);
        }

        if let Some(long) = arg.get_long() {
            println!("--{}", long);
        }
    }
}

fn suggest_ruby_versions(versions: &[String], prefix: &str) {
    for version in versions {
        if version.starts_with(prefix) {
            println!("{version}");
        }
    }
}

fn suggest_script_names(prefix: &str, project: Option<&rb_core::project::ProjectRuntime>) {
    if let Some(project) = project {
        for name in project.script_names() {
            if name.starts_with(prefix) {
                println!("{name}");
            }
        }
    }
}

fn suggest_binstubs(prefix: &str, bin_dirs: &[std::path::PathBuf]) {
    use std::collections::HashSet;

    let mut suggested = HashSet::new();

    for bin_dir in bin_dirs {
        if bin_dir.exists() {
            collect_executables_from_dir(bin_dir, prefix, &mut suggested);
        }
    }

    let mut items: Vec<_> = suggested.into_iter().collect();
    items.sort();
    for item in items {
        println!("{}", item);
    }
}

fn collect_executables_from_dir(
    bin_dir: &std::path::Path,
    prefix: &str,
    collected: &mut std::collections::HashSet<String>,
) {
    if let Ok(entries) = std::fs::read_dir(bin_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file()
                && let Some(name) = entry.file_name().to_str()
                && name.starts_with(prefix)
            {
                collected.insert(name.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn completed_prefix_uses_cli_grammar() {
        let matches = super::completed_matches("rb -vB x --package-backend native tool");
        assert_eq!(matches.get_one::<bool>("no_bundler"), Some(&true));
        assert_eq!(matches.subcommand_name(), Some("exec"));
        assert!(
            matches
                .subcommand()
                .unwrap()
                .1
                .get_many::<String>("args")
                .is_none()
        );
        let matches = super::completed_matches("rb x ruby --package-backend native ");
        assert_eq!(
            matches
                .subcommand()
                .unwrap()
                .1
                .get_many::<String>("args")
                .unwrap()
                .count(),
            3
        );
        assert_eq!(super::before_cursor("rb 💎", "5"), "rb ");
    }
}
