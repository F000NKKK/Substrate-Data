//! A document of one `Stream` field, forged byte by byte.

use substrate_data::binary::tag;
use substrate_data::{Schema, Stream, to_bytes};

/// A stream of small ints.
#[derive(Schema, Debug, PartialEq)]
pub(crate) struct Narrow {
    pub(crate) items: Stream<u8>,
}

/// A stream of strings.
#[derive(Schema, Debug, PartialEq)]
pub(crate) struct Words {
    pub(crate) items: Stream<String>,
}

fn varint(out: &mut Vec<u8>, mut n: u64) {
    while n >= 0x80 {
        out.push(u8::try_from(n & 0x7f).unwrap_or(0) | 0x80);
        n >>= 7;
    }
    out.push(u8::try_from(n).unwrap_or(0));
}

/// A document of one `Stream` field, whose stream claims `count` items and
/// holds `items` as its bytes, whatever they are.
pub(crate) fn forged(count: u64, items: &[u8]) -> Vec<u8> {
    let empty = to_bytes(&Narrow {
        items: Stream::default(),
    });
    // The stream is the last thing in the document: its tag, a `u32` length,
    // and its count (0, one byte).
    let mut doc = empty[..empty.len() - 6].to_vec();
    let mut body = Vec::new();
    varint(&mut body, count);
    body.extend_from_slice(items);
    doc.push(tag::STREAM);
    doc.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
    doc.extend_from_slice(&body);
    doc
}
