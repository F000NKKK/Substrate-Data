//! Negative cases for the text form and for schemas reading it.

use substrate_data::{Diagnostic, Value, from_text, text};

use crate::case::{Case, Fault, Found};
use crate::schemas::read_text;

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = syntax();
    all.extend(schema());
    all
}

fn text_fault(name: &str, why: &str, input: &str) -> Case {
    let e = text::parse_value(input).expect_err("malformed text");
    let diagnostics = from_text::<Value>(input).expect_err("malformed text");
    let first = diagnostics.0.first().expect("a diagnostic");
    assert_eq!(
        first.code.map(|c| c.to_string()).as_deref(),
        Some("SD0001"),
        "{name}"
    );
    assert_eq!(
        (first.line, first.col),
        (Some(e.at.line), Some(e.at.col)),
        "{name}"
    );
    Case::new(name, why).fault(Fault::Text {
        input: input.to_owned(),
        line: e.at.line,
        col: e.at.col,
        message: e.message,
    })
}

const SYNTAX_FAULTS: [(&str, &str, &str); 25] = [
    ("empty", "Nothing at all is not a value.", ""),
    ("only_whitespace", "Whitespace only.", "  "),
    (
        "missing_value",
        "A comma with no value before it.",
        "[1,,2]",
    ),
    ("missing_comma", "Two values without a comma.", "[1 2]"),
    ("missing_colon", "A key without a colon.", "{\"a\" 1}"),
    ("missing_map_value", "A key with no value.", "{a:}"),
    (
        "missing_map_comma",
        "Two entries without a comma.",
        "{a:1 b:2}",
    ),
    (
        "number_key",
        "A key must be a string or an identifier.",
        "{1:2}",
    ),
    (
        "duplicate_key",
        "Keys are unique: the second `a` is the error.",
        "{a:1,a:2}",
    ),
    ("unclosed_list", "A list never closed.", "["),
    ("extra_closer", "Text after the value.", "[]]"),
    ("two_values", "Two values in one document.", "1 2"),
    ("unclosed_string", "A string never closed.", "\"abc"),
    ("single_quotes", "Strings use double quotes.", "'x'"),
    (
        "raw_newline",
        "A line break inside a string must be written `\\n`.",
        "\"a\nb\"",
    ),
    (
        "raw_tab",
        "A control character inside a string must be escaped.",
        "\"\t\"",
    ),
    (
        "unknown_escape",
        "An escape that is not defined.",
        "\"\\x\"",
    ),
    (
        "lone_surrogate",
        "A `\\u` surrogate without its pair.",
        "\"\\ud800\"",
    ),
    ("unclosed_comment", "A block comment never closed.", "/* x"),
    ("leading_zero", "Numbers do not start with 0.", "01"),
    ("plus_sign", "A number has no leading plus.", "+1"),
    ("no_digits_after_point", "A fraction needs digits.", "1."),
    ("no_exponent_digits", "An exponent needs digits.", "1e"),
    ("nan_word", "NaN and Infinity are no text values.", "NaN"),
    ("hex_number", "Hexadecimal numbers are not text.", "0x10"),
];

fn syntax() -> Vec<Case> {
    let deep = "[".repeat(257);
    let mut out: Vec<Case> = SYNTAX_FAULTS
        .iter()
        .map(|&(name, why, input)| text_fault(&format!("text_{name}"), why, input))
        .collect();
    out.push(text_fault(
        "text_line_and_column",
        "The position is a 1-based line and column (columns count characters): here line 3.",
        "{\n  name: \"x\",\n  size: ,\n}",
    ));
    out.push(text_fault("text_too_deep", "Nesting deeper than 256 levels is an error at the opening bracket that passes it (here column 257).", &deep));
    out
}

fn found(d: &Diagnostic) -> Found {
    Found {
        code: d.code.map(|c| c.to_string()).expect("a code"),
        line: d.line.expect("a line"),
        col: d.col.expect("a column"),
        path: d.path.to_string(),
        message: d.message.clone(),
    }
}

fn schema_fault(name: &str, why: &str, schema: &'static str, input: &str, codes: &[&str]) -> Case {
    let errors = read_text(schema, input).expect_err("a problem");
    let found: Vec<Found> = errors.0.iter().map(found).collect();
    let got: Vec<&str> = found.iter().map(|f| f.code.as_str()).collect();
    assert_eq!(got, codes, "{name}");
    Case::new(name, why).fault(Fault::Schema {
        schema,
        input: input.to_owned(),
        found,
    })
}

const LIMITS_OK: &str = "{\n  copies: 3,\n  owner: \"@user:7\",\n  file: \"@file:a.png\",\n}";

/// A schema fault: name, why, edits to a good document (from, to), the codes found.
type Row = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
    &'static [&'static str],
);

const SCHEMA_FAULTS: [Row; 9] = [
    (
        "schema_type_mismatch",
        "SD0002: a value of another type than its field's, at the value.",
        &[("copies: 3", "copies: \"three\"")],
        &["SD0002"],
    ),
    (
        "schema_bare_id",
        "SD0003: an id where a reference `@kind:id` goes.",
        &[("\"@user:7\"", "\"7\"")],
        &["SD0003"],
    ),
    (
        "schema_wrong_kind",
        "SD0004: a reference of another kind than the field takes.",
        &[("@file:a.png", "@doc:a.png")],
        &["SD0004"],
    ),
    (
        "schema_unknown_field",
        "SD0005: an unknown key, reported at the key (not its value).",
        &[("copies: 3,", "copies: 3,\n  colour: 1,")],
        &["SD0005"],
    ),
    (
        "schema_missing_field",
        "SD0006: a required field that is not there, reported at the object.",
        &[("  file: \"@file:a.png\",\n", "")],
        &["SD0006"],
    ),
    (
        "schema_out_of_range",
        "SD0007: a number outside its field's `range` (1..=99 here).",
        &[("copies: 3", "copies: 300")],
        &["SD0007"],
    ),
    (
        "schema_bad_typed",
        "SD0008: a string spelled as a typed value that is not one (a kind is lower case).",
        &[("@file:a.png", "@File:a.png")],
        &["SD0008"],
    ),
    (
        "schema_bad_id",
        "SD0010: a reference whose id is not its kind's id type (a user's id is a number).",
        &[("@user:7", "@user:abc")],
        &["SD0010"],
    ),
    (
        "schema_several",
        "Every problem is reported, in file order.",
        &[("copies: 3", "copies: 300"), ("@file:a.png", "@doc:a.png")],
        &["SD0007", "SD0004"],
    ),
];

fn schema() -> Vec<Case> {
    SCHEMA_FAULTS
        .iter()
        .map(|&(name, why, edits, codes)| {
            let input = edits.iter().fold(LIMITS_OK.to_owned(), |doc, (from, to)| {
                doc.replace(from, to)
            });
            schema_fault(name, why, "limits", &input, codes)
        })
        .collect()
}
