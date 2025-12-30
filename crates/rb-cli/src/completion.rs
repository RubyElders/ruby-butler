use crate::{Cli, resolve_search_dir};
use clap::CommandFactory;
use rb_core::ruby::RubyRuntimeDetector;
use std::path::PathBuf;

/// Defines how a command should complete its arguments
#[derive(Debug, Clone, PartialEq)]
enum CompletionBehavior {
    /// Complete the first argument with scripts from rbproject.toml, then fallback to default
    Scripts,
    /// Complete the first argument with binstubs from bundler, then fallback to default
    Binstubs,
    /// Always fallback to default bash completion (files/dirs)
    DefaultOnly,
}

/// Get completion behavior for a command
fn get_completion_behavior(command: &str) -> CompletionBehavior {
    match command {
        "run" | "r" => CompletionBehavior::Scripts,
        "exec" | "x" => CompletionBehavior::Binstubs,
        _ => CompletionBehavior::DefaultOnly,
    }
}

/// Expand tilde (~) to home directory in paths
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            let mut expanded = PathBuf::from(home);
            expanded.push(stripped);
            return expanded;
        }
    } else if path == "~"
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home);
    }
    PathBuf::from(path)
}

/// Extract rubies_dir from command line words if -R or --rubies-dir flag is present
fn extract_rubies_dir_from_line(words: &[&str]) -> Option<PathBuf> {
    for i in 0..words.len() {
        if (words[i] == "-R" || words[i] == "--rubies-dir") && i + 1 < words.len() {
            return Some(expand_tilde(words[i + 1]));
        }
    }
    None
}

/// Suggest directories for completion
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

/// Suggest files and directories for completion  
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
pub fn generate_completions(
    line: &str,
    cursor_pos: &str,
    butler_runtime: Option<&rb_core::butler::ButlerRuntime>,
) {
    let cursor: usize = cursor_pos.parse().unwrap_or(line.len());
    let line = &line[..cursor.min(line.len())];

    let words: Vec<&str> = line.split_whitespace().collect();

    let rubies_dir = None; // Not needed - ButlerRuntime already configured

    let rubies_dir = extract_rubies_dir_from_line(&words).or(rubies_dir);

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

    let value_taking_flags = [
        "-r",
        "--ruby",
        "-R",
        "--rubies-dir",
        "-c",
        "--config",
        "-P",
        "--project",
        "-G",
        "--gem-home",
        "--log-level",
    ];
    let mut skip_next = false;
    let command_pos = words.iter().skip(1).position(|w| {
        if skip_next {
            skip_next = false;
            false
        } else if value_taking_flags.contains(w) {
            skip_next = true;
            false
        } else {
            !w.starts_with('-')
        }
    });
    let command = command_pos
        .and_then(|pos| words.get(pos + 1))
        .unwrap_or(&"");

    let completing_command =
        command.is_empty() || (current_word == *command && !line.ends_with(' '));

    if completing_command {
        print_commands(current_word);
        return;
    }

    let behavior = get_completion_behavior(command);

    let command_word_pos = command_pos.unwrap() + 1;

    let args_after_command = if line.ends_with(' ') {
        words.len() - command_word_pos - 1
    } else {
        words.len().saturating_sub(command_word_pos + 2)
    };

    match behavior {
        CompletionBehavior::Scripts => {
            if args_after_command == 0 {
                suggest_script_names(current_word);
            }
        }
        CompletionBehavior::Binstubs => {
            if args_after_command == 0
                && let Some(runtime) = butler_runtime
            {
                suggest_binstubs(current_word, runtime);
            }
            // If no runtime available, just don't suggest binstubs (no Ruby found)
        }
        CompletionBehavior::DefaultOnly => {}
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

        // Also include visible aliases
        for alias in subcommand.get_visible_aliases() {
            if alias.starts_with(prefix) {
                println!("{}", alias);
            }
        }
    }
}

fn print_flags() {
    let cmd = Cli::command();

    // Get all global flags from the root command
    for arg in cmd.get_arguments() {
        // Skip positional arguments and hidden flags
        if arg.is_positional() || arg.is_hide_set() {
            continue;
        }

        // Print short flag if available
        if let Some(short) = arg.get_short() {
            println!("-{}", short);
        }

        // Print long flag if available
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

fn suggest_script_names(prefix: &str) {
    let current_dir = std::env::current_dir().ok();
    if let Some(dir) = current_dir {
        let project_file = dir.join("rbproject.toml");
        if project_file.exists()
            && let Ok(content) = std::fs::read_to_string(&project_file)
            && let Ok(parsed) = toml::from_str::<toml::Value>(&content)
            && let Some(scripts) = parsed.get("scripts").and_then(|s| s.as_table())
        {
            for script_name in scripts.keys() {
                if script_name.starts_with(prefix) {
                    println!("{}", script_name);
                }
            }
        }
    }
}

fn suggest_binstubs(prefix: &str, butler_runtime: &rb_core::butler::ButlerRuntime) {
    use std::collections::HashSet;

    let mut suggested = HashSet::new();

    for bin_dir in butler_runtime.bin_dirs() {
        if bin_dir.exists() {
            collect_executables_from_dir(&bin_dir, prefix, &mut suggested);
        }
    }

    let mut items: Vec<_> = suggested.into_iter().collect();
    items.sort();
    for item in items {
        println!("{}", item);
    }
}

/// Helper function to collect executables from a directory into a HashSet
fn collect_executables_from_dir(
    bin_dir: &std::path::Path,
    prefix: &str,
    collected: &mut std::collections::HashSet<String>,
) {
    if let Ok(entries) = std::fs::read_dir(bin_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type()
                && file_type.is_file()
                && let Some(name) = entry.file_name().to_str()
                && name.starts_with(prefix)
            {
                collected.insert(name.to_string());
            }
        }
    }
}
