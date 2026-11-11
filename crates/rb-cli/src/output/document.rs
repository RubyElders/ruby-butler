use colored::{Color, ColoredString, Colorize};
use std::io::{self, Write};

#[derive(Clone, Default)]
pub(crate) struct Span {
    text: String,
    color: Option<Color>,
    bold: bool,
    dimmed: bool,
    italic: bool,
    width: usize,
}

pub(crate) fn plain(value: impl ToString) -> Span {
    Span {
        text: value.to_string(),
        ..Default::default()
    }
}

impl Span {
    pub(crate) fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    pub(crate) fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub(crate) fn dimmed(mut self) -> Self {
        self.dimmed = true;
        self
    }
    pub(crate) fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
    pub(crate) fn padded(mut self, width: usize) -> Self {
        self.width = width;
        self
    }
    fn paint(&self) -> ColoredString {
        let mut text = self.text.normal();
        if let Some(color) = self.color {
            text = text.color(color);
        }
        if self.bold {
            text = text.bold();
        }
        if self.dimmed {
            text = text.dimmed();
        }
        if self.italic {
            text = text.italic();
        }
        text
    }
}

#[derive(Clone, Default)]
pub(crate) struct Line(pub Vec<Span>);
impl From<&str> for Line {
    fn from(value: &str) -> Self {
        Self(vec![plain(value)])
    }
}
impl From<String> for Line {
    fn from(value: String) -> Self {
        Self(vec![plain(value)])
    }
}
impl From<Span> for Line {
    fn from(value: Span) -> Self {
        Self(vec![value])
    }
}
impl Line {
    pub(crate) fn indented(mut self, width: usize) -> Self {
        self.0.insert(0, plain(" ".repeat(width)));
        self
    }

    pub(crate) fn spaced(spans: impl IntoIterator<Item = Span>) -> Self {
        let mut values = Vec::new();
        for span in spans {
            if !values.is_empty() {
                values.push(plain(" "));
            }
            values.push(span);
        }
        Self(values)
    }
    fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        for span in &self.0 {
            write!(writer, "{:<width$}", span.paint(), width = span.width)?;
        }
        Ok(())
    }
}

pub(crate) enum Block {
    Heading(Line),
    Text(Line),
    Fields {
        width: usize,
        separator: &'static str,
        rows: Vec<(Span, Line)>,
    },
    Table {
        widths: Vec<usize>,
        separator: &'static str,
        rows: Vec<Vec<Span>>,
    },
    List {
        marker: String,
        items: Vec<Line>,
    },
    Verbatim(String),
    Group {
        indent: usize,
        document: Document,
    },
    BlankLine,
}

#[derive(Default)]
pub(crate) struct Document(pub Vec<Block>);
impl Document {
    pub(crate) fn line(&mut self, line: impl Into<Line>) {
        self.0.push(Block::Text(line.into()));
    }
    pub(crate) fn heading(&mut self, line: impl Into<Line>) {
        self.0.push(Block::Heading(line.into()));
    }
    pub(crate) fn blank(&mut self) {
        self.0.push(Block::BlankLine);
    }
    pub(crate) fn group(&mut self, indent: usize, document: Document) {
        self.0.push(Block::Group { indent, document });
    }
    pub(crate) fn verbatim(value: impl Into<String>) -> Self {
        Self(vec![Block::Verbatim(value.into())])
    }
    pub(crate) fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        self.write_at(writer, 0)
    }
    fn write_at(&self, writer: &mut impl Write, indent: usize) -> io::Result<()> {
        let prefix = " ".repeat(indent);
        for block in &self.0 {
            match block {
                Block::Heading(line) | Block::Text(line) => {
                    write!(writer, "{prefix}")?;
                    line.write(writer)?;
                    writeln!(writer)?;
                }
                Block::BlankLine => writeln!(writer)?,
                Block::Group {
                    indent: offset,
                    document,
                } => document.write_at(writer, indent + offset)?,
                Block::Verbatim(text) => writer.write_all(text.as_bytes())?,
                Block::Fields {
                    width,
                    separator,
                    rows,
                } => {
                    for (label, value) in rows {
                        write!(writer, "{prefix}{:<width$}{separator}", label.paint())?;
                        value.write(writer)?;
                        writeln!(writer)?;
                    }
                }
                Block::Table {
                    widths,
                    separator,
                    rows,
                } => {
                    for row in rows {
                        write!(writer, "{prefix}")?;
                        for (index, cell) in row.iter().enumerate() {
                            if index > 0 {
                                write!(writer, "{separator}")?;
                            }
                            write!(
                                writer,
                                "{:<width$}",
                                cell.paint(),
                                width = widths.get(index).copied().unwrap_or(0)
                            )?;
                        }
                        writeln!(writer)?;
                    }
                }
                Block::List { marker, items } => {
                    for item in items {
                        write!(writer, "{prefix}{marker}")?;
                        item.write(writer)?;
                        writeln!(writer)?;
                    }
                }
            }
        }
        Ok(())
    }
}
