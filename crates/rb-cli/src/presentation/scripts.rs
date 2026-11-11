use crate::{
    commands::info::project::ProjectSummary,
    output::document::{Block, Document, Line, plain},
};
use colored::Color;
pub(crate) fn document(project: &ProjectSummary) -> Document {
    let mut doc = Document::default();
    doc.heading(plain("🎯 Run Project Scripts").color(Color::Green).bold());
    doc.blank();
    if let Some(name) = &project.name {
        doc.line(name.clone());
    }
    if let Some(description) = &project.description {
        doc.line(plain(description).color(Color::BrightBlack));
    }
    if project.name.is_some() || project.description.is_some() {
        doc.blank();
    }
    doc.heading(plain("Usage:").color(Color::Green).bold());
    doc.line("  rb run <SCRIPT> [ARGS]...");
    doc.blank();
    doc.heading(plain("Scripts:").color(Color::Green).bold());
    if project.scripts.is_empty() {
        doc.line(Line::from(plain("No scripts defined.").color(Color::BrightBlack)).indented(2));
        doc.blank();
        doc.line(Line(vec![
            plain("To define scripts, add them to "),
            plain(&project.config_filename).color(Color::Cyan),
            plain(":"),
        ]));
        doc.blank();
        doc.line(Line::from(plain("[scripts]").color(Color::BrightBlack)).indented(2));
        doc.line(Line(vec![
            plain("  "),
            plain("test").color(Color::Cyan),
            plain(" = "),
            plain("\"rspec\"").color(Color::BrightBlack),
        ]));
        doc.line(Line(vec![
            plain("  "),
            plain("lint").color(Color::Cyan),
            plain(" = { command = "),
            plain("\"rubocop\"").color(Color::BrightBlack),
            plain(", description = "),
            plain("\"Check code quality\"").color(Color::BrightBlack),
            plain(" }"),
        ]));
    } else {
        let width = project
            .scripts
            .keys()
            .map(|name| name.len())
            .max()
            .unwrap_or(0);
        let rows = project
            .scripts
            .iter()
            .map(|(name, script)| {
                vec![
                    plain(name).color(Color::Cyan).bold(),
                    plain(script.description.as_deref().unwrap_or(&script.command))
                        .color(Color::BrightBlack),
                ]
            })
            .collect();
        let mut table = Document::default();
        table.0.push(Block::Table {
            widths: vec![width],
            separator: "  ",
            rows,
        });
        doc.group(2, table);
        doc.blank();
        doc.heading(plain("Details:").color(Color::Green).bold());
        doc.line(Line(vec![
            plain("  "),
            plain("Project").color(Color::BrightBlack),
            plain(": "),
            plain(project.file.display()).color(Color::BrightBlack),
        ]));
        doc.line(Line(vec![
            plain("  "),
            plain("Scripts").color(Color::BrightBlack),
            plain(": "),
            plain(project.scripts.len()).color(Color::BrightBlack),
        ]));
    }
    doc
}
