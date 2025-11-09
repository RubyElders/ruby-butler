use crate::{Cli, resolve_search_dir};
use clap::CommandFactory;
use rb_core::ruby::RubyRuntimeDetector;
use std::path::PathBuf;

/// Defines how a command should complete its arguments
#[derive(Debug, Clone, PartialEq)]
enum CompletionBehavior {
    /// Complete the first argument with scripts from rbproject.toml, then fallback to default
    ScriptsThenDefault,
    /// Always fallback to default bash completion (files/dirs)
    AlwaysDefault,
}

/// Get completion behavior for a command
fn get_completion_behavior(command: &str) -> CompletionBehavior {
    match command {
        "run" | "r" => CompletionBehavior::ScriptsThenDefault,
        "exec" | "x" => CompletionBehavior::AlwaysDefault,
        _ => CompletionBehavior::AlwaysDefault,
    }
}

/// Generate dynamic completions based on current line and cursor position
pub fn generate_completions(line: &str, cursor_pos: &str, rubies_dir: Option<PathBuf>) {
    // Parse cursor position and truncate line at cursor
    let cursor: usize = cursor_pos.parse().unwrap_or(line.len());
    let line = &line[..cursor.min(line.len())];

    let words: Vec<&str> = line.split_whitespace().collect();

    // If empty or just "rb", suggest all commands
    if words.is_empty() || words.len() == 1 {
        print_commands("");
        return;
    }

    // Determine the current word being completed and previous word
    // If line ends with space, we're completing a new word (empty prefix)
    let (current_word, prev_word) = if line.ends_with(' ') {
        ("", words.last().copied())
    } else {
        (
            words.last().copied().unwrap_or(""),
            words.get(words.len().saturating_sub(2)).copied(),
        )
    };

    // Check if previous word was a flag that expects a value - do this first
    if let Some(prev) = prev_word {
        if prev == "-r" || prev == "--ruby" {
            // Suggest available Ruby versions filtered by prefix
            suggest_ruby_versions(rubies_dir, current_word);
            return;
        }
        if prev == "shell-integration" {
            // Suggest shell types for shell-integration command
            if "bash".starts_with(current_word) {
                println!("bash");
            }
            return;
        }
    }

    // Check if current word starts with dash (completing a flag)
    if current_word.starts_with('-') {
        print_flags();
        return;
    }

    // Find the first non-flag word after "rb" (this would be the command)
    // Skip words that are arguments to flags (come after -r, --ruby, etc.)
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
        } else if w.starts_with('-') {
            false
        } else {
            true
        }
    });
    let command = command_pos
        .and_then(|pos| words.get(pos + 1))
        .unwrap_or(&"");

    // Check if we're still completing the command name
    // We're completing the command if:
    // 1. No command found yet and we're not starting a flag
    // 2. The current word is the command and line doesn't end with space
    let completing_command =
        if command.is_empty() || (current_word == *command && !line.ends_with(' ')) {
            true
        } else {
            false
        };

    if completing_command {
        print_commands(current_word);
        return;
    }

    // Now handle completion after a complete command
    // Get the completion behavior for this command
    let behavior = get_completion_behavior(command);

    // Count how many arguments we have after the command
    // command_pos is the index in skip(1) iterator, so actual position is command_pos + 1
    let command_word_pos = command_pos.unwrap() + 1; // position of command in words array

    let args_after_command = if line.ends_with(' ') {
        // Line ends with space, we're starting a new argument
        // words after command = total words - command position - 1 (for the command itself)
        words.len() - command_word_pos - 1
    } else {
        // Current word is incomplete, count completed arguments only
        // words after command minus the incomplete current word
        words.len().saturating_sub(command_word_pos + 2)
    };

    match behavior {
        CompletionBehavior::ScriptsThenDefault => {
            // Only complete the first argument with scripts
            if args_after_command == 0 {
                suggest_script_names(current_word);
                return;
            }
            // For subsequent arguments, return nothing (fallback to default)
        }
        CompletionBehavior::AlwaysDefault => {
            // Always fallback to default completion
        }
    }

    // No completions - bash will fallback to default
}

fn print_commands(prefix: &str) {
    let cmd = Cli::command();

    for subcommand in cmd.get_subcommands() {
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
    // Try to find and parse rbproject.toml in current directory
    let current_dir = std::env::current_dir().ok();
    if let Some(dir) = current_dir {
        let project_file = dir.join("rbproject.toml");
        if project_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&project_file) {
                if let Ok(parsed) = toml::from_str::<toml::Value>(&content) {
                    if let Some(scripts) = parsed.get("scripts").and_then(|s| s.as_table()) {
                        for script_name in scripts.keys() {
                            if script_name.starts_with(prefix) {
                                println!("{}", script_name);
                            }
                        }
                    }
                }
            }
        }
    }
}
