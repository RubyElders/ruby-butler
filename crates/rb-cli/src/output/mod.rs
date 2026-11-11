pub(crate) mod document;
use document::Document;
use rb_core::butler::ButlerError;
use serde::Serialize;
use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Output {
    format: OutputFormat,
}

#[derive(Serialize)]
struct Report<'a, T> {
    schema_version: u32,
    command: &'static str,
    data: &'a T,
}

impl Output {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }
    pub(crate) fn emit<T: Serialize>(
        &self,
        command: &'static str,
        data: &T,
        text: impl FnOnce() -> Document,
    ) -> Result<(), ButlerError> {
        self.write_to(command, data, text, &mut io::stdout().lock())
            .map_err(|error| ButlerError::General(error.to_string()))
    }
    pub(crate) fn write_to<T: Serialize>(
        &self,
        command: &'static str,
        data: &T,
        text: impl FnOnce() -> Document,
        writer: &mut impl Write,
    ) -> io::Result<()> {
        match self.format {
            OutputFormat::Text => text().write(writer),
            OutputFormat::Json => {
                let mut bytes = serde_json::to_vec(&Report {
                    schema_version: 1,
                    command,
                    data,
                })
                .map_err(io::Error::other)?;
                bytes.push(b'\n');
                writer.write_all(&bytes)
            }
        }
    }
    pub(crate) fn partial(&self, text: impl FnOnce() -> Document) -> Result<(), ButlerError> {
        if self.format == OutputFormat::Text {
            text()
                .write(&mut io::stdout().lock())
                .map_err(|error| ButlerError::General(error.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
