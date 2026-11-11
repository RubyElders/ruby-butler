use crate::{
    commands::info::config::{ConfigurationReport, Setting},
    output::document::{Document, Line, plain},
};
use colored::Color;

fn setting<T>(
    doc: &mut Document,
    label: &str,
    value: crate::output::document::Span,
    setting: &Setting<T>,
) {
    doc.line(Line::spaced([
        plain(label).color(Color::BrightWhite).bold(),
        value,
    ]));
    doc.line(
        Line::spaced([
            plain("Source:").dimmed(),
            plain(source_label(setting.source)).color(Color::Yellow),
        ])
        .indented(2),
    );
}

pub(crate) fn document(report: &ConfigurationReport) -> Document {
    let mut doc = Document::default();
    doc.heading(
        plain("🎩 Current Configuration")
            .color(Color::BrightCyan)
            .bold(),
    );
    doc.blank();
    setting(
        &mut doc,
        "Rubies Directory:",
        plain(report.rubies_dir.value.display()),
        &report.rubies_dir,
    );
    doc.blank();
    setting(
        &mut doc,
        "Ruby Version:",
        report
            .ruby_version
            .value
            .as_ref()
            .map(plain)
            .unwrap_or_else(|| plain("latest").dimmed()),
        &report.ruby_version,
    );
    if report.ruby_version.value.is_none() || report.ruby_unresolved {
        doc.line(
            Line::spaced([
                plain("Note:").dimmed(),
                plain(if report.ruby_version.value.is_none() {
                    "Will use latest available Ruby"
                } else {
                    "Will be resolved to latest available Ruby"
                })
                .color(Color::Cyan),
            ])
            .indented(2),
        );
    }
    doc.blank();
    setting(
        &mut doc,
        "Gem Home:",
        plain(report.gem_home.value.display()),
        &report.gem_home,
    );
    doc.blank();
    setting(
        &mut doc,
        "No Bundler:",
        if report.no_bundler.value {
            plain("yes").color(Color::Green)
        } else {
            plain("no").dimmed()
        },
        &report.no_bundler,
    );
    doc.blank();
    setting(
        &mut doc,
        "Working Directory:",
        plain(report.work_dir.value.display()),
        &report.work_dir,
    );
    doc.blank();
    doc.line(format!(
        "Auto-install Ruby: {}",
        report.auto_install_ruby.value
    ));
    doc.line(format!(
        "  Source: {}",
        source_label(report.auto_install_ruby.source)
    ));
    doc.blank();
    doc.line(format!("Package Backend: {}", report.package_backend.value));
    doc.line(format!(
        "  Source: {}",
        source_label(report.package_backend.source)
    ));
    doc.blank();
    doc.line(plain("Configuration sources (in priority order):").dimmed());
    for (number, label) in [
        "CLI arguments",
        "Environment variables",
        "Configuration file",
        "Built-in defaults",
    ]
    .iter()
    .enumerate()
    {
        doc.line(
            Line::spaced([plain(format!("{}.", number + 1)).dimmed(), plain(label)]).indented(2),
        );
    }
    doc
}

fn source_label(source: &str) -> &str {
    match source {
        "config_file" => "config file",
        "cli" => "CLI argument",
        "resolved" => "auto-resolved",
        other => other,
    }
}
