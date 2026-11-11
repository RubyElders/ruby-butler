use crate::output::document::{Block, Document, Line};

pub(crate) fn document() -> Document {
    let mut doc = Document::default();
    doc.line("✨ Splendid! A new rbproject.toml has been created with appropriate ceremony.");
    doc.blank();
    for (heading, items) in [
        (
            "📝 This template includes:",
            vec![
                "Project metadata (name and description)",
                "A sample script (ruby-version) to demonstrate usage",
            ],
        ),
        (
            "🎯 You may now:",
            vec![
                "Edit rbproject.toml to add your own scripts",
                "Run 'rb run' to list available scripts",
                "Execute scripts with: rb run <script-name>",
            ],
        ),
    ] {
        doc.heading(heading);
        doc.group(
            3,
            Document(vec![Block::List {
                marker: "• ".into(),
                items: items.into_iter().map(Line::from).collect(),
            }]),
        );
        doc.blank();
    }
    doc.line("For comprehensive examples, please consult:");
    doc.line("   https://github.com/RubyElders/ruby-butler/blob/main/examples/rbproject.toml");
    doc
}
