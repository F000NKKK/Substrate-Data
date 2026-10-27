//! [`Unpacker`]: reads a frame.

use bumpalo::Bump;

use super::sizes::{F32_BYTES, F64_BYTES};
use super::strings::Strings;
use super::{floats, lanes};
use crate::binary::{
    Decoder, MAX_DEPTH, VARINT_BITS, VARINT_LAST_SHIFT, VARINT_MAX_SHIFT, VARINT_MORE,
    VARINT_SHIFT, read_varint, unzigzag,
};
use crate::schema::{Schema, read_ref_value};
use crate::sync::Arc;

/// A `T` out of a frame; `None` if the frame isn't one.
#[must_use]
pub fn read_frame<T: Schema>(bytes: &[u8]) -> Option<T> {
    let mut u = Unpacker::open(bytes)?;
    let v = T::unpack(&mut u)?;
    u.at_end().then_some(v)
}

/// Reads a frame. Every read answers `None` for bytes that aren't what
/// [`Packer`](super::Packer) writes there.
#[derive(Debug)]
pub struct Unpacker<'b> {
    body: &'b [u8],
    i: usize,
    table: &'b [u8],
    /// How far the table is read.
    table_at: usize,
    strings: Strings<'b>,
    /// Strings handed out shared ([`Unpacker::shared_str`]), by table index:
    /// a string a frame repeats is allocated once.
    shared: Vec<Option<Arc<str>>>,
    depth: usize,
}

impl<'b> Unpacker<'b> {
    /// The frame's body and table found; the root value next.
    #[must_use]
    pub fn open(bytes: &'b [u8]) -> Option<Self> {
        let (table_len, trailer) = read_varint_back(bytes)?;
        let body_end = bytes.len().checked_sub(trailer)?.checked_sub(table_len)?;
        Some(Unpacker {
            body: &bytes[..body_end],
            i: 0,
            table: &bytes[body_end..bytes.len() - trailer],
            table_at: 0,
            strings: Strings::new(),
            shared: Vec::new(),
            depth: 0,
        })
    }

    /// Whether the body is read through.
    #[must_use]
    pub fn at_end(&self) -> bool {
        self.i == self.body.len()
    }

    fn take(&mut self, len: usize) -> Option<&'b [u8]> {
        let end = self.i.checked_add(len).filter(|&e| e <= self.body.len())?;
        let out = &self.body[self.i..end];
        self.i = end;
        Some(out)
    }

    /// An unsigned number ([`Packer::put_u`](super::Packer::put_u)).
    pub fn uint(&mut self) -> Option<u64> {
        read_varint(self.body, &mut self.i)
    }

    /// A signed number.
    pub fn int(&mut self) -> Option<i64> {
        self.uint().map(unzigzag)
    }

    /// A bool.
    pub fn bool(&mut self) -> Option<bool> {
        match self.take(1)? {
            [0] => Some(false),
            [1] => Some(true),
            _ => None,
        }
    }

    /// An `f64`.
    pub fn f64(&mut self) -> Option<f64> {
        Some(f64::from_le_bytes(self.take(F64_BYTES)?.try_into().ok()?))
    }

    /// An `f32`.
    pub fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(F32_BYTES)?.try_into().ok()?))
    }

    /// A count of things each at least `min` bytes: no bigger than what's left.
    fn count_of(&mut self, min: usize) -> Option<usize> {
        let n = usize::try_from(self.uint()?).ok()?;
        (n <= (self.body.len() - self.i) / min.max(1)).then_some(n)
    }

    /// A list's count (every value takes a byte at least).
    pub fn count(&mut self) -> Option<usize> {
        self.count_of(1)
    }

    /// A list of `f64`s.
    pub fn f64s(&mut self) -> Option<Vec<f64>> {
        let (_, narrow, raw) = self.packed_floats(true)?;
        Some(if narrow {
            floats::widen(raw)
        } else {
            raw.as_chunks::<F64_BYTES>()
                .0
                .iter()
                .map(|b| f64::from_le_bytes(*b))
                .collect()
        })
    }

    /// Bit-packed numbers ([`Packer::put_uints`](super::Packer::put_uints)), each made a `T` by `of`.
    pub fn uints<T>(&mut self, of: impl Fn(u64) -> Option<T>) -> Option<Vec<T>> {
        let (n, bits, raw) = self.packed_ints()?;
        let mut out = Vec::with_capacity(n);
        let mut ok = true;
        lanes::unpack(raw, n, bits, |v| match of(v) {
            Some(t) if ok => out.push(t),
            _ => ok = false,
        });
        ok.then_some(out)
    }

    /// A list of `f32`s.
    pub fn f32s(&mut self) -> Option<Vec<f32>> {
        let (_, _, raw) = self.packed_floats(false)?;
        Some(
            raw.as_chunks::<F32_BYTES>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect(),
        )
    }

    /// A packed list of ints, unread: how many, bits each, their bytes.
    pub fn packed_ints(&mut self) -> Option<(usize, u32, &'b [u8])> {
        let n = usize::try_from(self.uint()?).ok()?;
        let bits = u32::from(*self.take(1)?.first()?);
        if !(1..=u64::BITS).contains(&bits)
            || n > (self.body.len() - self.i) * u8::BITS as usize / bits as usize
        {
            return None;
        }
        let bytes = self.take(lanes::packed_bytes(n, bits)?)?;
        Some((n, bits, bytes))
    }

    /// A list of floats, unread: how many, whether they're `f32`s, their
    /// bytes. A list of `f64` says whether it's narrowed (`wide`); one of
    /// `f32` always is.
    pub fn packed_floats(&mut self, wide: bool) -> Option<(usize, bool, &'b [u8])> {
        let n = self.count()?;
        let narrow = if wide { self.bool()? } else { true };
        let width = if narrow { F32_BYTES } else { F64_BYTES };
        Some((n, narrow, self.take(n.checked_mul(width)?)?))
    }

    /// A string, borrowed from the frame: the table read as far as it's needed.
    pub fn str(&mut self) -> Option<&'b str> {
        let i = self.string_index()?;
        self.strings.get(i)
    }

    /// A string as an `Arc<str>`: one allocation for every time the frame has it.
    pub fn shared_str(&mut self) -> Option<Arc<str>> {
        let i = self.string_index()?;
        if self.shared.len() <= i {
            self.shared.resize(i + 1, None);
        }
        let s = self.strings.get(i)?;
        Some(self.shared[i].get_or_insert_with(|| s.into()).clone())
    }

    /// A string's index, the table read up to it.
    fn string_index(&mut self) -> Option<usize> {
        let i = usize::try_from(self.uint()?).ok()?;
        while self.strings.len() <= i {
            let len = usize::try_from(read_varint(self.table, &mut self.table_at)?).ok()?;
            let end = self
                .table_at
                .checked_add(len)
                .filter(|&e| e <= self.table.len())?;
            let s = std::str::from_utf8(&self.table[self.table_at..end]).ok()?;
            self.table_at = end;
            self.strings.push(s);
        }
        Some(i)
    }

    /// Bytes, borrowed from the frame.
    pub fn bytes(&mut self) -> Option<&'b [u8]> {
        let len = self.count()?;
        self.take(len)
    }

    /// A value held in its binary form ([`Packer::put_tagged`](super::Packer::put_tagged)).
    pub fn tagged<T: Schema>(&mut self) -> Option<T> {
        let bytes = self.bytes()?;
        let mut d = Decoder::body(bytes);
        if let Some(v) = T::decode(&mut d)
            && d.at_end()
        {
            return Some(v);
        }
        let arena = Bump::new();
        let mut any = Decoder::body(bytes);
        let value = any.value_ref(&arena).ok()?;
        any.at_end().then_some(())?;
        read_ref_value(&value).ok()
    }

    /// A record starts: where it ends.
    pub fn begin(&mut self) -> Option<usize> {
        let len = self.count()?;
        self.depth += 1;
        (self.depth <= MAX_DEPTH).then_some(self.i + len)
    }

    /// A list ([`Packer::begin_list`](super::Packer::begin_list)): how many items, and where it ends.
    pub fn list(&mut self) -> Option<(usize, usize)> {
        let n = self.count()?;
        let len = self.count()?;
        Some((n, self.i + len))
    }

    /// Steps to `end` (a list's), past what's left of it.
    pub fn skip_to(&mut self, end: usize) -> Option<()> {
        (self.i <= end && end <= self.body.len()).then(|| self.i = end)
    }

    /// Where it's read up to.
    #[must_use]
    pub fn position(&self) -> usize {
        self.i
    }

    /// A reader of the same frame from `at`, with a string cache of its own.
    #[must_use]
    pub fn fork(&self, at: usize) -> Unpacker<'b> {
        Unpacker {
            body: self.body,
            i: at,
            table: self.table,
            table_at: 0,
            strings: Strings::new(),
            shared: Vec::new(),
            depth: self.depth,
        }
    }

    /// Whether the record ending at `end` has more fields.
    #[must_use]
    pub fn more(&self, end: usize) -> bool {
        self.i < end
    }

    /// The record ending at `end` is done: fields this reader doesn't know skipped.
    pub fn leave(&mut self, end: usize) -> Option<()> {
        self.depth -= 1;
        (self.i <= end).then(|| self.i = end)
    }
}

/// A varint written backwards at the end of `bytes`: its value, and how
/// many bytes it took.
fn read_varint_back(bytes: &[u8]) -> Option<(usize, usize)> {
    let mut n = 0u64;
    let mut shift = 0;
    for (taken, b) in bytes.iter().rev().enumerate() {
        if shift >= VARINT_MAX_SHIFT {
            return None;
        }
        let bits = u64::from(*b) & VARINT_BITS;
        if shift == VARINT_LAST_SHIFT && bits > 1 {
            return None;
        }
        n |= bits << shift;
        if b & VARINT_MORE == 0 {
            return Some((usize::try_from(n).ok()?, taken + 1));
        }
        shift += VARINT_SHIFT;
    }
    None
}
