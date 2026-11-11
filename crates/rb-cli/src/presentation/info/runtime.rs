use crate::{
    commands::info::runtime::{AvailableRuby, RuntimeReport},
    output::document::{Document, Line, plain},
};
use colored::Color;
pub(crate) fn available(rubies: &[AvailableRuby]) -> Document {
    let mut doc = Document::default();
    for ruby in rubies {
        doc.line(format!(
            "{}{}",
            ruby.version,
            if ruby.installed { " (installed)" } else { "" }
        ));
    }
    doc
}
pub(crate) fn document(report: &RuntimeReport) -> Document {
    let mut doc = Document::default();
    doc.heading(plain("💎 Ruby Environment Survey").bold());
    doc.blank();
    if report.installations.is_empty() {
        return doc;
    }
    let width = 16;
    for ruby in &report.installations {
        doc.line(Line::spaced([
            plain("💎 CRuby").color(Color::Green),
            plain(format!("({})", ruby.version)).color(Color::Cyan),
        ]));
        let field = |label: &str, value| {
            Line(vec![
                plain("    "),
                plain(label).color(Color::BrightBlue).bold().padded(width),
                plain(": "),
                value,
            ])
        };
        doc.line(field(
            "Installation",
            plain(ruby.path.display()).color(Color::BrightBlack),
        ));
        doc.line(field(
            "Gem home",
            ruby.gem_home
                .as_ref()
                .map(|p| plain(p.display()).color(Color::BrightBlack))
                .unwrap_or_else(|| plain("Not available").color(Color::Yellow)),
        ));
        for (label, paths, color) in [
            ("Gem libraries", &ruby.gem_dirs, Color::Cyan),
            ("Executable paths", &ruby.bin_dirs, Color::Green),
        ] {
            if !paths.is_empty() {
                doc.line(Line(vec![
                    plain("    "),
                    plain(label).color(Color::BrightBlue).bold().padded(width),
                    plain(":"),
                ]));
                for path in paths {
                    doc.line(Line(vec![
                        plain(" ".repeat(4 + width + 2)),
                        plain(path.display()).color(color),
                    ]));
                }
            }
        }
        doc.blank();
    }
    doc.blank();
    let Some(index) = report.selected else {
        return doc;
    };
    let ruby = &report.installations[index];
    doc.line(Line(vec![
        plain(if report.requested.is_some() {
            "Environment Selected"
        } else {
            "Environment Ready"
        })
        .bold(),
        plain(": "),
        plain(if report.requested.is_some() {
            "(as requested)"
        } else {
            "(latest available)"
        })
        .color(Color::BrightBlue),
        plain(" "),
        plain(ruby.engine).color(Color::Green),
        plain(" "),
        plain(format!("({})", ruby.version)).color(Color::Cyan),
        plain(" "),
        plain(format!("residing at {}", ruby.path.display())).color(Color::BrightBlack),
    ]));
    doc.blank();
    doc.line(plain(report.requested.as_ref().map(|request| format!("Environment ready for distinguished Ruby development with version {request}.")).unwrap_or_else(|| "Environment ready for distinguished Ruby development.".into())).dimmed());
    doc
}
