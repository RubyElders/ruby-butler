use std::time::Duration;
use terminal_size::{Height, Width, terminal_size};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(crate) fn activity_icon(elapsed: Duration) -> char {
    const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    FRAMES[((elapsed.as_millis() / 250) % FRAMES.len() as u128) as usize]
}

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds < 10 {
        return format!("{:.1}s", duration.as_secs_f64());
    }
    if seconds < 60 {
        return format!("{}s", seconds);
    }
    format!("{}m{:02}s", seconds / 60, seconds % 60)
}

pub(crate) fn fit_terminal(text: &str) -> String {
    let width = terminal_size()
        .map(|(Width(width), Height(_))| usize::from(width))
        .or_else(|| {
            std::env::var("COLUMNS")
                .ok()
                .and_then(|value| value.parse().ok())
        })
        .unwrap_or(80)
        .saturating_sub(1);
    fit_width(text, width)
}

fn fit_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let text = plain_text(text);
    if UnicodeWidthStr::width(text.as_str()) <= width {
        return text;
    }
    let mut result = String::new();
    let mut used = 0;
    for character in text.chars() {
        let columns = character.width().unwrap_or(0);
        if used + columns >= width {
            break;
        }
        used += columns;
        result.push(character);
    }
    result.push('…');
    result
}

pub(crate) fn plain_text(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\x1b' => match chars.next() {
                Some('[') => {
                    for character in chars.by_ref() {
                        if ('@'..='~').contains(&character) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(character) = chars.next() {
                        if character == '\x07' {
                            break;
                        }
                        if character == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\n' | '\r' => result.push(' '),
            '\t' => result.push_str("    "),
            character if !character.is_control() => result.push(character),
            _ => {}
        }
    }
    result
}

pub(crate) fn viewport_height() -> usize {
    terminal_size()
        .map(|(_, Height(height))| usize::from(height))
        .or_else(|| {
            std::env::var("LINES")
                .ok()
                .and_then(|value| value.parse().ok())
        })
        .unwrap_or(24)
        .saturating_sub(1)
        .max(1)
}

pub(crate) fn limit_tree_viewport(
    header: String,
    mut body: Vec<String>,
    footer: String,
    limit: usize,
) -> Vec<String> {
    let limit = limit.max(2);
    let body_limit = limit.saturating_sub(2);
    if body.len() > body_limit {
        if body_limit == 0 {
            body.clear();
        } else if body_limit == 1 {
            body = vec!["│  · working…".into()];
        } else {
            let start = body.len().saturating_sub(body_limit);
            body.drain(..start);
        }
    }
    let mut lines = Vec::with_capacity(body.len() + 2);
    lines.push(header);
    lines.extend(body);
    lines.push(footer);
    lines
}

pub(crate) fn clear_sequence(lines: usize) -> String {
    if lines == 0 {
        return String::new();
    }
    let mut sequence = format!("\x1b[{}A\r", lines.saturating_sub(1));
    for index in 0..lines {
        sequence.push_str("\x1b[2K");
        if index + 1 < lines {
            sequence.push('\n');
        }
    }
    sequence.push('\r');
    if lines > 1 {
        sequence.push_str(&format!("\x1b[{}A\r", lines - 1));
    }
    sequence
}

#[cfg(test)]
mod tests;
