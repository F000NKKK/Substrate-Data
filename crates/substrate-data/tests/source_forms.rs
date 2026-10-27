//! Source files, JSON and TOML read into the model.

use substrate_data::source::{self, ReadError};
use substrate_data::{Value, binary, text};

#[test]
fn a_source_is_text_or_its_binary_form() {
    let bytes = source::binary("{ a: 1 }").unwrap();
    assert!(source::is_binary(&bytes));
    assert!(!source::is_binary(b"{ a: 1 }"));
    assert!(source::text(&bytes).unwrap().contains("\"a\": 1"));
    assert_eq!(source::text(b"plain").unwrap(), "plain");
    assert!(source::binary("{ a: ").is_err());
}

#[test]
fn a_source_that_is_neither_is_a_typed_error() {
    let not_utf8 = source::text(&[0xff, 0xfe, b'x']).unwrap_err();
    assert!(matches!(not_utf8, ReadError::NotUtf8(_)));
    assert!(not_utf8.to_string().starts_with("not UTF-8 text"));
    let mut broken = binary::encode(&Value::List(vec![Value::Int(1), Value::from("text")]));
    broken.truncate(broken.len() - 2);
    let err = source::text(&broken).unwrap_err();
    assert!(matches!(err, ReadError::Binary(_)));
    assert!(err.to_string().starts_with("malformed binary form"));
    let as_error: &dyn std::error::Error = &err;
    assert_ne!(as_error.to_string(), "");
}

#[cfg(feature = "json")]
#[test]
fn json_converts_both_ways() {
    use substrate_data::json::{from_json, from_json_text, to_json};
    let v =
        from_json_text(r#"{"a":[1,2.5,"x",null,true],"b":{"c":-3},"big":18446744073709551615}"#)
            .unwrap();
    assert_eq!(
        v.get("a").and_then(|a| a.as_list()).map(<[Value]>::len),
        Some(5)
    );
    assert_eq!(
        v.get("big"),
        Some(&Value::Float(18_446_744_073_709_552_000.0))
    );
    assert_eq!(from_json_text("{"), None);
    let back = to_json(&v);
    assert_eq!(from_json(&back), v);
    let typed = Value::List(vec![
        Value::Ref(substrate_data::Ref::parse("file", "x").unwrap()),
        Value::Bytes(vec![1, 2]),
        Value::Float(f64::NAN),
    ]);
    assert_eq!(
        to_json(&typed),
        serde_json::json!(["@file:x", "@bytes:AQI=", null])
    );
}

#[cfg(feature = "toml")]
#[test]
fn toml_is_read_with_its_positions() {
    use substrate_data::Path;
    use substrate_data::schema::from_toml;
    let src = "name = \"docs\"\ncount = 0x10\nbig = 99999999999999999999\nratio = 1.5\ninf = inf\non = true\nwhen = 2024-01-02\ntags = [\"a\", \"b\"]\n\n[owner]\nid = 7\n";
    let doc = substrate_data::toml::parse(src).unwrap();
    assert_eq!(doc.value.get("count"), Some(&Value::Int(16)));
    assert_eq!(doc.value.get("big").and_then(Value::as_f64), Some(1e20));
    assert_eq!(doc.value.get("ratio"), Some(&Value::Float(1.5)));
    assert_eq!(doc.value.get("when"), Some(&Value::from("2024-01-02")));
    assert_eq!(doc.value.get("on"), Some(&Value::Bool(true)));
    assert_eq!(
        doc.value.at(&Path::root().key("owner").key("id")),
        Some(&Value::Int(7))
    );
    assert_eq!(doc.locate(src, &Path::root().key("tags").index(1)).line, 8);
    assert_eq!(
        doc.locate(src, &Path::root().key("owner").key("id")).line,
        11
    );
    let err = substrate_data::toml::parse("a = ").unwrap_err();
    assert_eq!(err.at.line, 1);
    let bad = from_toml::<u8>("a = 1").unwrap_err();
    assert_ne!(bad.0, [] as [substrate_data::Diagnostic; 0]);
    assert!(text::parse("a = 1").is_err());
}
