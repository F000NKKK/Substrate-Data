//! [`Expand`]: a field that takes a value or a reference to one.
//!
//! A format lets a person write a text in place or name it (`@word:greeting`,
//! a word of a dictionary; `@text:intro`, a localized text). What the name
//! stands for is the tool's: it plugs a [`Resolve`](crate::schema::Resolve)
//! whose [`expand`](crate::schema::Resolve::expand) hands back the value, and
//! the field is read from that value as if it had been written there.
//!
//! ```
//! use substrate_data::with::Expand;
//! use substrate_data::schema::{Dictionary, from_text, from_doc_with};
//!
//! #[derive(substrate_data::Schema)]
//! struct Notice {
//!     #[data(with = "Expand<substrate_data::with::Plain>")]
//!     title: String,
//! }
//!
//! let words: Dictionary = from_text(r#"{ word: { hello: "Hello, world" } }"#).unwrap().value;
//!
//! let text = r#"{ title: "@word:hello" }"#;
//! let doc = substrate_data::text::parse(text).unwrap();
//! let mut words = words;
//! let notice: Notice = from_doc_with(text, &doc, &mut words).unwrap().value;
//! assert_eq!(notice.title, "Hello, world");
//!
//! // Written in place, it is read as it is.
//! let inline: Notice = from_text(r#"{ title: "Hi" }"#).unwrap().value;
//! assert_eq!(inline.title, "Hi");
//! ```

use std::marker::PhantomData;

use super::{Expansion, Reader, Shape};
use crate::binary::{Decoder, Writer};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::{Ref, Value};
use crate::with::With;

/// A field of codec `W` written in place, or as a reference the reader's
/// resolver expands into the value `W` reads. Without a resolver, or when
/// it keeps the reference, `W` reads what was written (a field that is
/// itself a reference — [`RefTo`](crate::with::RefTo) — reads it as one).
///
/// Writing, the binary form and frames carry the value, never the
/// reference: expansion is a read-time step of a tool that has a resolver
/// ([`from_value_with`](crate::schema::from_value_with),
/// [`from_doc_with`](crate::schema::from_doc_with),
/// [`from_bytes_with`](crate::schema::from_bytes_with)). A problem with the
/// expanded value is reported at the reference's path.
#[derive(Debug)]
pub struct Expand<W>(PhantomData<W>);

/// The reference `v` is, as a field takes it: a reference value, or a
/// string spelling one (`"@kind:id"`).
fn reference_of(v: &ValueRef<'_>) -> Option<Ref> {
    match v {
        ValueRef::Str(s) if s.starts_with(crate::names::TYPED) => match crate::typed::classify(s) {
            Ok(crate::typed::Spelled::Ref(r)) => Some(r),
            _ => None,
        },
        other => other.to_ref(),
    }
}

impl<F, W: With<F>> With<F> for Expand<W> {
    fn shape() -> Shape {
        W::shape()
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<F> {
        let reference = reference_of(v).filter(|_| cx.resolves());
        let Some(r) = reference else {
            return W::read(v, cx);
        };
        match cx.expand_ref(&r) {
            Expansion::Kept => W::read(v, cx),
            Expansion::Failed => {
                cx.note_ref(r);
                None
            }
            Expansion::Value(value) => {
                cx.note_ref(r);
                cx.expanded(|cx| {
                    let arena = bumpalo::Bump::new();
                    W::read(&value.borrow(&arena), cx)
                })
            }
        }
    }
    fn write(f: &F) -> Value {
        W::write(f)
    }
    fn absent() -> Option<F> {
        W::absent()
    }
    fn skip(f: &F) -> bool {
        W::skip(f)
    }
    fn write_text(f: &F, w: &mut crate::text::Writer) {
        W::write_text(f, w);
    }
    fn encode(f: &F, w: &mut Writer) {
        W::encode(f, w);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<F> {
        W::decode(d)
    }
    fn pack(f: &F, p: &mut Packer) {
        W::pack(f, p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<F> {
        W::unpack(u)
    }
}
