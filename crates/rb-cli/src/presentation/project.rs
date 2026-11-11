use crate::{
    commands::info::project::ProjectReport,
    output::document::{Document, Line, plain},
};
use colored::Color;
pub(crate) fn document(report: &ProjectReport) -> Document {
    let mut doc = Document::default();
    doc.heading(plain("📁 Project Configuration").bold());
    doc.blank();
    if let Some(project) = &report.project {
        doc.line(
            Line::spaced([plain("Project File:").bold(), plain(project.file.display())])
                .indented(2),
        );
        doc.blank();
        if let Some(name) = &project.name {
            doc.line(
                Line::spaced([plain("Name:").bold(), plain(name).color(Color::Cyan)]).indented(2),
            );
        }
        if let Some(description) = &project.description {
            doc.line(
                Line::spaced([plain("Description:").bold(), plain(description).dimmed()])
                    .indented(2),
            );
        }
        if !project.scripts.is_empty() {
            doc.blank();
            doc.heading(Line::from(plain("Scripts:").bold()).indented(2));
            for (name, script) in &project.scripts {
                let mut line = Line::spaced([
                    plain(name).color(Color::Cyan),
                    plain("→"),
                    plain(&script.command).dimmed(),
                ]);
                if let Some(description) = &script.description {
                    line.0.push(plain(" "));
                    line.0
                        .push(plain(format!("({description})")).color(Color::BrightBlack));
                }
                doc.line(line.indented(4));
            }
        }
    } else {
        doc.line(
            Line::from(plain("No rbproject.toml found in current directory or parents").dimmed())
                .indented(2),
        );
        doc.blank();
        doc.line(
            Line::spaced([
                plain("Tip:").bold(),
                plain("Run"),
                plain("rb new").color(Color::Cyan),
                plain("to create one."),
            ])
            .indented(2),
        );
    }
    doc.blank();
    doc.heading(plain("🔧 Effective Configuration").bold());
    doc.blank();
    doc.line(
        Line::spaced([
            plain("Rubies Directory:").bold(),
            plain(report.configuration.rubies_dir.value.display()),
        ])
        .indented(2),
    );
    doc.line(
        Line::spaced([
            plain("Gem Home:").bold(),
            plain(report.configuration.gem_home.value.display()),
        ])
        .indented(2),
    );
    if let Some(requested) = &report.configuration.ruby_version.value {
        doc.line(Line::spaced([plain("Requested Ruby:").bold(), plain(requested)]).indented(2));
    }
    doc.blank();
    doc
}
