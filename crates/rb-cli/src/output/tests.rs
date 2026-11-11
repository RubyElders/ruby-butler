use super::*;
use document::{Block, Line, plain};
use serde::ser::Error;

#[test]
fn json_uses_typed_data_without_running_the_text_presenter() {
    #[derive(Serialize)]
    struct Data {
        installed: bool,
        versions: Vec<&'static str>,
        path: &'static str,
    }
    let mut bytes = Vec::new();
    Output::new(OutputFormat::Json)
        .write_to(
            "info.runtime",
            &Data {
                installed: true,
                versions: vec!["4.0.9"],
                path: "café/\"tea\"\nroom",
            },
            || panic!("JSON must not render text"),
            &mut bytes,
        )
        .unwrap();
    assert_eq!(bytes.iter().filter(|&&byte| byte == b'\n').count(), 1);
    assert!(!bytes.contains(&0x1b));
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "schema_version": 1,
            "command": "info.runtime",
            "data": {"installed":true,"versions":["4.0.9"],"path":"café/\"tea\"\nroom"}
        })
    );
}

struct Unserializable;
impl Serialize for Unserializable {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("deliberate failure"))
    }
}

#[test]
fn text_does_not_serialize_and_preserves_verbatim_bytes() {
    let mut bytes = Vec::new();
    Output::default()
        .write_to(
            "export-gemfile",
            &Unserializable,
            || Document::verbatim("source \"https://example.test\"\n\n"),
            &mut bytes,
        )
        .unwrap();
    assert_eq!(bytes, b"source \"https://example.test\"\n\n");
}

#[test]
fn serialization_failure_leaves_stdout_empty() {
    let mut bytes = Vec::new();
    let error = Output::new(OutputFormat::Json)
        .write_to("new", &Unserializable, Document::default, &mut bytes)
        .unwrap_err();
    assert!(error.to_string().contains("deliberate failure"));
    assert!(bytes.is_empty());
}

#[test]
fn both_formats_propagate_writer_failure() {
    struct Closed;
    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    for format in [OutputFormat::Text, OutputFormat::Json] {
        let error = Output::new(format)
            .write_to(
                "version",
                &true,
                || Document::verbatim("version\n"),
                &mut Closed,
            )
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }
}

#[test]
fn document_composes_fields_tables_lists_and_nested_groups() {
    let mut details = Document::default();
    details.0.push(Block::Fields {
        width: 7,
        separator: ": ",
        rows: vec![
            (plain("Ruby"), Line::from("4.0.9")),
            (plain("Backend"), Line::from("native")),
        ],
    });
    details.0.push(Block::Table {
        widths: vec![5, 0],
        separator: " | ",
        rows: vec![
            vec![plain("check"), plain("ruby -v")],
            vec![plain("run"), plain("rails s")],
        ],
    });
    let mut items = Document::default();
    items.0.push(Block::List {
        marker: "- ".into(),
        items: vec![Line::from("Install"), Line::from("Run")],
    });
    details.group(2, items);
    details.blank();
    let mut document = Document::default();
    document.heading("Environment");
    document.group(2, details);
    document.line(Line::spaced([plain("Ready").padded(7), plain("yes")]));
    document.0.push(Block::Verbatim("no extra newline".into()));
    let mut bytes = Vec::new();
    document.write(&mut bytes).unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "Environment\n  Ruby   : 4.0.9\n  Backend: native\n  check | ruby -v\n  run   | rails s\n    - Install\n    - Run\n\nReady   yes\nno extra newline"
    );
}
