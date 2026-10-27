//! Diagnostics: built, placed, shown, coded and passed around in every form.

use substrate_data::frame::{read_frame, to_frame};
use substrate_data::{
    Atom, Code, Diagnostic, Errors, LineCol, Path, Problem, Severity, read_bytes, to_bytes,
};

#[test]
fn a_diagnostic_is_shown_with_what_is_known() {
    let base = Diagnostic::error(Problem::Invalid, "wrong");
    assert_eq!(base.to_string(), "wrong");
    assert_eq!(base.clone().at_line(4).to_string(), "line 4: wrong");
    assert_eq!(
        base.clone().at(LineCol { line: 4, col: 2 }).to_string(),
        "4:2: wrong"
    );
    assert_eq!(base.clone().in_file("a.doc").to_string(), "a.doc: wrong");
    assert_eq!(
        base.clone().in_file("a.doc").at_line(4).to_string(),
        "a.doc:4: wrong"
    );
    let placed = base
        .clone()
        .in_file("a.doc")
        .at(LineCol { line: 4, col: 2 })
        .at_path(Path::root().key("owner").index(1));
    assert_eq!(placed.to_string(), "a.doc:4:2: owner[1]: wrong");
    assert!(placed.is_error());
    assert!(!Diagnostic::warning(Problem::Invalid, "w").is_error());
    assert_eq!(
        Diagnostic::of(Severity::Note, Problem::Invalid, "n").severity,
        Severity::Note
    );
    let mut line_only = base.clone().at_line(1);
    line_only.col = None;
    line_only.file = Some(Atom::new("f"));
    line_only.line = None;
    assert_eq!(line_only.to_string(), "f: wrong");
}

#[test]
fn a_diagnostic_is_changed_by_its_builders() {
    let d = Diagnostic::from("bad").or_code(Problem::BadId);
    assert_eq!(d.code, Some(Code::Data(Problem::BadId)));
    // A code already known stays.
    assert_eq!(
        d.clone().or_code(Problem::Syntax).code,
        Some(Code::Data(Problem::BadId))
    );
    assert_eq!(d.clone().map(|m| format!("<{m}>")).message, "<bad>");
    assert_eq!(
        d.clone().by(Atom::new("rustc")).tool,
        Some(Atom::new("rustc"))
    );
    assert_eq!(String::from(Diagnostic::from(String::from("text"))), "text");
    let huge = Diagnostic::error(Problem::Invalid, "x").at_line(usize::MAX);
    assert_eq!(huge.line, None);
}

#[test]
fn codes_name_problems_and_tools() {
    assert_eq!(Problem::Syntax.to_string(), "SD0001");
    assert_eq!(Problem::of_code("SD0011"), Some(Problem::Unresolved));
    assert_eq!(Problem::of_code("E0308"), None);
    let tool = Code::tool(Atom::new("E0308"));
    assert_eq!(tool.as_str(), "E0308");
    assert_eq!(tool.to_string(), "E0308");
    assert_eq!(
        Code::from(Atom::new("SD0003")),
        Code::Data(Problem::NotAReference)
    );
    assert_eq!(Code::from(Problem::Invalid), Problem::Invalid);
    assert_ne!(tool, Code::Data(Problem::Invalid));
    assert_eq!(
        Problem::ALL
            .iter()
            .map(|p| p.code())
            .collect::<Vec<_>>()
            .len(),
        12
    );
}

#[test]
fn diagnostics_round_trip_in_every_form() {
    let d = Diagnostic::warning(Atom::new("lint_name"), "careful")
        .in_file("a.doc")
        .at(LineCol { line: 2, col: 3 })
        .at_path(Path::root().key("x"))
        .by(Atom::new("cargo"));
    assert_eq!(read_bytes::<Diagnostic>(&to_bytes(&d)).unwrap(), d);
    assert_eq!(read_frame::<Diagnostic>(&to_frame(&d)), Some(d.clone()));
    let value = substrate_data::Schema::write(&d);
    assert_eq!(substrate_data::read_value::<Diagnostic>(&value).unwrap(), d);
    assert_eq!(
        value.get("severity").and_then(|s| s.as_str()),
        Some("warning")
    );
}

#[test]
fn errors_list_one_per_line() {
    let errors = Errors(vec![
        Diagnostic::error(Problem::Invalid, "one"),
        Diagnostic::error(Problem::Invalid, "two").at_line(3),
    ]);
    assert_eq!(errors.to_string(), "one\nline 3: two");
    let as_error: &dyn std::error::Error = &errors;
    assert_eq!(as_error.to_string(), errors.to_string());
}
