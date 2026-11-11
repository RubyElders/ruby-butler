use crate::{
    commands::shell_integration::IntegrationReport,
    output::document::{Document, Line, plain},
};
use colored::Color;
pub(crate) fn document(integrations: &[IntegrationReport]) -> Document {
    let mut doc = Document::default();
    doc.heading(plain("🎩 Available Shell Integrations").bold());
    doc.blank();
    doc.heading(plain("Shells:").bold());
    for item in integrations {
        doc.line(Line(vec![
            plain("  "),
            plain(item.shell).color(Color::Green).padded(12),
            plain(" "),
            plain(item.description),
        ]));
    }
    doc.blank();
    doc.heading(plain("Installation:").bold());
    for item in integrations {
        doc.line(Line(vec![
            plain("  "),
            plain(item.shell).color(Color::Green).padded(12),
            plain(" "),
            plain(item.instruction),
        ]));
    }
    doc
}
