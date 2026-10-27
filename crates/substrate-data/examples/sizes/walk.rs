//! A walk over the bytes of the binary form that says where they go: tags,
//! counts, strings (first use and back-references, keys and values),
//! integers, floats, bytes. It follows the format in `docs/format.md`, not
//! the crate's decoder, and fails if it does not consume the input exactly.

use substrate_data::binary::{MAGIC, NEW_STRING, STREAM_LEN_BYTES, tag};

/// Bytes spent on each kind of thing.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Spent {
    pub(crate) header: usize,
    pub(crate) tags: usize,
    pub(crate) counts: usize,
    pub(crate) key_literal: usize,
    pub(crate) key_ref: usize,
    pub(crate) str_literal: usize,
    pub(crate) str_ref: usize,
    pub(crate) ints: usize,
    pub(crate) floats: usize,
    pub(crate) blobs: usize,
}

impl Spent {
    /// Everything counted.
    pub(crate) fn total(&self) -> usize {
        self.header
            + self.tags
            + self.counts
            + self.key_literal
            + self.key_ref
            + self.str_literal
            + self.str_ref
            + self.ints
            + self.floats
            + self.blobs
    }
}

struct Walk<'a> {
    b: &'a [u8],
    i: usize,
    strings: usize,
    spent: Spent,
}

/// Where the bytes of `bytes` (a whole document) go; `None` if it isn't one.
pub(crate) fn spent(bytes: &[u8]) -> Option<Spent> {
    let mut w = Walk {
        b: bytes,
        i: 0,
        strings: 0,
        spent: Spent::default(),
    };
    w.header()?;
    w.value()?;
    (w.i == bytes.len()).then_some(w.spent)
}

impl Walk<'_> {
    fn header(&mut self) -> Option<()> {
        self.b.starts_with(MAGIC).then_some(())?;
        self.i = MAGIC.len() + 1;
        self.spent.header = self.i;
        Some(())
    }

    /// A varint and the bytes it took.
    fn varint(&mut self) -> Option<(u64, usize)> {
        let start = self.i;
        let mut n = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = *self.b.get(self.i)?;
            self.i += 1;
            n |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Some((n, self.i - start));
            }
        }
        None
    }

    fn take(&mut self, n: usize) -> Option<()> {
        (self.i + n <= self.b.len()).then(|| self.i += n)
    }

    /// A string: its literal or its index, counted as a key or a value.
    fn string(&mut self, key: bool) -> Option<()> {
        let (v, width) = self.varint()?;
        if v & NEW_STRING == 1 {
            let len = usize::try_from(v >> 1).ok()?;
            self.take(len)?;
            self.strings += 1;
            let spent = width + len;
            if key {
                self.spent.key_literal += spent;
            } else {
                self.spent.str_literal += spent;
            }
        } else if key {
            self.spent.key_ref += width;
        } else {
            self.spent.str_ref += width;
        }
        Some(())
    }

    fn count(&mut self) -> Option<usize> {
        let (n, width) = self.varint()?;
        self.spent.counts += width;
        usize::try_from(n).ok()
    }

    fn value(&mut self) -> Option<()> {
        let t = *self.b.get(self.i)?;
        self.i += 1;
        self.spent.tags += 1;
        match t {
            tag::NULL | tag::FALSE | tag::TRUE => Some(()),
            tag::INT => {
                let (_, width) = self.varint()?;
                self.spent.ints += width;
                Some(())
            }
            tag::FLOAT => self.fixed(8),
            tag::FLOAT32 => self.fixed(4),
            tag::STR => self.string(false),
            tag::LIST => self.items(false),
            tag::MAP => self.items(true),
            tag::REF => self.string(false).and_then(|()| self.string(false)),
            tag::BYTES => {
                let n = self.count()?;
                self.spent.blobs += n;
                self.take(n)
            }
            tag::INTS => self.packed_ints(),
            tag::FLOATS => self.packed_floats(8),
            tag::FLOATS32 => self.packed_floats(4),
            tag::STREAM => self.stream(),
            _ => None,
        }
    }

    fn fixed(&mut self, n: usize) -> Option<()> {
        self.spent.floats += n;
        self.take(n)
    }

    fn items(&mut self, keyed: bool) -> Option<()> {
        for _ in 0..self.count()? {
            if keyed {
                self.string(true)?;
            }
            self.value()?;
        }
        Some(())
    }

    fn packed_ints(&mut self) -> Option<()> {
        for _ in 0..self.count()? {
            let (_, width) = self.varint()?;
            self.spent.ints += width;
        }
        Some(())
    }

    fn packed_floats(&mut self, width: usize) -> Option<()> {
        let n = self.count()?;
        self.spent.floats += n * width;
        self.take(n * width)
    }

    /// A stream: its length, then a count and values with their own string table.
    fn stream(&mut self) -> Option<()> {
        self.spent.counts += STREAM_LEN_BYTES;
        self.take(STREAM_LEN_BYTES)?;
        let outer = std::mem::take(&mut self.strings);
        let n = self.count()?;
        for _ in 0..n {
            self.value()?;
        }
        self.strings = outer;
        Some(())
    }
}
