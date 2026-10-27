//! The text syntax: what parses, where it stops when it doesn't, spans,
//! the writer's spelling, and comments kept as data.

use substrate_data::text::{
    Doc, LineCol, MAX_DEPTH, Span, parse, parse_value, to_string, to_string_pretty, with_comments,
    write_string,
};
use substrate_data::{Diagnostic, Errors, Path, Problem, Value};

fn error_of(text: &str) -> String {
    parse(text)
        .map(|_| ())
        .expect_err("text that doesn't parse")
        .to_string()
}

#[test]
fn json_with_comments_commas_and_bare_keys_parses() {
    let v = parse_value(
        "\u{feff}// head\n{ a: 1, /* mid */ \"b\": [true, false, null,], $c: -2.5e3, }",
    )
    .unwrap();
    assert_eq!(v.get("a"), Some(&Value::Int(1)));
    assert_eq!(
        v.get("b").and_then(|b| b.as_list()).map(<[Value]>::len),
        Some(3)
    );
    assert_eq!(v.get("$c"), Some(&Value::Float(-2500.0)));
    assert_eq!(
        parse_value("12345678901234567890").unwrap(),
        Value::Float(12_345_678_901_234_567_890.0)
    );
    assert_eq!(parse_value("1E2").unwrap(), Value::Float(100.0));
    assert_eq!(parse_value("-0").unwrap(), Value::Int(0));
}

#[test]
fn strings_unescape() {
    let v = parse_value(r#""a\"b\\c\/d\b\f\n\r\t\u0041\ud83d\ude00é""#).unwrap();
    assert_eq!(v, Value::from("a\"b\\c/d\u{8}\u{c}\n\r\tA\u{1F600}é"));
}

#[test]
fn syntax_errors_say_what_and_where() {
    let cases = [
        ("", "1:1: expected a value, found the end of the file"),
        ("   ", "1:4: expected a value, found the end of the file"),
        ("/* open", "1:1: unclosed `/*` comment"),
        ("[1] x", "1:5: unexpected text after the value"),
        ("'a'", "1:1: strings use double quotes"),
        ("tru", "1:1: expected a value"),
        ("\"open", "1:1: unclosed string"),
        ("\"a\nb\"", "1:3: line break in a string (write `\\n`)"),
        ("\"a\u{1}b\"", "1:3: control character in a string"),
        ("\"\\", "1:3: unclosed string, found the end of the file"),
        ("\"\\q\"", "1:2: unknown escape"),
        ("\"\\u12\"", "1:2: `\\u` needs four hex digits"),
        ("\"\\ud83d\"", "unpaired surrogate"),
        ("\"\\ud83d\\u0041\"", "unpaired surrogate"),
        ("\"\\udc00\"", "unpaired"),
        ("-", "expected digits"),
        ("01", "numbers don't start with 0"),
        ("1.", "expected digits after `.`"),
        ("1e", "expected digits in the exponent"),
        ("1e+", "expected digits in the exponent"),
        ("[1 2]", "1:4: expected `,` or `]`"),
        ("[1", "expected `,` or `]`, found the end of the file"),
        ("{a 1}", "expected `:` after the key"),
        ("{a: 1 b: 2}", "expected `,` or `}`"),
        ("{1: 2}", "expected a key"),
        ("{a: 1, a: 2}", "1:8: `a` is already set in this object"),
    ];
    for (text, want) in cases {
        let got = error_of(text);
        assert!(
            got.contains(want) || got.ends_with(want),
            "{text:?}: {got:?} vs {want:?}"
        );
    }
    let deep = "[".repeat(MAX_DEPTH + 1);
    assert!(error_of(&deep).contains("nested deeper than"));
    assert!(error_of("{\n  a: [1,\n  ?]}").starts_with("3:3:"));
}

#[test]
fn duplicate_keys_are_found_in_big_objects_too() {
    let text: String = std::iter::once("{".to_owned())
        .chain((0..40).map(|i| ["k", &i.to_string(), ": ", &i.to_string(), ", "].concat()))
        .collect();
    assert!(parse_value(&format!("{text}}}")).is_ok());
    let dup = format!("{text}k3: 0}}");
    assert!(error_of(&dup).contains("`k3` is already set"));
    let dup_after_set = format!("{text}k39: 0, k0: 1}}");
    assert!(error_of(&dup_after_set).contains("`k39` is already set"));
}

#[test]
fn a_syntax_error_becomes_a_diagnostic() {
    let e = parse("{\n a: ?}").unwrap_err();
    let d = Diagnostic::syntax(e.clone());
    assert_eq!(d.code, Some(Problem::Syntax.into()));
    assert_eq!((d.line, d.col), (Some(2), Some(5)));
    let errors: Errors = e.into();
    assert_eq!(errors.0.len(), 1);
    assert_eq!(errors.to_string(), "2:5: expected a value");
}

#[test]
fn spans_say_where_every_part_is() {
    let src = "{ a: [1, 22], bb: { c: true } }";
    let doc = parse(src).unwrap();
    let at = |p: &Path| {
        let s = doc.spans_at(p).span;
        &src[s.start as usize..s.end as usize]
    };
    assert_eq!(at(&Path::root().key("a").index(1)), "22");
    assert_eq!(at(&Path::root().key("bb").key("c")), "true");
    // Past the document: the closest parent's.
    assert_eq!(at(&Path::root().key("bb").key("nope")), "{ c: true }");
    assert_eq!(at(&Path::root().key("a").index(9)), "[1, 22]");
    assert_eq!(at(&Path::root().key("a").key("x")), "[1, 22]");
    assert_eq!(
        doc.locate(src, &Path::root().key("bb")),
        LineCol { line: 1, col: 19 }
    );
    let key = doc.spans_at(&Path::root().key("bb")).key.unwrap();
    assert_eq!(&src[key.start as usize..key.end as usize], "bb");
    assert_eq!(
        Span::new(2, 5).to(Span::new(1, 3)),
        Span { start: 2, end: 5 }
    );
    assert_eq!(Span::new(usize::MAX, usize::MAX).start, u32::MAX);
    let empty = Doc {
        value: Value::Null,
        spans: substrate_data::text::Spans::default(),
    };
    assert_eq!(empty.spans_at(&Path::root().key("x")).span, Span::default());
}

#[test]
fn line_and_column_count_characters() {
    let text = "ab\nécd";
    assert_eq!(LineCol::of(text, 0).to_string(), "1:1");
    assert_eq!(LineCol::of(text, 3).to_string(), "2:1");
    assert_eq!(LineCol::of(text, 6).to_string(), "2:3");
    assert_eq!(LineCol::of(text, 999).to_string(), "2:4");
    // Inside a multi-byte character: counted by bytes.
    assert_eq!(LineCol::of(text, 4).line, 2);
}

#[test]
fn the_writer_spells_values_so_they_read_back() {
    let v = parse_value(
        r#"{ s: "a\"\\\n\r\t\u0001é", f: [1.0, 0.1, 1e-7, 1e16], i: -3, n: null, e: {}, l: [] }"#,
    )
    .unwrap();
    for out in [to_string(&v), to_string_pretty(&v)] {
        assert_eq!(parse_value(&out).unwrap(), v, "{out}");
    }
    assert!(to_string_pretty(&v).contains('\n'));
    assert_eq!(to_string(&Value::Float(f64::NAN)), "null");
    assert_eq!(to_string(&Value::Float(f64::INFINITY)), "null");
    let mut out = String::new();
    write_string("a\u{1}\"", &mut out);
    assert_eq!(out, "\"a\\u0001\\\"\"");
    let typed = Value::Map(
        [
            (
                "r".to_owned(),
                Value::Ref(substrate_data::Ref::parse("file", "x").unwrap()),
            ),
            ("b".to_owned(), Value::Bytes(vec![1, 2])),
        ]
        .into_iter()
        .collect(),
    );
    let written = to_string(&typed);
    assert!(
        written.contains("@file:x") && written.contains("@bytes:AQI="),
        "{written}"
    );
}

#[test]
fn comments_are_kept_as_notes() {
    let src = "// File.\n{\n  // About a.\n  a: 1,\n  /* The b object.\n   * Two lines. */\n  b: { c: [\n    // lone\n    1,\n    2, // trailing\n  ] },\n  // end\n}";
    let v = with_comments(src).unwrap();
    assert_eq!(
        v.get("comment").and_then(|c| c.as_str()),
        Some("File.\nend")
    );
    assert_eq!(v.get("$a").and_then(|c| c.as_str()), Some("About a."));
    let b = v.get("b").unwrap();
    assert_eq!(
        b.get("comment").and_then(|c| c.as_str()),
        Some("The b object.\nTwo lines.\nlone")
    );
    assert!(with_comments("{ a: ?").is_err());
    assert_eq!(
        with_comments("[1, 2]").unwrap(),
        Value::List(vec![Value::Int(1), Value::Int(2)])
    );
    assert!(parse("/* x */ 1").is_ok());
}

#[test]
fn long_strings_comments_and_errors_are_exact_at_every_alignment() {
    for pad in 0..70 {
        let body = format!("{}日本 é\\n\\u00e9\\\"", "x".repeat(pad));
        let want = format!("{}日本 é\n\u{e9}\"", "x".repeat(pad));
        let doc = format!("// c\n/* b */ {{ \"k\": \"{body}\" /* tail */ }}");
        assert_eq!(
            parse_value(&doc).unwrap().get("k").and_then(Value::as_str),
            Some(want.as_str()),
            "pad {pad}"
        );

        let plain = "x".repeat(pad);
        let unclosed = format!("\"{plain}日本");
        let e1 = parse_value(&unclosed).unwrap_err();
        assert_eq!(e1.offset, 0, "pad {pad}");
        assert!(e1.message.starts_with("unclosed string"));

        let control = format!("\"{plain}日\u{1}x\"");
        let e2 = parse_value(&control).unwrap_err();
        assert_eq!(e2.offset, 1 + pad + 3, "pad {pad}");
        assert!(e2.message.starts_with("control character"));

        let newline = format!("[\n\"{plain}\n\"]");
        let e3 = parse_value(&newline).unwrap_err();
        assert_eq!(
            (e3.at.line, e3.at.col),
            (2, u32::try_from(pad + 2).unwrap())
        );
        assert!(e3.message.starts_with("line break"));

        let e4 = parse_value(&format!("[1, /* {plain} *")).unwrap_err();
        assert_eq!(e4.offset, 4, "pad {pad}");
        assert!(e4.message.starts_with("unclosed `/*` comment"));
        assert!(parse_value(&format!("1 // {plain}")).is_ok());
    }
}
