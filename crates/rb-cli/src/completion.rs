use crate::{Cli, resolve_search_dir};
use clap::CommandFactory;
use rb_core::ruby::RubyRuntimeDetector;
use std::path::PathBuf;

pub(crate) struct CompletionEnvironment {
    pub bin_dirs: Vec<PathBuf>,
    pub project_file: Option<PathBuf>,
    pub rubies_dir: PathBuf,
}

pub(crate) fn before_cursor<'a>(line: &'a str, point: &str) -> &'a str {
    let mut end = point.parse::<usize>().unwrap_or(line.len()).min(line.len());
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    &line[..end]
}

pub(crate) fn completed_matches(line: &str) -> clap::ArgMatches {
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
    command
        .disable_help_flag(true)
        .disable_version_flag(true)
        .try_get_matches_from(words)
        .unwrap_or_else(|_| Cli::command().ignore_errors(true).get_matches_from(["rb"]))
}

fn suggest_directories(current: &str) {
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

    let Ok(entries) = std::fs::read_dir(&search_dir) else {
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

fn suggest_files(current: &str) {
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

    let Ok(entries) = std::fs::read_dir(&search_dir) else {
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
) {
    let line = before_cursor(line, cursor_pos);

    let tokens =
        shlex::split(line).unwrap_or_else(|| line.split_whitespace().map(String::from).collect());
    let words: Vec<&str> = tokens.iter().map(String::as_str).collect();

    let rubies_dir = Some(environment.rubies_dir.clone());

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

    let matches = completed_matches(line);
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
        if prev == "-r" || prev == "--ruby" {
            suggest_ruby_versions(rubies_dir, current_word);
            return;
        }
        if prev == "-R" || prev == "--rubies-dir" {
            suggest_directories(current_word);
            return;
        }
        if prev == "-C" || prev == "--work-dir" {
            suggest_directories(current_word);
            return;
        }
        if prev == "-G" || prev == "--gem-home" {
            suggest_directories(current_word);
            return;
        }
        if prev == "-c" || prev == "--config" {
            suggest_files(current_word);
            return;
        }
        if prev == "-P" || prev == "--project" {
            suggest_files(current_word);
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
        print_flags();
        return;
    }

    match subcommand {
        None => print_commands(current_word),
        Some(("run", _)) => suggest_script_names(current_word, environment.project_file.as_deref()),
        Some(("exec", _)) => suggest_binstubs(current_word, &environment.bin_dirs),
        _ => {}
    }
}

fn print_commands(prefix: &str) {
    let cmd = Cli::command();

    for subcommand in cmd.get_subcommands() {
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

fn print_flags() {
    let cmd = Cli::command();

    for arg in cmd.get_arguments() {
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

fn suggest_ruby_versions(rubies_dir: Option<PathBuf>, prefix: &str) {
    let search_dir = resolve_search_dir(rubies_dir);

    if let Ok(rubies) = RubyRuntimeDetector::discover(&search_dir) {
        for ruby in rubies {
            let version = ruby.version.to_string();
            if version.starts_with(prefix) {
                println!("{}", version);
            }
        }
    }
}

fn suggest_script_names(prefix: &str, project_file: Option<&std::path::Path>) {
    let project = if let Some(file) = project_file {
        rb_core::project::ProjectRuntime::from_file(file).ok()
    } else {
        std::env::current_dir().ok().and_then(|directory| {
            rb_core::project::RbprojectDetector::discover(&directory)
                .ok()
                .flatten()
        })
    };
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
