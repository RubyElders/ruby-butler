use crate::{
    commands::help::HelpReport,
    output::document::{Document, Line, plain},
};
use colored::Color;
fn command_line(doc: &mut Document, command: &clap::Command, prefix: &str) {
    let name = format!("{prefix}{}", command.get_name());
    let about = command
        .get_about()
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut line = Line(vec![
        plain("  "),
        plain(name).color(Color::Cyan).bold().padded(18),
        plain(" "),
        plain(about),
    ]);
    let aliases: Vec<_> = command.get_all_aliases().collect();
    if !aliases.is_empty() {
        line.0.push(plain(" "));
        line.0
            .push(plain(format!("[aliases: {}]", aliases.join(", "))).color(Color::Cyan));
    }
    doc.line(line);
}
pub(crate) fn document(report: &HelpReport) -> Document {
    if !report.custom {
        let mut command = report.command.clone();
        let help = command.render_help();
        let text = if colored::control::SHOULD_COLORIZE.should_colorize() {
            help.ansi().to_string()
        } else {
            help.to_string()
        };
        return Document::verbatim(format!("{text}\n"));
    }
    let command = &report.command;
    let mut doc = Document::default();
    if let Some(about) = command.get_about() {
        doc.line(about.to_string());
    }
    doc.blank();
    doc.line(Line::spaced([
        plain("Usage:").color(Color::Green).bold(),
        plain(command.get_name()).color(Color::Cyan).bold(),
        plain("[OPTIONS]").color(Color::Cyan),
        plain("COMMAND").color(Color::Cyan).bold(),
        plain("[COMMAND_OPTIONS]").color(Color::Cyan),
    ]));
    doc.blank();
    for (heading, names) in [
        ("Commands:", &["run", "exec", "sync"][..]),
        ("Diagnostic Commands:", &["info"][..]),
        (
            "Utility Commands:",
            &[
                "install",
                "new",
                "export-gemfile",
                "version",
                "help",
                "shell-integration",
            ][..],
        ),
    ] {
        doc.heading(plain(heading).color(Color::Green).bold());
        for subcommand in command
            .get_subcommands()
            .filter(|cmd| names.contains(&cmd.get_name()))
        {
            if subcommand.get_name() == "info" {
                for info in subcommand
                    .get_subcommands()
                    .filter(|cmd| cmd.get_name() != "help")
                {
                    command_line(&mut doc, info, "info ");
                }
            } else {
                command_line(&mut doc, subcommand, "");
            }
        }
        doc.blank();
    }
    doc.heading(plain("Options:").color(Color::Green).bold());
    for arg in command
        .get_arguments()
        .filter(|arg| !matches!(arg.get_id().as_str(), "help" | "version"))
    {
        let short = arg.get_short().map(|v| format!("-{v}")).unwrap_or_default();
        let long = arg.get_long().map(|v| format!("--{v}")).unwrap_or_default();
        let flag = if !short.is_empty() && !long.is_empty() {
            format!("{short}, {long}")
        } else {
            format!("{short}{long}")
        };
        let value = if arg.get_num_args().unwrap_or_default().takes_values()
            && arg.get_action().takes_values()
        {
            format!(
                " <{}>",
                arg.get_id().as_str().to_uppercase().replace('_', "-")
            )
        } else {
            String::new()
        };
        let visual_width = flag.len() + value.len();
        let value = if value.is_empty() {
            plain(value)
        } else {
            plain(value).color(Color::Cyan)
        };
        let padding = if visual_width < 31 {
            31 - visual_width
        } else {
            1
        };
        let mut line = Line(vec![
            plain("  "),
            plain(flag).color(Color::Cyan).bold(),
            value,
            plain(" ".repeat(padding)),
            plain(arg.get_help().map(ToString::to_string).unwrap_or_default()),
        ]);
        if let Some(env) = arg.get_env() {
            line.0.push(plain(" "));
            line.0
                .push(plain(format!(" [env: {}]", env.to_string_lossy())).color(Color::Cyan));
        }
        doc.line(line);
    }
    doc
}
