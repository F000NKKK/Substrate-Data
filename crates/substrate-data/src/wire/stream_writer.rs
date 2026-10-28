//! [`StreamWriter`]: writes a [`Stream`]'s items as they come.

use std::marker::PhantomData;

use super::stream::Stream;
use crate::binary::Writer;
use crate::schema::Schema;

/// Writes a [`Stream`]'s items as they come.
#[derive(Debug)]
pub struct StreamWriter<T> {
    pub(super) w: Writer,
    pub(super) len: usize,
    pub(super) item: PhantomData<fn() -> T>,
}

impl<T: Schema> StreamWriter<T> {
    /// Writes `item`.
    pub fn push(&mut self, item: &T) {
        item.encode(&mut self.w);
        self.len += 1;
    }

    /// The stream written.
    #[must_use]
    pub fn finish(self) -> Stream<T> {
        Stream::from_written(self.w.finish(), self.len)
    }
}
