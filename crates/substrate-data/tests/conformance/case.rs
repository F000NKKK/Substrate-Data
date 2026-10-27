//! One conformance case: what it is called, the forms it carries, and how
//! it is written to the manifest and the raw files.

use substrate_data::{Value, binary, text};

use crate::fault::fault_json;
pub(crate) use crate::fault::{Fault, Found};
use crate::hex::to_hex;
use crate::json::Json;
use crate::render::value_json;
use crate::schemas::shape_text;
use crate::{store, v1};

/// Bytes longer than this are only in their file, not inline in the manifest.
const INLINE_MAX: usize = 256;

/// A frame, and every schema that reads it.
#[derive(Debug, Clone)]
pub(crate) struct Frame {
    /// The schema the frame was packed from, and what reading it by shape gives.
    pub(crate) writer: Reading,
    /// The frame's bytes: what was written before version 2 when that
    /// differs from what is written now (a tagged value inside is
    /// version 1 then), else what is written now.
    pub(crate) bytes: Vec<u8>,
    /// What the reference writes now, when it is not `bytes`.
    pub(crate) v2: Option<Vec<u8>>,
    /// Other schemas that read the same bytes (an older or newer record).
    pub(crate) also: Vec<Reading>,
}

impl Frame {
    /// A frame of the case `name`, which the reference packs as `now`. A
    /// frame file written before version 2 is kept as it was, and
    /// `now` goes beside it when it is other bytes (a tagged value inside
    /// a frame is version 2 now, version 1 then); both must read the same.
    pub(crate) fn new(name: &str, writer: Reading, now: Vec<u8>, also: Vec<Reading>) -> Frame {
        let legacy = store::legacy(&format!("frames/{name}.frame")).filter(|old| *old != now);
        Frame {
            writer,
            bytes: legacy.unwrap_or_else(|| now.clone()),
            v2: None,
            also,
        }
        .with_now(now)
    }

    fn with_now(mut self, now: Vec<u8>) -> Frame {
        if self.bytes != now {
            self.v2 = Some(now);
        }
        self
    }
}

/// A schema and the value reading a frame by its shape gives.
#[derive(Debug, Clone)]
pub(crate) struct Reading {
    /// The schema's name in this suite.
    pub(crate) schema: &'static str,
    /// What `frame::read_shaped` gives.
    pub(crate) value: Value,
}

/// A byte stream of length-prefixed frames.
#[derive(Debug, Clone)]
pub(crate) struct Io {
    /// The whole stream.
    pub(crate) bytes: Vec<u8>,
    /// The frames in it, in order (each a binary document).
    pub(crate) messages: Vec<Vec<u8>>,
}

/// A case.
#[derive(Debug, Clone)]
pub(crate) struct Case {
    /// Unique, `snake_case`: the file names derive from it.
    pub(crate) name: String,
    /// What the case shows.
    pub(crate) description: String,
    /// The text form.
    pub(crate) text: Option<String>,
    /// The expected model value.
    pub(crate) value: Option<Value>,
    /// The binary form, version 1 (`SDAT` document): read, never written now.
    pub(crate) binary: Option<Vec<u8>>,
    /// The binary form, version 2: what the reference writes.
    pub(crate) binary_v2: Option<Vec<u8>>,
    /// A packed frame.
    pub(crate) frame: Option<Frame>,
    /// A length-prefixed frame stream.
    pub(crate) io: Option<Io>,
    /// A negative case.
    pub(crate) fault: Option<Fault>,
}

impl Case {
    /// A case with nothing in it yet.
    pub(crate) fn new(name: &str, description: &str) -> Case {
        Case {
            name: name.to_owned(),
            description: description.to_owned(),
            text: None,
            value: None,
            binary: None,
            binary_v2: None,
            frame: None,
            io: None,
            fault: None,
        }
    }

    /// With text `t`.
    #[must_use]
    pub(crate) fn text(mut self, t: &str) -> Case {
        self.text = Some(t.to_owned());
        self
    }

    /// With the expected value `v`.
    #[must_use]
    pub(crate) fn value(mut self, v: Value) -> Case {
        self.value = Some(v);
        self
    }

    /// With the binary form `b`.
    #[must_use]
    pub(crate) fn binary(mut self, b: Vec<u8>) -> Case {
        self.binary = Some(b);
        self
    }

    /// With the version 1 form this case had before version 2, when its file is there.
    #[must_use]
    pub(crate) fn legacy_binary(mut self) -> Case {
        self.binary = store::legacy(&format!("binary/{}.sdat", self.name));
        self
    }

    /// With the binary form of version 2 `b`.
    #[must_use]
    pub(crate) fn binary_v2(mut self, b: Vec<u8>) -> Case {
        self.binary_v2 = Some(b);
        self
    }

    /// With the value, its canonical text and its canonical binary forms:
    /// version 1 as it was written, version 2 as it is written now.
    #[must_use]
    pub(crate) fn all_forms(self, v: Value) -> Case {
        let t = text::to_string(&v);
        let old = v1::encode(&v);
        let new = binary::encode(&v);
        self.text(&t).binary(old).binary_v2(new).value(v)
    }

    /// With frame `f`.
    #[must_use]
    pub(crate) fn frame(mut self, f: Frame) -> Case {
        self.frame = Some(f);
        self
    }

    /// With stream `i`.
    #[must_use]
    pub(crate) fn io(mut self, i: Io) -> Case {
        self.io = Some(i);
        self
    }

    /// With the fault `f`.
    #[must_use]
    pub(crate) fn fault(mut self, f: Fault) -> Case {
        self.fault = Some(f);
        self
    }

    /// The case as a manifest entry.
    pub(crate) fn to_json(&self) -> Json {
        let mut out = vec![
            ("name".to_owned(), Json::text(&self.name)),
            ("description".to_owned(), Json::text(&self.description)),
        ];
        self.put_value_forms(&mut out);
        if let Some(b) = &self.binary {
            put_bytes("binary", b, &format!("binary/{}.sdat", self.name), &mut out);
        }
        if let Some(b) = &self.binary_v2 {
            let file = format!("binary_v2/{}.sdat", self.name);
            put_bytes("binary_v2", b, &file, &mut out);
        }
        if let Some(f) = &self.frame {
            out.push(("frame".to_owned(), frame_json(f, &self.name)));
        }
        if let Some(i) = &self.io {
            out.push(("io".to_owned(), io_json(i, &self.name)));
        }
        if let Some(f) = &self.fault {
            out.push(("error".to_owned(), fault_json(f)));
        }
        Json::Block(out)
    }

    fn put_value_forms(&self, out: &mut Vec<(String, Json)>) {
        if let Some(t) = &self.text {
            out.push(("text".to_owned(), Json::text(t)));
            let writes = self
                .value
                .as_ref()
                .is_some_and(|v| text::to_string(v) == *t);
            out.push(("text_is_written".to_owned(), Json::Bool(writes)));
        }
        if let Some(v) = &self.value {
            out.push(("value".to_owned(), value_json(v)));
        }
        if let (Some(v), Some(b)) = (&self.value, &self.binary) {
            let encodes = v1::encode(v) == *b;
            out.push(("binary_is_encoded".to_owned(), Json::Bool(encodes)));
        }
        if let (Some(v), Some(b)) = (&self.value, &self.binary_v2) {
            let encodes = binary::encode(v) == *b;
            out.push(("binary_v2_is_encoded".to_owned(), Json::Bool(encodes)));
        }
        if let (Some(old), Some(new)) = (&self.binary, &self.binary_v2) {
            let exact = binary::upgrade(old).is_ok_and(|upgraded| upgraded == *new);
            out.push(("binary_upgrades".to_owned(), Json::Bool(exact)));
        }
    }

    /// The raw files this case writes: path under `conformance/`, bytes.
    pub(crate) fn files(&self) -> Vec<(String, Vec<u8>)> {
        let mut files = Vec::new();
        if let Some(b) = &self.binary {
            files.push((format!("binary/{}.sdat", self.name), b.clone()));
        }
        if let Some(b) = &self.binary_v2 {
            files.push((format!("binary_v2/{}.sdat", self.name), b.clone()));
        }
        if let Some(f) = &self.frame {
            files.push((format!("frames/{}.frame", self.name), f.bytes.clone()));
            if let Some(v2) = &f.v2 {
                files.push((format!("frames_v2/{}.frame", self.name), v2.clone()));
            }
        }
        if let Some(i) = &self.io {
            files.push((format!("io/{}.stream", self.name), i.bytes.clone()));
        }
        files
    }
}

/// `bytes` as `key` (hex, when short), `key_len` and `key_file`.
fn put_bytes(key: &str, bytes: &[u8], file: &str, out: &mut Vec<(String, Json)>) {
    if bytes.len() <= INLINE_MAX {
        out.push((key.to_owned(), Json::text(to_hex(bytes))));
    }
    out.push((format!("{key}_len"), Json::num(bytes.len())));
    out.push((format!("{key}_file"), Json::text(file)));
}

fn reading_json(r: &Reading) -> Vec<(String, Json)> {
    vec![
        ("schema".to_owned(), Json::text(r.schema)),
        ("shape".to_owned(), Json::text(shape_text(r.schema))),
        ("value".to_owned(), value_json(&r.value)),
    ]
}

fn frame_json(f: &Frame, name: &str) -> Json {
    let mut out = reading_json(&f.writer);
    put_bytes("bytes", &f.bytes, &format!("frames/{name}.frame"), &mut out);
    if let Some(v2) = &f.v2 {
        put_bytes("bytes_v2", v2, &format!("frames_v2/{name}.frame"), &mut out);
    }
    if !f.also.is_empty() {
        let also = f.also.iter().map(|r| Json::Obj(reading_json(r))).collect();
        out.push(("also_read_by".to_owned(), Json::Arr(also)));
    }
    Json::Block(out)
}

fn io_json(i: &Io, name: &str) -> Json {
    let mut out = Vec::new();
    put_bytes("bytes", &i.bytes, &format!("io/{name}.stream"), &mut out);
    let messages = i.messages.iter().map(|m| Json::text(to_hex(m))).collect();
    out.push(("messages".to_owned(), Json::Arr(messages)));
    out.push(("end".to_owned(), Json::text("clean")));
    Json::Block(out)
}
