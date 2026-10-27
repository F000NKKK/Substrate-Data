//! [`Schema`]: a format, a type that reads, writes, encodes, packs and describes itself.

use super::{Reader, Shape, read_ref_value};
use crate::binary::{Decoder, Writer};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;

/// A format: a type that reads, writes, encodes, packs and describes itself.
pub trait Schema: Sized {
    /// What the format looks like.
    fn shape() -> Shape;

    /// Reads `v`. Problems go to `cx` at its current path; `None` when the
    /// value couldn't be read (the problem is in `cx`).
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self>;

    /// The value, as the format writes it.
    fn write(&self) -> Value;

    /// The value of a field that isn't there — `None`: the field is required.
    #[must_use]
    fn absent() -> Option<Self> {
        None
    }

    /// Whether a struct leaves this field out when writing it.
    fn skip(&self) -> bool {
        false
    }

    /// Writes itself in the binary form, straight — [`Schema::write`]'s
    /// value encoded, unless the type knows better (derived ones do).
    fn encode(&self, w: &mut Writer) {
        w.value(&self.write());
    }

    /// Reads itself from the binary form, straight; `None` when it isn't
    /// what [`Schema::encode`] writes — then [`read_bytes`](crate::schema::read_bytes) reads the slow
    /// way, which says what's wrong. Through a value unless the type knows
    /// better (derived ones do).
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        let arena = bumpalo::Bump::new();
        let v = d.value_ref(&arena).ok()?;
        read_ref_value(&v).ok()
    }

    /// Packs itself into a frame ([`crate::frame`]): its binary form as
    /// bytes, unless the type packs itself (derived ones and plain types do).
    fn pack(&self, p: &mut Packer) {
        p.put_tagged(self);
    }

    /// Reads itself out of a frame; `None` if the bytes aren't what
    /// [`Schema::pack`] writes.
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        u.tagged()
    }

    /// A list of these packed: a count, then each one (floats raw).
    fn pack_list(items: &[Self], p: &mut Packer) {
        let at = p.begin_list(items.len());
        for item in items {
            item.pack(p);
        }
        p.end(at);
    }

    /// A list of these out of a frame.
    fn unpack_list(u: &mut Unpacker<'_>) -> Option<Vec<Self>> {
        let (n, end) = u.list()?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(Self::unpack(u)?);
        }
        u.skip_to(end)?;
        Some(out)
    }

    /// A list of these written straight (numbers pack theirs).
    fn encode_list(items: &[Self], w: &mut Writer) {
        w.list(items.len());
        for item in items {
            item.encode(w);
        }
    }

    /// A list of these read straight.
    fn decode_list(d: &mut Decoder<'_>) -> Option<Vec<Self>> {
        let n = d.list()?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(Self::decode(d)?);
        }
        d.leave();
        Some(out)
    }
}
