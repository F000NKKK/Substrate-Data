//! [`Stream`]: a list kept in the binary form: its items written as they
//! come (no `Vec` of them on the way), cut out whole when the stream is
//! decoded and read back one at a time, when asked. For the big ones: a
//! table's rows, a picture's pixels, a log.
//!
//! Decoding a stream checks its framing only (its byte length and its item
//! count against the bytes there are), so it costs a copy, not a pass over
//! the items. An item is read when it is asked for, and
//! [`Stream::iter`] yields a `Result` for each: a bad item is an
//! [`ItemError`](crate::ItemError) naming its index and byte.
//!
//! ```
//! use substrate_data::{Schema, Stream};
//!
//! #[derive(Schema)]
//! struct Sheet {
//!     name: String,
//!     cells: Stream<u16>,
//! }
//!
//! let sheet = Sheet { name: "q3".into(), cells: (0..1000u16).map(|i| i % 3).collect() };
//! let back: Sheet = substrate_data::read_bytes(&substrate_data::to_bytes(&sheet)).unwrap();
//! assert_eq!(back.cells.len(), 1000);
//! let twos = back.cells.iter().filter(|c| matches!(c, Ok(2))).count();
//! assert_eq!(twos, 333);
//! // All of them, or the first that cannot be read:
//! let cells: Vec<u16> = back.cells.iter().collect::<Result<_, _>>().unwrap();
//! assert_eq!(cells.len(), 1000);
//! ```

use std::fmt;
use std::marker::PhantomData;

use super::item_error::ItemError;
use super::items::Items;
use super::stream_writer::StreamWriter;
use crate::binary::{Decoder, Writer};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::{Seg, Value};
use crate::schema::{Reader, Schema, Shape};

/// A list of `T` kept written: its body in the binary form, its strings its own.
///
/// A stream that was built here (`collect`, `From<Vec<T>>`, a
/// [`StreamWriter`]) always reads. A decoded one has had its framing checked
/// and nothing more, so an item can fail when it is read: see [`Stream::iter`].
///
/// Two streams are equal when they have the same count and the same bytes,
/// not when their items are equal: the same items written by another encoder
/// (a different but valid spelling of an int, say) are not equal, and a
/// stream with an unreadable item equals only a copy of itself. Compare
/// [`Stream::to_vec`] to compare items.
pub struct Stream<T> {
    /// The items, written one after another.
    items: Vec<u8>,
    len: usize,
    item: PhantomData<fn() -> T>,
}

impl<T> Stream<T> {
    pub(super) fn from_written(items: Vec<u8>, len: usize) -> Self {
        Stream {
            items,
            len,
            item: PhantomData,
        }
    }

    /// How many items it says it has (a decoded stream's are not read yet).
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether it says it has none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// How many bytes it takes.
    #[must_use]
    pub fn bytes_len(&self) -> usize {
        self.items.len()
    }
}

impl<T: Schema> Stream<T> {
    /// Writes items one by one ([`StreamWriter::push`]).
    #[must_use]
    pub fn writer() -> StreamWriter<T> {
        StreamWriter {
            w: Writer::body(),
            len: 0,
            item: PhantomData,
        }
    }

    /// The items, each read as it's asked for.
    ///
    /// An item that cannot be read (a newer item type, a corrupt byte) is an
    /// `Err` naming its index and byte, and the iterator ends there; so does
    /// a stream with bytes left after its counted items, with an `Err` at
    /// index [`len`](Stream::len). `iter().collect::<Result<Vec<_>, _>>()`
    /// reads them all or stops at the first bad one.
    #[must_use]
    pub fn iter(&self) -> Items<'_, T> {
        Items::new(&self.items, self.len)
    }

    /// Every item, read now: the first that cannot be read is the error.
    ///
    /// # Errors
    ///
    /// An [`ItemError`] for the first unreadable item.
    pub fn to_vec(&self) -> Result<Vec<T>, ItemError> {
        self.iter().collect()
    }

    /// Reads every item and drops it: whether the whole stream reads.
    ///
    /// # Errors
    ///
    /// An [`ItemError`] for the first unreadable item.
    pub fn validate(&self) -> Result<(), ItemError> {
        self.iter().try_for_each(|item| item.map(drop))
    }
}

impl<'a, T: Schema> IntoIterator for &'a Stream<T> {
    type Item = Result<T, ItemError>;
    type IntoIter = Items<'a, T>;

    fn into_iter(self) -> Items<'a, T> {
        self.iter()
    }
}

impl<T: Schema> FromIterator<T> for Stream<T> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        let mut w = Stream::writer();
        for item in items {
            w.push(&item);
        }
        w.finish()
    }
}

impl<T: Schema> From<Vec<T>> for Stream<T> {
    fn from(items: Vec<T>) -> Self {
        items.into_iter().collect()
    }
}

impl<T> Clone for Stream<T> {
    fn clone(&self) -> Self {
        Stream::from_written(self.items.clone(), self.len)
    }
}

impl<T> PartialEq for Stream<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.items == other.items
    }
}

impl<T> Default for Stream<T> {
    fn default() -> Self {
        Stream::from_written(Vec::new(), 0)
    }
}

impl<T> fmt::Debug for Stream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Stream({} items, {} bytes)", self.len, self.items.len())
    }
}

impl<T: Schema> Schema for Stream<T> {
    fn shape() -> Shape {
        Shape::List(T::shape)
    }
    /// The slow way (a text file, a value): read whole, then written.
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        let ValueRef::List(items) = v else {
            cx.expected(crate::schema::Expected::List, v);
            return None;
        };
        let mut w = Stream::writer();
        let mut ok = true;
        for (i, item) in items.iter().enumerate() {
            match cx.at(Seg::Index(i), |cx| T::read(item, cx)) {
                Some(x) => w.push(&x),
                None => ok = false,
            }
        }
        ok.then(|| w.finish())
    }
    /// The items that read, up to the first that does not: a `Value` has no
    /// place for an error, so check a decoded stream with [`Stream::validate`].
    fn write(&self) -> Value {
        Value::List(
            self.iter()
                .map_while(Result::ok)
                .map(|item| item.write())
                .collect(),
        )
    }
    fn encode(&self, w: &mut Writer) {
        w.stream_body(self.len, &self.items);
    }
    /// Checks the framing only: the byte length, and that the count fits the
    /// bytes (each item takes one at least, so a hostile count allocates
    /// nothing). The items are read by [`Stream::iter`].
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        let mut body = Decoder::body(d.stream()?);
        let len = body.stream_count()?;
        Some(Stream::from_written(body.rest().to_vec(), len))
    }
    /// In a frame it's a list, as its shape says (the items that read, as
    /// in [`Stream::write`]).
    fn pack(&self, p: &mut Packer) {
        let items: Vec<T> = self.iter().map_while(Result::ok).collect();
        T::pack_list(&items, p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        T::unpack_list(u).map(Stream::from)
    }
}
