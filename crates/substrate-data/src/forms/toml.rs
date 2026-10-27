//! TOML read into the model (feature `toml`) — for manifests that keep
//! the syntax people know from Cargo but get the same
//! schemas, checks and references as every other format.
//!
//! Tables are maps in file order, arrays lists; a datetime becomes its
//! string. Every value keeps its span, so schema problems point at their
//! line just as in the JSON-shaped formats.

use serde_spanned::Spanned;
use toml::de::{DeTable, DeValue};

use crate::model::value::{Map, Value};
use crate::text::{Doc, LineCol, Span, Spans, SyntaxError};

/// Parses a TOML document.
///
/// # Errors
///
/// [`SyntaxError`] when the text isn't TOML.
pub fn parse(text: &str) -> Result<Doc, SyntaxError> {
    let table = DeTable::parse(text).map_err(|e| {
        let offset = e.span().map_or(0, |s| s.start);
        SyntaxError {
            message: e.message().trim().to_string(),
            at: LineCol::of(text, offset),
            offset,
        }
    })?;
    let span = to_span(table.span());
    let (value, children) = table_value(table.into_inner());
    Ok(Doc {
        value,
        spans: Spans {
            span,
            key: None,
            children,
        },
    })
}

fn to_span(r: std::ops::Range<usize>) -> Span {
    Span::new(r.start, r.end)
}

fn table_value(table: DeTable<'_>) -> (Value, Vec<Spans>) {
    let mut map = Map::with_capacity(table.len());
    let mut spans = Vec::with_capacity(table.len());
    for (k, v) in table {
        let key = to_span(k.span());
        let (value, mut s) = convert(v);
        s.key = Some(key);
        map.push_unchecked(k.into_inner().as_ref().into(), value);
        spans.push(s);
    }
    (Value::Map(map), spans)
}

fn convert(v: Spanned<DeValue<'_>>) -> (Value, Spans) {
    let span = to_span(v.span());
    let (value, children) = match v.into_inner() {
        DeValue::String(s) => (Value::Str(s.into_owned()), Vec::new()),
        DeValue::Integer(n) => {
            let value = i64::from_str_radix(n.as_str(), n.radix()).map_or_else(
                |_| Value::Float(n.as_str().parse().unwrap_or(f64::NAN)),
                Value::Int,
            );
            (value, Vec::new())
        }
        DeValue::Float(f) => (Value::Float(parse_float(f.as_str())), Vec::new()),
        DeValue::Boolean(b) => (Value::Bool(b), Vec::new()),
        DeValue::Datetime(d) => (Value::Str(d.to_string()), Vec::new()),
        DeValue::Array(items) => {
            let mut list = Vec::with_capacity(items.len());
            let mut spans = Vec::with_capacity(items.len());
            for item in items {
                let (item_value, item_spans) = convert(item);
                list.push(item_value);
                spans.push(item_spans);
            }
            (Value::List(list), spans)
        }
        DeValue::Table(t) => table_value(t),
    };
    (
        value,
        Spans {
            span,
            key: None,
            children,
        },
    )
}

fn parse_float(s: &str) -> f64 {
    match s.trim_start_matches('+') {
        "inf" => f64::INFINITY,
        "-inf" => f64::NEG_INFINITY,
        "nan" | "-nan" => f64::NAN,
        other => other.replace('_', "").parse().unwrap_or(f64::NAN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Path;

    const MANIFEST: &str = r#"[package]
id = "app"
version = "0.1.0"
depends = ["core", "physics"]

[run."profile-1.2.3"]
memory = 0x800
scale = 1.5
released = 1979-05-27

[external.tools]
items = ["tools:wrench"]
"#;

    #[test]
    fn reads_tables_in_order_with_positions() {
        let doc = parse(MANIFEST).unwrap();
        let v = &doc.value;
        assert_eq!(
            v.as_map().unwrap().keys().collect::<Vec<_>>(),
            ["package", "run", "external"]
        );
        assert_eq!(
            v.get("package")
                .unwrap()
                .get("depends")
                .unwrap()
                .as_list()
                .unwrap()
                .len(),
            2
        );
        let run = v.get("run").unwrap().get("profile-1.2.3").unwrap();
        assert_eq!(run.get("memory"), Some(&Value::Int(2048)));
        assert_eq!(run.get("scale"), Some(&Value::Float(1.5)));
        assert_eq!(run.get("released"), Some(&Value::Str("1979-05-27".into())));
        let scale = doc.locate(
            MANIFEST,
            &Path::root().key("run").key("profile-1.2.3").key("scale"),
        );
        assert_eq!(scale, LineCol { line: 8, col: 9 });
        let item = doc.locate(
            MANIFEST,
            &Path::root()
                .key("external")
                .key("tools")
                .key("items")
                .index(0),
        );
        assert_eq!(item, LineCol { line: 12, col: 10 });
    }

    #[test]
    fn syntax_errors_say_where() {
        let e = parse("[package]\nid = \"a\"\nid = \"x\"\n").unwrap_err();
        assert_eq!(e.at.line, 3, "{e}");
        let unclosed = parse("[package\nid = 1\n").unwrap_err();
        assert_eq!(unclosed.at.line, 1, "{unclosed}");
    }
}
