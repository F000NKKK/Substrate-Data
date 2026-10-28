//! [`Items`]: the items of a [`Stream`](crate::Stream), read one at a time.

use std::iter::FusedIterator;
use std::marker::PhantomData;

use super::item_error::ItemError;
use crate::binary::Decoder;
use crate::schema::Schema;

/// The items of a [`Stream`](crate::Stream), each read when asked for
/// ([`Stream::iter`](crate::Stream::iter)).
///
/// An item that cannot be read is an `Err` naming its index and byte; the
/// iterator ends there, since the items after it cannot be told apart.
#[derive(Debug)]
pub struct Items<'a, T> {
    d: Decoder<'a>,
    index: usize,
    len: usize,
    done: bool,
    item: PhantomData<fn() -> T>,
}

impl<'a, T> Items<'a, T> {
    pub(super) fn new(items: &'a [u8], len: usize) -> Self {
        Items {
            d: Decoder::body(items),
            index: 0,
            len,
            done: false,
            item: PhantomData,
        }
    }

    /// Ends the iteration on the fault at `offset`.
    fn fault(&mut self, offset: usize) -> ItemError {
        self.done = true;
        ItemError {
            index: self.index,
            offset,
        }
    }
}

impl<T: Schema> Iterator for Items<'_, T> {
    type Item = Result<T, ItemError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let offset = self.d.offset();
        if self.index == self.len {
            // Past the last item: bytes left over are a fault, once.
            if self.d.at_end() {
                self.done = true;
                return None;
            }
            return Some(Err(self.fault(offset)));
        }
        match T::decode(&mut self.d) {
            Some(item) => {
                self.index += 1;
                Some(Ok(item))
            }
            None => Some(Err(self.fault(offset))),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.done {
            return (0, Some(0));
        }
        // Each item may be the one that fails, and so is the leftover check.
        (0, Some(self.len - self.index + 1))
    }
}

impl<T: Schema> FusedIterator for Items<'_, T> {}
