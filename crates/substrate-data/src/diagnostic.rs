//! What a tool finds wrong — reading a document, building a mod, serving a
//! request — each as one record: what it is (a code), how bad, what's
//! wrong and where (a file, its line and column, a value's path in it).
//! The one kind of problem every tool says and every tool reads.

use std::fmt;

use crate::{Atom, LineCol, Path, Schema, SyntaxError};

/// What's wrong with a value, as this crate finds it: each a stable code
/// (`SD…`) tools can match on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Problem {
    /// Any other problem with a value.
    Invalid,
    /// Text that doesn't parse.
    Syntax,
    /// A value of another type than its field's.
    Expected,
    /// A bare id where a reference goes (`@file:…`).
    NotAReference,
    /// A reference to another kind than its field takes.
    WrongKind,
    /// A key the type doesn't have.
    UnknownField,
    /// A required field that isn't there.
    MissingField,
    /// A number outside its field's range.
    OutOfRange,
    /// A string written as a typed value that isn't one (`@File:x`).
    BadTyped,
    /// Binary data that won't decode.
    BadBinary,
    /// A reference whose id isn't of its kind's id type (`@user:abc` where
    /// a user's id is a number).
    BadId,
    /// A reference to nothing there is (a [`crate::Resolve`] says so).
    Unresolved,
}

impl Problem {
    /// Every problem, in code order.
    pub const ALL: [Problem; 12] = [
        Problem::Invalid,
        Problem::Syntax,
        Problem::Expected,
        Problem::NotAReference,
        Problem::WrongKind,
        Problem::UnknownField,
        Problem::MissingField,
        Problem::OutOfRange,
        Problem::BadTyped,
        Problem::BadBinary,
        Problem::BadId,
        Problem::Unresolved,
    ];

    /// Its stable code: `SD0000` …
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Problem::Invalid => "SD0000",
            Problem::Syntax => "SD0001",
            Problem::Expected => "SD0002",
            Problem::NotAReference => "SD0003",
            Problem::WrongKind => "SD0004",
            Problem::UnknownField => "SD0005",
            Problem::MissingField => "SD0006",
            Problem::OutOfRange => "SD0007",
            Problem::BadTyped => "SD0008",
            Problem::BadBinary => "SD0009",
            Problem::BadId => "SD0010",
            Problem::Unresolved => "SD0011",
        }
    }

    /// The problem a code names, if it's one of this crate's.
    #[must_use]
    pub fn of_code(code: &str) -> Option<Problem> {
        Problem::ALL.into_iter().find(|p| p.code() == code)
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// A diagnostic's kind: one of this crate's [`Problem`]s, or a tool's own
/// code (`E0308`, a lint's name), kept as an [`Atom`]. Written as its code
/// in every form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Code {
    /// One of this crate's problems (`SD0001`).
    Data(Problem),
    /// A tool's own code (`E0308`).
    Tool(Atom),
}

impl Code {
    /// A tool's own code; one of this crate's codes is that [`Problem`].
    pub fn tool(code: Atom) -> Code {
        Problem::of_code(code.as_str()).map_or(Code::Tool(code), Code::Data)
    }

    /// As written: `SD0001`, `E0308`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Code::Data(p) => p.code(),
            Code::Tool(a) => a.as_str(),
        }
    }

    /// As an atom: what the code is written as.
    fn atom(self) -> Atom {
        match self {
            Code::Data(p) => Atom::new(p.code()),
            Code::Tool(a) => a,
        }
    }
}

impl From<Problem> for Code {
    fn from(p: Problem) -> Code {
        Code::Data(p)
    }
}

impl From<Atom> for Code {
    fn from(a: Atom) -> Code {
        Code::tool(a)
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq<Problem> for Code {
    fn eq(&self, p: &Problem) -> bool {
        *self == Code::Data(*p)
    }
}

/// Written as its code; packed in a frame as an [`Atom`] (its number).
impl Schema for Code {
    fn shape() -> crate::Shape {
        Atom::shape()
    }
    fn read(v: &crate::ValueRef<'_>, cx: &mut crate::Reader<'_>) -> Option<Self> {
        Atom::read(v, cx).map(Code::tool)
    }
    fn write(&self) -> crate::Value {
        self.atom().write()
    }
    fn encode(&self, w: &mut crate::binary::Writer) {
        self.atom().encode(w);
    }
    fn decode(d: &mut crate::binary::Decoder<'_>) -> Option<Self> {
        Atom::decode(d).map(Code::tool)
    }
    fn pack(&self, p: &mut crate::frame::Packer) {
        self.atom().pack(p);
    }
    fn unpack(u: &mut crate::frame::Unpacker<'_>) -> Option<Self> {
        Atom::unpack(u).map(Code::tool)
    }
}

/// How bad it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Schema)]
pub enum Severity {
    /// It's wrong: the document isn't read.
    #[default]
    Error,
    /// It may be wrong.
    Warning,
    /// Something worth knowing.
    Note,
}

/// One thing found.
#[derive(Debug, Clone, PartialEq, Eq, Default, Schema)]
#[data(rename_all = "camelCase")]
pub struct Diagnostic {
    /// How bad it is.
    pub severity: Severity,
    /// What it is: one of this crate's [`Problem`]s (`SD0001`) or a
    /// tool's own code (`E0308`, a lint's name); none until it's known.
    pub code: Option<Code>,
    /// What's wrong, for a person.
    pub message: String,
    /// The file it's in.
    pub file: Option<Atom>,
    /// The value it's about, in its document (`entries[2].icon`).
    #[data(default)]
    pub path: Path,
    /// About the key at `path` rather than its value (an unknown field).
    #[data(default)]
    pub on_key: bool,
    /// 1-based.
    pub line: Option<u32>,
    /// Its 1-based column, when known.
    pub col: Option<u32>,
    /// The line it ends on, when known.
    pub end_line: Option<u32>,
    /// The column it ends at, when known.
    pub end_col: Option<u32>,
    /// Who found it, when not the tool itself (`rustc`, `cargo`).
    pub tool: Option<Atom>,
}

impl Diagnostic {
    /// An error of kind `code`.
    pub fn error(code: impl Into<Code>, message: impl fmt::Display) -> Self {
        Self::of(Severity::Error, code, message)
    }

    /// A warning of kind `code`.
    pub fn warning(code: impl Into<Code>, message: impl fmt::Display) -> Self {
        Self::of(Severity::Warning, code, message)
    }

    /// A diagnostic of `severity`, `code` and `message`, not yet placed.
    pub fn of(severity: Severity, code: impl Into<Code>, message: impl fmt::Display) -> Self {
        Diagnostic {
            severity,
            code: Some(code.into()),
            message: message.to_string(),
            ..Self::default()
        }
    }

    /// Text that doesn't parse, where it stops.
    #[must_use]
    pub fn syntax(e: SyntaxError) -> Self {
        Self::error(Problem::Syntax, e.message).at(e.at)
    }

    /// In `file`.
    #[must_use]
    pub fn in_file(mut self, file: impl fmt::Display) -> Self {
        self.file = Some(Atom::new(&file.to_string()));
        self
    }

    /// At `line` of its file.
    #[must_use]
    pub fn at_line(mut self, line: usize) -> Self {
        self.line = u32::try_from(line).ok();
        self
    }

    /// At `at` (a line and column) of its file.
    #[must_use]
    pub fn at(mut self, at: LineCol) -> Self {
        self.line = Some(at.line);
        self.col = Some(at.col);
        self
    }

    /// About the value at `path`.
    #[must_use]
    pub fn at_path(mut self, path: Path) -> Self {
        self.path = path;
        self
    }

    /// Found by `tool`.
    #[must_use]
    pub fn by(mut self, tool: Atom) -> Self {
        self.tool = Some(tool);
        self
    }

    /// Of kind `code` when its kind isn't known yet (it was met as plain text).
    #[must_use]
    pub fn or_code(mut self, code: impl Into<Code>) -> Self {
        self.code.get_or_insert(code.into());
        self
    }

    /// Its message `wrap`ped (`<Slot> {e}`), the rest kept.
    #[must_use]
    pub fn map(mut self, wrap: impl FnOnce(String) -> String) -> Self {
        self.message = wrap(self.message);
        self
    }

    /// Whether it's an error.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// As text: `file:line:col: path: message`, each part when it's known.
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.file, self.line, self.col) {
            (Some(file), Some(line), Some(col)) => write!(f, "{file}:{line}:{col}: ")?,
            (Some(file), Some(line), None) => write!(f, "{file}:{line}: ")?,
            (Some(file), None, _) => write!(f, "{file}: ")?,
            (None, Some(line), Some(col)) => write!(f, "{line}:{col}: ")?,
            (None, Some(line), None) => write!(f, "line {line}: ")?,
            (None, None, _) => {}
        }
        if !self.path.is_root() {
            write!(f, "{}: ", self.path)?;
        }
        f.write_str(&self.message)
    }
}

impl std::error::Error for Diagnostic {}

/// An error met as plain text: its kind not known yet ([`Diagnostic::or_code`]).
impl From<String> for Diagnostic {
    fn from(message: String) -> Self {
        Diagnostic {
            message,
            ..Self::default()
        }
    }
}

impl From<&str> for Diagnostic {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

/// As text, for what says errors as text.
impl From<Diagnostic> for String {
    fn from(d: Diagnostic) -> Self {
        d.to_string()
    }
}

/// Every problem found reading a document, in its order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Errors(pub Vec<Diagnostic>);

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{e}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Errors {}

impl From<SyntaxError> for Errors {
    fn from(e: SyntaxError) -> Self {
        Errors(vec![Diagnostic::syntax(e)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn said_with_what_is_known_and_a_frame_reads_back() {
        let d = Diagnostic::error(Atom::new("XY0002"), "no owner")
            .in_file("docs/intro.doc")
            .at_line(3)
            .at_path(Path::root().key("owner"));
        assert_eq!(d.to_string(), "docs/intro.doc:3: owner: no owner");
        let mut bytes = Vec::new();
        crate::io::write_frame(&mut bytes, &d).unwrap();
        let back: Diagnostic = crate::io::read_frame(&mut bytes.as_slice())
            .unwrap()
            .unwrap();
        assert_eq!(back, d);
        assert_eq!(
            Diagnostic::from("bad")
                .or_code(Atom::new("XY0004"))
                .code
                .map(Code::as_str),
            Some("XY0004")
        );
        // A crate code read back is its problem, typed.
        let missing = Diagnostic::error(Problem::MissingField, "missing `id`");
        let mut framed = Vec::new();
        crate::io::write_frame(&mut framed, &missing).unwrap();
        let read_back: Diagnostic = crate::io::read_frame(&mut framed.as_slice())
            .unwrap()
            .unwrap();
        assert_eq!(read_back.code, Some(Code::Data(Problem::MissingField)));
        assert_eq!(Code::tool(Atom::new("SD0006")), Problem::MissingField);
        assert!(
            Problem::ALL
                .iter()
                .all(|p| Problem::of_code(p.code()) == Some(*p))
        );
    }
}
