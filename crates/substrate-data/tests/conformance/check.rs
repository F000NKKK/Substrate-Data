//! Checks the golden files on disk by what they say, using only the
//! crate's public API: each form of a case reads as the case's value, the
//! flags about what writes what hold, and each negative case fails the way
//! it says. This is the contract a port in another language must meet.

use std::io::Cursor;

use serde_json::Value as Json;
use substrate_data::frame::read_shaped;
use substrate_data::io::read_frame_bytes;
use substrate_data::{Bump, Value, binary, text, typed};

use crate::hex::{from_hex, to_hex};
use crate::render::{same, value_of};
use crate::schemas::{read_text, shape_of, shape_text};
use crate::store::{manifest, raw};
use crate::v1;

/// Checks every case of the manifest.
pub(crate) fn run() {
    let cases = manifest();
    assert!(!cases.is_empty(), "the manifest has no cases");
    for case in &cases {
        let name = case["name"].as_str().expect("a name");
        check_case(name, case);
    }
}

fn check_case(name: &str, case: &Json) {
    let value = case
        .get("value")
        .map(|v| value_of(v).unwrap_or_else(|| panic!("{name}: a value")));
    if let Some(t) = case.get("text").and_then(Json::as_str) {
        check_text(
            name,
            t,
            value.as_ref(),
            case["text_is_written"].as_bool() == Some(true),
        );
    }
    if case.get("binary_file").is_some() {
        check_binary(name, case, value.as_ref());
    }
    if let Some(frame) = case.get("frame") {
        check_frame(name, frame);
    }
    if let Some(io) = case.get("io") {
        check_io(name, io);
    }
    if let Some(error) = case.get("error") {
        check_error(name, error);
    }
}

fn check_text(name: &str, input: &str, value: Option<&Value>, written: bool) {
    let parsed = text::parse_value(input).unwrap_or_else(|e| panic!("{name}: text: {e}"));
    let Some(value) = value else { return };
    assert!(
        same(&parsed, &typed::flatten(value)),
        "{name}: text reads as {parsed:?}, not {value:?}"
    );
    if written {
        assert_eq!(
            text::to_string(value),
            input,
            "{name}: the text the reference writes"
        );
    }
}

/// The bytes of `section`'s `key` (inline hex when there, else its file), checked against each other.
fn bytes_of(name: &str, section: &Json, key: &str) -> Vec<u8> {
    let file = section[format!("{key}_file")].as_str().expect("a file");
    let bytes = raw(file);
    assert_eq!(
        section[format!("{key}_len")].as_u64(),
        Some(bytes.len() as u64),
        "{name}: length"
    );
    if let Some(hex) = section.get(key).and_then(Json::as_str) {
        assert_eq!(
            from_hex(hex).as_deref(),
            Some(bytes.as_slice()),
            "{name}: hex and file"
        );
    }
    bytes
}

fn check_binary(name: &str, case: &Json, value: Option<&Value>) {
    let old = case
        .get("binary_file")
        .map(|_| check_version_1(name, case, value));
    let new = case
        .get("binary_v2_file")
        .map(|_| check_version_2(name, case, value));
    if let (Some(old), Some(new)) = (old, new) {
        check_upgrade(name, case, &old, &new, value);
    }
}

/// Both decoders read `bytes` as the same value: that value.
fn decoded_both_ways(name: &str, bytes: &[u8]) -> Value {
    let decoded = binary::decode(bytes).unwrap_or_else(|e| panic!("{name}: binary: {e}"));
    let arena = Bump::new();
    let borrowed = binary::decode_ref(bytes, &arena).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(
        same(&borrowed.to_value(), &decoded),
        "{name}: decode and decode_ref agree"
    );
    decoded
}

/// The version 1 document: read, and (when the case says so) what the old writer wrote.
fn check_version_1(name: &str, case: &Json, value: Option<&Value>) -> Vec<u8> {
    let bytes = bytes_of(name, case, "binary");
    assert_eq!(bytes.get(4), Some(&1), "{name}: a version 1 document");
    let decoded = decoded_both_ways(name, &bytes);
    if let Some(value) = value {
        assert!(
            same(&decoded, value),
            "{name}: binary reads as {decoded:?}, not {value:?}"
        );
        if case["binary_is_encoded"].as_bool() == Some(true) {
            assert_eq!(
                v1::encode(value),
                bytes,
                "{name}: the bytes the version 1 writer wrote"
            );
        }
    }
    bytes
}

/// The version 2 document: read, and (when the case says so) what the reference writes.
fn check_version_2(name: &str, case: &Json, value: Option<&Value>) -> Vec<u8> {
    let bytes = bytes_of(name, case, "binary_v2");
    assert_eq!(bytes.get(4), Some(&2), "{name}: a version 2 document");
    let decoded = decoded_both_ways(name, &bytes);
    let Some(value) = value else {
        assert_eq!(
            binary::encode(&decoded),
            bytes,
            "{name}: writing what has no value"
        );
        return bytes;
    };
    assert!(
        same(&decoded, value),
        "{name}: binary_v2 reads as {decoded:?}, not {value:?}"
    );
    if case["binary_v2_is_encoded"].as_bool() == Some(true) {
        assert_eq!(
            binary::encode(value),
            bytes,
            "{name}: the bytes the reference writes"
        );
    }
    bytes
}

/// Reading version 1 and writing gives version 2 without losing the value.
fn check_upgrade(name: &str, case: &Json, old: &[u8], new: &[u8], value: Option<&Value>) {
    let upgraded = binary::upgrade(old).unwrap_or_else(|e| panic!("{name}: upgrade: {e}"));
    let read = binary::decode(&upgraded).unwrap_or_else(|e| panic!("{name}: upgraded: {e}"));
    let before = binary::decode(old).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(
        same(&read, &before),
        "{name}: the upgrade changed the value"
    );
    if let Some(value) = value {
        assert!(same(&read, value), "{name}: the upgrade reads as {read:?}");
    }
    let exact = case["binary_upgrades"].as_bool().expect("binary_upgrades");
    assert_eq!(
        upgraded == new,
        exact,
        "{name}: binary_upgrades says what upgrading writes"
    );
    if value.is_none() || case["binary_v2_is_encoded"].as_bool() == Some(true) {
        assert!(
            exact,
            "{name}: version 2 is what the value writes, so upgrading writes it"
        );
    }
}

fn check_reading(name: &str, bytes: &[u8], reading: &Json) {
    let schema = reading["schema"].as_str().expect("a schema");
    let shape = shape_of(schema).unwrap_or_else(|| panic!("{name}: no schema {schema}"));
    let want = value_of(&reading["value"]).expect("a value");
    let got =
        read_shaped(bytes, &shape).unwrap_or_else(|| panic!("{name}: {schema} reads nothing"));
    assert!(
        same(&got, &want),
        "{name}: {schema} reads {got:?}, not {want:?}"
    );
    assert_eq!(
        reading["shape"].as_str(),
        Some(shape_text(schema).as_str()),
        "{name}: shape"
    );
}

fn check_frame(name: &str, frame: &Json) {
    let mut all = vec![bytes_of(name, frame, "bytes")];
    if frame.get("bytes_v2_file").is_some() {
        all.push(bytes_of(name, frame, "bytes_v2"));
    }
    for bytes in all {
        check_reading(name, &bytes, frame);
        for also in frame
            .get("also_read_by")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
        {
            check_reading(name, &bytes, also);
        }
    }
}

fn check_io(name: &str, io: &Json) {
    let bytes = bytes_of(name, io, "bytes");
    let mut reader = Cursor::new(&bytes);
    let mut messages = Vec::new();
    while let Some(m) = read_frame_bytes(&mut reader).unwrap_or_else(|e| panic!("{name}: {e}")) {
        messages.push(to_hex(&m));
    }
    let want: Vec<&str> = io["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .map(|m| m.as_str().expect("hex"))
        .collect();
    assert_eq!(messages, want, "{name}: frames of the stream");
    assert_eq!(io["end"].as_str(), Some("clean"), "{name}: end");
}

fn check_error(name: &str, error: &Json) {
    let input = error["input"].as_str().expect("an input");
    match error["form"].as_str().expect("a form") {
        "text" => check_text_error(name, input, error),
        "text_schema" => check_schema_error(name, input, error),
        "binary" => check_binary_error(name, input, error),
        "frame" => check_frame_error(name, input, error),
        "io" => check_io_error(name, input, error),
        other => panic!("{name}: form {other}"),
    }
}

fn check_text_error(name: &str, input: &str, error: &Json) {
    let e = text::parse_value(input).expect_err(name);
    assert_eq!(error["code"].as_str(), Some("SD0001"), "{name}");
    assert_eq!(
        (u64::from(e.at.line), u64::from(e.at.col)),
        (
            error["line"].as_u64().expect("line"),
            error["col"].as_u64().expect("col")
        ),
        "{name}"
    );
    assert_eq!(
        error["message"].as_str(),
        Some(e.message.as_str()),
        "{name}"
    );
}

fn check_schema_error(name: &str, input: &str, error: &Json) {
    let schema = error["schema"].as_str().expect("a schema");
    let got = read_text(schema, input).expect_err(name).0;
    let want = error["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(got.len(), want.len(), "{name}: how many");
    for (g, w) in got.iter().zip(want) {
        assert_eq!(
            g.code.map(|c| c.to_string()).as_deref(),
            w["code"].as_str(),
            "{name}: code"
        );
        assert_eq!(
            (g.line.map(u64::from), g.col.map(u64::from)),
            (w["line"].as_u64(), w["col"].as_u64()),
            "{name}: position"
        );
        assert_eq!(
            Some(g.path.to_string().as_str()),
            w["path"].as_str(),
            "{name}: path"
        );
    }
}

fn check_binary_error(name: &str, hex: &str, error: &Json) {
    let input = from_hex(hex).expect("hex");
    let e = binary::decode(&input).expect_err(name);
    assert_eq!(
        Some(e.offset as u64),
        error["offset"].as_u64(),
        "{name}: offset"
    );
    assert_eq!(
        error["message"].as_str(),
        Some(e.message.as_str()),
        "{name}: message"
    );
    let diagnostics = substrate_data::read_bytes::<Value>(&input).expect_err(name);
    let code = diagnostics
        .first()
        .and_then(|d| d.code)
        .map(|c| c.to_string());
    assert_eq!(
        error["schema_code"].as_str(),
        code.as_deref(),
        "{name}: code"
    );
}

fn check_frame_error(name: &str, hex: &str, error: &Json) {
    let input = from_hex(hex).expect("hex");
    let schema = error["schema"].as_str().expect("a schema");
    let shape = shape_of(schema).expect("a schema");
    assert_eq!(read_shaped(&input, &shape), None, "{name}");
    assert_eq!(
        error["shape"].as_str(),
        Some(shape_text(schema).as_str()),
        "{name}: shape"
    );
}

fn check_io_error(name: &str, hex: &str, error: &Json) {
    let input = from_hex(hex).expect("hex");
    let mut reader = Cursor::new(&input);
    let mut before = 0u64;
    let kind = loop {
        match read_frame_bytes(&mut reader) {
            Ok(Some(_)) => before += 1,
            Ok(None) => panic!("{name}: ended cleanly"),
            Err(e) => break e.kind(),
        }
    };
    assert_eq!(
        error["frames_before"].as_u64(),
        Some(before),
        "{name}: frames before"
    );
    let want = match error["kind"].as_str() {
        Some("unexpected_eof") => std::io::ErrorKind::UnexpectedEof,
        Some("invalid_data") => std::io::ErrorKind::InvalidData,
        other => panic!("{name}: kind {other:?}"),
    };
    assert_eq!(kind, want, "{name}: kind");
}
