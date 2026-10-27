//! A malformed input, and how a fault is written to the manifest.

use crate::hex::to_hex;
use crate::json::Json;
use crate::schemas::shape_text;

/// A malformed input and what reading it says.
#[derive(Debug, Clone)]
pub(crate) enum Fault {
    /// Text that does not parse.
    Text {
        /// The text.
        input: String,
        /// 1-based line.
        line: u32,
        /// 1-based column.
        col: u32,
        /// The reference reader's message.
        message: String,
    },
    /// Text that parses but a schema finds wrong.
    Schema {
        /// The schema's name in this suite.
        schema: &'static str,
        /// The text.
        input: String,
        /// Everything found, in file order.
        found: Vec<Found>,
    },
    /// Binary bytes that do not decode.
    Binary {
        /// The bytes.
        input: Vec<u8>,
        /// The class of error, the same in every port.
        kind: &'static str,
        /// The byte the reference reader stops at.
        offset: usize,
        /// The reference reader's message.
        message: String,
        /// The diagnostic code reading them as a schema gives.
        code: String,
    },
    /// A frame no reading accepts.
    Frame {
        /// The schema it is read as.
        schema: &'static str,
        /// The bytes.
        input: Vec<u8>,
    },
    /// A stream that ends or lies badly.
    Io {
        /// The bytes.
        input: Vec<u8>,
        /// `unexpected_eof` or `invalid_data`.
        kind: &'static str,
        /// How many frames read well before it.
        frames_before: usize,
    },
}

/// One diagnostic a schema reading gives.
#[derive(Debug, Clone)]
pub(crate) struct Found {
    /// `SD00xx`.
    pub(crate) code: String,
    /// 1-based line.
    pub(crate) line: u32,
    /// 1-based column.
    pub(crate) col: u32,
    /// The value's path in the document.
    pub(crate) path: String,
    /// The reference reader's message.
    pub(crate) message: String,
}

pub(crate) fn fault_json(f: &Fault) -> Json {
    let entries = match f {
        Fault::Text {
            input,
            line,
            col,
            message,
        } => text_fault(input, *line, *col, message),
        Fault::Schema {
            schema,
            input,
            found,
        } => schema_fault(schema, input, found),
        Fault::Binary {
            input,
            kind,
            offset,
            message,
            code,
        } => binary_fault(input, kind, *offset, message, code),
        Fault::Frame { schema, input } => vec![
            ("form", Json::text("frame")),
            ("schema", Json::text(*schema)),
            ("shape", Json::text(shape_text(schema))),
            ("input", Json::text(to_hex(input))),
        ],
        Fault::Io {
            input,
            kind,
            frames_before,
        } => vec![
            ("form", Json::text("io")),
            ("input", Json::text(to_hex(input))),
            ("frames_before", Json::num(*frames_before)),
            ("kind", Json::text(*kind)),
        ],
    };
    Json::Block(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}

type Entries = Vec<(&'static str, Json)>;

fn text_fault(input: &str, line: u32, col: u32, message: &str) -> Entries {
    vec![
        ("form", Json::text("text")),
        ("input", Json::text(input)),
        ("code", Json::text("SD0001")),
        ("line", Json::Num(u64::from(line))),
        ("col", Json::Num(u64::from(col))),
        ("message", Json::text(message)),
    ]
}

fn schema_fault(schema: &str, input: &str, found: &[Found]) -> Entries {
    vec![
        ("form", Json::text("text_schema")),
        ("schema", Json::text(schema)),
        ("input", Json::text(input)),
        (
            "diagnostics",
            Json::Arr(found.iter().map(found_json).collect()),
        ),
    ]
}

fn binary_fault(input: &[u8], kind: &str, offset: usize, message: &str, code: &str) -> Entries {
    vec![
        ("form", Json::text("binary")),
        ("input", Json::text(to_hex(input))),
        ("kind", Json::text(kind)),
        ("offset", Json::num(offset)),
        ("message", Json::text(message)),
        ("schema_code", Json::text(code)),
    ]
}

fn found_json(d: &Found) -> Json {
    Json::obj([
        ("code", Json::text(&d.code)),
        ("line", Json::Num(u64::from(d.line))),
        ("col", Json::Num(u64::from(d.col))),
        ("path", Json::text(&d.path)),
        ("message", Json::text(&d.message)),
    ])
}
