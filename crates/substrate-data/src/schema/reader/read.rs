//! [`Read`]: what reading a document gives.

use super::Found;
use crate::model::value::Ref;

/// What reading a document gives: the value and the references in it.
#[derive(Debug)]
pub struct Read<T> {
    /// The value read.
    pub value: T,
    /// Every reference in it, with where.
    pub refs: Vec<Found<Ref>>,
}
