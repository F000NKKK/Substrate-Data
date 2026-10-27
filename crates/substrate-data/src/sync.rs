//! A reader-writer lock that opens whether or not it is poisoned: a thread
//! that panicked while holding it leaves the data as it last was.

use std::sync::{PoisonError, RwLockReadGuard, RwLockWriteGuard};

pub(crate) use std::sync::{Arc, OnceLock};

/// [`std::sync::RwLock`], its locks poison-free.
#[derive(Debug, Default)]
pub(crate) struct RwLock<T: ?Sized>(std::sync::RwLock<T>);

impl<T: ?Sized> RwLock<T> {
    pub(crate) fn read(&self) -> RwLockReadGuard<'_, T> {
        self.0.read().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.write().unwrap_or_else(PoisonError::into_inner)
    }
}
