//! A schema's value in: from a value, the binary form, text, a parsed document.

use crate::binary::Decoder;
use crate::diagnostic::{Diagnostic, Errors, Problem};
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;
use crate::schema::{Read, Reader, Resolve, Schema, Shape};
use crate::text::{self, Doc, LineCol};

/// The field names of a struct's shape (none for anything else).
#[must_use]
pub fn field_names(shape: &Shape) -> Vec<&'static str> {
    match shape {
        Shape::Struct(s) => s.fields.iter().map(|f| f.name).collect(),
        _ => Vec::new(),
    }
}

/// `A` and `B` read from the same object — each allowing the other's
/// fields, like serde's `flatten`: a record's common fields and its kind's.
pub fn read_both<A: Schema, B: Schema>(
    v: &ValueRef<'_>,
    cx: &mut Reader<'_>,
    also: &[&'static str],
) -> Option<(A, B)> {
    let (an, bn) = (field_names(&A::shape()), field_names(&B::shape()));
    let mut b_keys = bn.clone();
    b_keys.extend_from_slice(also);
    let mut a_keys = an;
    a_keys.extend_from_slice(also);
    let a = cx.with_allowed_all(&b_keys, |cx| A::read(v, cx));
    let b = cx.with_allowed_all(&a_keys, |cx| B::read(v, cx));
    Some((a?, b?))
}

/// Reads `v` as `T`: the value and its references, or every problem.
///
/// # Errors
///
/// Every problem reading it, with its path.
pub fn from_value<T: Schema>(v: &Value) -> Result<Read<T>, Vec<Diagnostic>> {
    let arena = bumpalo::Bump::new();
    read_with(&v.borrow(&arena), Reader::new())
}

/// Reads `v` as `T`, nothing else: what a runtime wants of content (the
/// references met aren't kept).
///
/// # Errors
///
/// Every problem reading it, with its path.
pub fn read_value<T: Schema>(v: &Value) -> Result<T, Vec<Diagnostic>> {
    let arena = bumpalo::Bump::new();
    read_ref_value(&v.borrow(&arena))
}

/// [`read_value`] of a borrowed value.
///
/// # Errors
///
/// Every problem reading it, with its path.
pub fn read_ref_value<T: Schema>(v: &ValueRef<'_>) -> Result<T, Vec<Diagnostic>> {
    let reader = Reader {
        value_only: true,
        ..Reader::new()
    };
    read_with(v, reader).map(|r| r.value)
}

/// Reads the binary form as `T`, borrowed all the way: no string is
/// copied but into `T` itself — content as the runtime takes it.
///
/// # Errors
///
/// The decoding error, or every problem reading it.
pub fn read_bytes<T: Schema>(bytes: &[u8]) -> Result<T, Vec<Diagnostic>> {
    // Straight, the usual case; the slow way says what's wrong.
    if let Ok(mut d) = Decoder::open(bytes)
        && let Some(v) = T::decode(&mut d)
        && d.at_end()
    {
        return Ok(v);
    }
    let arena = bumpalo::Bump::new();
    let v = crate::binary::decode_ref(bytes, &arena)
        .map_err(|e| vec![Diagnostic::error(Problem::BadBinary, e)])?;
    read_ref_value(&v)
}

/// `from_value`, references and texts going through `resolver`.
///
/// # Errors
///
/// Every problem reading it, with its path (the resolver's among them).
pub fn from_value_with<T: Schema>(
    v: &Value,
    resolver: &mut dyn Resolve,
) -> Result<Read<T>, Vec<Diagnostic>> {
    let arena = bumpalo::Bump::new();
    read_with(&v.borrow(&arena), Reader::with_resolver(resolver))
}

fn read_with<T: Schema>(v: &ValueRef<'_>, mut cx: Reader<'_>) -> Result<Read<T>, Vec<Diagnostic>> {
    let value = T::read(v, &mut cx);
    match value {
        Some(value) if cx.problems.is_empty() => Ok(Read {
            value,
            refs: cx.refs,
        }),
        _ => Err(cx.problems),
    }
}

/// Parses `text` and reads it as `T`; problems come with their line and column.
///
/// # Errors
///
/// [`Errors`]: the text's syntax error, or every problem reading it, placed.
pub fn from_text<T: Schema>(text: &str) -> Result<Read<T>, Errors> {
    let doc = text::parse(text)?;
    from_doc(text, &doc)
}

/// Parses TOML `text` and reads it as `T`.
///
/// # Errors
///
/// [`Errors`]: the text's syntax error, or every problem reading it, placed.
#[cfg(feature = "toml")]
pub fn from_toml<T: Schema>(text: &str) -> Result<Read<T>, Errors> {
    let doc = crate::toml::parse(text)?;
    from_doc(text, &doc)
}

/// Reads a parsed document as `T`; `text` is its source, for positions.
///
/// # Errors
///
/// [`Errors`]: every problem reading it, placed.
pub fn from_doc<T: Schema>(text: &str, doc: &Doc) -> Result<Read<T>, Errors> {
    locate(text, doc, from_value(&doc.value))
}

/// `from_doc`, references and texts going through `resolver`.
///
/// # Errors
///
/// [`Errors`]: every problem reading it, placed.
pub fn from_doc_with<T: Schema>(
    text: &str,
    doc: &Doc,
    resolver: &mut dyn Resolve,
) -> Result<Read<T>, Errors> {
    locate(text, doc, from_value_with(&doc.value, resolver))
}

fn locate<T>(
    text: &str,
    doc: &Doc,
    read: Result<Read<T>, Vec<Diagnostic>>,
) -> Result<Read<T>, Errors> {
    read.map_err(|problems| {
        let mut located: Vec<Diagnostic> =
            problems.into_iter().map(|p| placed(text, doc, p)).collect();
        // In file order, as an editor lists them.
        located.sort_by_key(|d| (d.line, d.col));
        Errors(located)
    })
}

/// `d` (about a value of `doc`, parsed from `text`) at its line and column:
/// its key's for one about the key (an unknown field).
pub fn placed(text: &str, doc: &Doc, d: Diagnostic) -> Diagnostic {
    let spans = doc.spans_at(&d.path);
    let span = match (d.on_key, spans.key) {
        (true, Some(key)) => key,
        _ => spans.span,
    };
    let at = LineCol::of(text, span.start as usize);
    d.at(at)
}
