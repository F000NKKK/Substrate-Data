//! A small ordered JSON tree and its writer: the manifest's keys keep the
//! order written here, so the file is the same on every run.

/// A JSON value with ordered keys.
#[derive(Debug, Clone)]
pub(crate) enum Json {
    /// `null`.
    Null,
    /// `true` / `false`.
    Bool(bool),
    /// A non-negative whole number (a line, an offset, a length).
    Num(u64),
    /// A whole number, negative or not (kept to 2^53 by its callers).
    Int(i64),
    /// A string.
    Str(String),
    /// An array, written on one line.
    Arr(Vec<Json>),
    /// An object, written on one line.
    Obj(Vec<(String, Json)>),
    /// An object, written one entry per line.
    Block(Vec<(String, Json)>),
}

impl Json {
    /// A string.
    pub(crate) fn text(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    /// A number from a length or offset.
    pub(crate) fn num(n: usize) -> Json {
        Json::Num(n as u64)
    }

    /// An object from entries.
    pub(crate) fn obj<const N: usize>(entries: [(&str, Json); N]) -> Json {
        Json::Obj(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
        )
    }
}

/// A document: the cases, one block each, a blank line between.
pub(crate) fn document(cases: &[Json]) -> String {
    let blocks: Vec<String> = cases
        .iter()
        .map(|case| {
            let mut out = String::new();
            put(case, 1, &mut out);
            format!("  {out}")
        })
        .collect();
    format!("[\n{}\n]\n", blocks.join(",\n"))
}

fn put(json: &Json, level: usize, out: &mut String) {
    match json {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Num(n) => out.push_str(&n.to_string()),
        Json::Int(n) => out.push_str(&n.to_string()),
        Json::Str(s) => substrate_data::text::write_string(s, out),
        Json::Arr(items) => put_items(items, out),
        Json::Obj(entries) => put_entries(entries, out),
        Json::Block(entries) => put_block(entries, level, out),
    }
}

fn put_items(items: &[Json], out: &mut String) {
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        put(item, 0, out);
    }
    out.push(']');
}

fn put_entries(entries: &[(String, Json)], out: &mut String) {
    out.push('{');
    for (i, (key, value)) in entries.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        substrate_data::text::write_string(key, out);
        out.push_str(": ");
        put(value, 0, out);
    }
    out.push('}');
}

fn put_block(entries: &[(String, Json)], level: usize, out: &mut String) {
    let pad = "  ".repeat(level + 1);
    out.push_str("{\n");
    for (i, (key, value)) in entries.iter().enumerate() {
        out.push_str(&pad);
        substrate_data::text::write_string(key, out);
        out.push_str(": ");
        put(value, level + 1, out);
        out.push_str(if i + 1 < entries.len() { ",\n" } else { "\n" });
    }
    out.push_str(&"  ".repeat(level));
    out.push('}');
}
