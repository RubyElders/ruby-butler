use crate::{
    commands::info::discovery::EnvironmentReport,
    output::document::{Block, Document, Line, Span, plain},
};
use colored::Color;
fn value(text: impl ToString) -> Span {
    plain(text).color(Color::BrightBlack)
}
fn field(doc: &mut Document, label: &str, content: Span, width: usize) {
    let mut row = Document::default();
    row.0.push(Block::Fields {
        width,
        separator: ": ",
        rows: vec![(plain(label).color(Color::BrightBlue).bold(), content.into())],
    });
    doc.group(4, row);
}
pub(crate) fn document(report: &EnvironmentReport) -> Document {
    let mut doc = Document::default();
    doc.heading(plain("🌍 Your Current Ruby Environment").bold());
    doc.blank();
    let Some(environment) = &report.environment else {
        field(&mut doc, "Backend", value(&report.backend), 12);
        field(&mut doc, "Ruby request", value(&report.ruby.request), 12);
        if let Some(version) = &report.ruby.version {
            field(&mut doc, "Ruby version", value(version), 12);
        }
        if let Some(path) = &report.ruby.path {
            field(&mut doc, "Installation", value(path.display()), 12);
        }
        field(
            &mut doc,
            "Installed",
            value(if report.ruby.installed { "Yes" } else { "No" }),
            12,
        );
        if let Some(project) = &report.project {
            field(&mut doc, "Project root", value(project.root.display()), 12);
        }
        if let Some(cache) = &report.cache {
            field(
                &mut doc,
                "Cache inputs",
                value(if cache.inputs.complete {
                    "Complete"
                } else {
                    "Incomplete"
                }),
                12,
            );
        }
        return doc;
    };
    let width = 16;
    let version = report.ruby.version.as_deref().unwrap_or_default();
    doc.line(Line::spaced([
        plain("💎 CRuby").color(Color::Green),
        plain(format!("({version})")).color(Color::Cyan),
    ]));
    field(
        &mut doc,
        "Installation",
        value(report.ruby.path.as_ref().unwrap().display()),
        width,
    );
    field(
        &mut doc,
        "Gem home",
        value(
            environment
                .gem_home
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Not available".into()),
        ),
        width,
    );
    if !environment.gem_dirs.is_empty() {
        field(
            &mut doc,
            "Gem libraries",
            value(
                environment
                    .gem_dirs
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            width,
        );
    }
    if !environment.bin_dirs.is_empty() {
        field(
            &mut doc,
            "Executable paths",
            value(
                environment
                    .bin_dirs
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            width,
        );
    }
    doc.blank();
    doc.heading(
        plain(format!("📦 {} Environment", environment.package_name))
            .color(Color::Green)
            .bold(),
    );
    if let Some(details) = &environment.package {
        for (label, path) in [
            ("Project root", Some(&details.root)),
            ("Gemfile", details.gemfile.as_ref()),
            ("App config", details.configuration.as_ref()),
            ("Installation", Some(&details.installation)),
        ] {
            if let Some(path) = path {
                field(&mut doc, label, value(path.display()), width);
            }
        }
        if let Some(version) = &details.required_ruby {
            field(&mut doc, "Required Ruby", value(version), width);
        }
        field(
            &mut doc,
            "Configured",
            if details.configured {
                plain("Yes").color(Color::Green)
            } else {
                plain("No").color(Color::Yellow)
            },
            width,
        );
        field(
            &mut doc,
            "Synchronized",
            match environment.synchronized {
                Some(true) => plain("✅ Synchronized").color(Color::Green),
                Some(false) => plain("⚠️  Out of sync").color(Color::Yellow),
                None => value("❓ Unknown"),
            },
            width,
        );
    } else {
        doc.line(Line::from(value("Package environment not detected")).indented(4));
    }
    doc.blank();
    if let Some(project) = &report.script_project {
        doc.heading(plain("📋 Project").color(Color::Green).bold());
        if let Some(name) = &project.name {
            field(&mut doc, "Name", value(name), width);
        }
        if let Some(description) = &project.description {
            field(&mut doc, "Description", value(description), width);
        }
        field(
            &mut doc,
            "Project file",
            value(project.file.display()),
            width,
        );
        field(
            &mut doc,
            "Scripts loaded",
            value(project.scripts.len()),
            width,
        );
        if !project.scripts.is_empty() {
            doc.blank();
            doc.heading(
                Line::from(plain("Available Scripts:").color(Color::BrightBlue).bold()).indented(4),
            );
            for (name, script) in &project.scripts {
                doc.line(Line(vec![
                    plain("      "),
                    plain(name).color(Color::Cyan).bold(),
                    plain(" "),
                    value("→"),
                    plain(" "),
                    value(&script.command),
                ]));
                if let Some(description) = &script.description {
                    doc.line(Line(vec![plain("        "), value(description).italic()]));
                }
            }
        }
    } else {
        doc.heading(value("📋 Project Scripts"));
        doc.line(Line::from(value("No project config detected")).indented(4));
    }
    doc.blank();
    doc.heading(plain("🎯 Environment Summary").color(Color::Green).bold());
    field(
        &mut doc,
        "Active Ruby",
        value(format!("CRuby {version}")),
        width,
    );
    if let Some(details) = &environment.package {
        field(
            &mut doc,
            "Package project",
            value(
                details
                    .root
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            ),
            width,
        );
        if let Some(required) = &details.required_ruby {
            field(
                &mut doc,
                "Version match",
                if version == required {
                    plain("✅ Matches").color(Color::Green)
                } else {
                    plain("⚠️  Mismatch").color(Color::Yellow)
                },
                width,
            );
        }
    }
    doc.blank();
    doc.line(value(
        "Environment ready for distinguished Ruby development.",
    ));
    doc
}
