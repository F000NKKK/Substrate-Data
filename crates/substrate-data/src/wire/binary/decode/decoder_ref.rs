//! [`Decoder`] reading a value borrowed: strings point into the bytes,
//! lists and maps go in an arena.

use std::collections::HashSet;

use bumpalo::Bump;

use super::decoder::{F32_BYTES, F64_BYTES, MAP_ENTRY_MIN};
use super::decoder_value::inline_int;
use crate::binary::{DecodeError, Decoder, tag, unzigzag};
use crate::model::borrowed::{MapRef, ValueRef};
use crate::model::quick_hash::QuickBuild;

/// Maps up to this many keys are checked for a repeat by scanning.
const SCAN_MAX: usize = 16;

impl<'b> Decoder<'b> {
    /// The next value, borrowed: strings point into the bytes, lists and maps go in `arena`.
    ///
    /// # Errors
    ///
    /// [`DecodeError`] when the bytes aren't a value.
    pub fn value_ref<'a>(&mut self, arena: &'a Bump) -> Result<ValueRef<'a>, DecodeError>
    where
        'b: 'a,
    {
        let mut stack = Vec::new();
        self.value_ref_in(arena, &mut stack)
    }

    /// [`Decoder::value_ref`]: children gather on `stack`, then move into `arena` as one slice.
    fn value_ref_in<'a>(
        &mut self,
        arena: &'a Bump,
        stack: &mut Vec<ValueRef<'a>>,
    ) -> Result<ValueRef<'a>, DecodeError>
    where
        'b: 'a,
    {
        let t = self.byte()?;
        Ok(match t {
            tag::NULL => ValueRef::Null,
            tag::FALSE => ValueRef::Bool(false),
            tag::TRUE => ValueRef::Bool(true),
            tag::INT => ValueRef::Int(unzigzag(self.varint()?)),
            tag::FLOAT => ValueRef::Float(f64::from_le_bytes(self.fixed()?)),
            tag::FLOAT32 => ValueRef::Float(f64::from(f32::from_le_bytes(self.fixed()?))),
            tag::STR => ValueRef::Str(self.string()?),
            tag::LIST => {
                let n = self.count(1)?;
                self.deeper()?;
                let items = self.items_ref(arena, stack, n)?;
                self.depth -= 1;
                ValueRef::List(items)
            }
            tag::MAP => self.map_ref(arena, stack)?,
            tag::REF => {
                let (kind, id) = self.reference()?;
                ValueRef::Ref { kind, id }
            }
            tag::BYTES => ValueRef::Bytes(self.length_bytes()?),
            tag::STREAM => self.stream_ref(arena, stack)?,
            tag::INTS | tag::FLOATS | tag::FLOATS32 => self.packed_ref(arena, t)?,
            tag::MAP_SHAPE if self.v2 => {
                let shape = self.shape_spec()?;
                self.shaped_ref(arena, stack, shape)?
            }
            tag::RECORDS if self.v2 => self.records_ref(arena, stack)?,
            t if self.is_inline_int(t) => ValueRef::Int(inline_int(t)),
            t if self.is_indexed(t) => ValueRef::Str(self.indexed(t)?),
            _ => {
                self.i -= 1;
                return Err(self.error(format!("unknown tag {t}")));
            }
        })
    }

    fn stream_ref<'a>(
        &mut self,
        arena: &'a Bump,
        stack: &mut Vec<ValueRef<'a>>,
    ) -> Result<ValueRef<'a>, DecodeError>
    where
        'b: 'a,
    {
        let body = self.stream_body()?;
        let mut d = self.inner(body);
        let n = d.count(1)?;
        let items = d.items_ref(arena, stack, n)?;
        d.end()?;
        Ok(ValueRef::List(items))
    }

    /// `n` values gathered on `stack`, then moved into `arena` as one slice.
    fn items_ref<'a>(
        &mut self,
        arena: &'a Bump,
        stack: &mut Vec<ValueRef<'a>>,
        n: usize,
    ) -> Result<&'a [ValueRef<'a>], DecodeError>
    where
        'b: 'a,
    {
        let start = stack.len();
        for _ in 0..n {
            let v = self.value_ref_in(arena, stack)?;
            stack.push(v);
        }
        let items = arena.alloc_slice_copy(&stack[start..]);
        stack.truncate(start);
        Ok(items)
    }

    fn map_ref<'a>(
        &mut self,
        arena: &'a Bump,
        stack: &mut Vec<ValueRef<'a>>,
    ) -> Result<ValueRef<'a>, DecodeError>
    where
        'b: 'a,
    {
        let n = self.count(MAP_ENTRY_MIN)?;
        self.deeper()?;
        let mut entries = bumpalo::collections::Vec::with_capacity_in(n, arena);
        let mut set = None;
        for _ in 0..n {
            let at = self.i;
            let k = self.string()?;
            if repeats(&entries, k, &mut set) {
                return Err(DecodeError::duplicate_key(k, at));
            }
            entries.push((k, self.value_ref_in(arena, stack)?));
        }
        self.depth -= 1;
        Ok(ValueRef::Map(MapRef(entries.into_bump_slice())))
    }

    /// The entries of a map of shape `shape`: its values, one per key.
    fn shaped_entries_ref<'a>(
        &mut self,
        arena: &'a Bump,
        stack: &mut Vec<ValueRef<'a>>,
        shape: usize,
        n: usize,
    ) -> Result<ValueRef<'a>, DecodeError>
    where
        'b: 'a,
    {
        let mut entries = bumpalo::collections::Vec::with_capacity_in(n, arena);
        for at in 0..n {
            let k = self.strings[self.shapes.key(shape, at)];
            entries.push((k, self.value_ref_in(arena, stack)?));
        }
        Ok(ValueRef::Map(MapRef(entries.into_bump_slice())))
    }

    fn shaped_ref<'a>(
        &mut self,
        arena: &'a Bump,
        stack: &mut Vec<ValueRef<'a>>,
        shape: usize,
    ) -> Result<ValueRef<'a>, DecodeError>
    where
        'b: 'a,
    {
        let n = self.shape_width(shape)?;
        self.deeper()?;
        let map = self.shaped_entries_ref(arena, stack, shape, n)?;
        self.depth -= 1;
        Ok(map)
    }

    fn records_ref<'a>(
        &mut self,
        arena: &'a Bump,
        stack: &mut Vec<ValueRef<'a>>,
    ) -> Result<ValueRef<'a>, DecodeError>
    where
        'b: 'a,
    {
        let n = self.count(1)?;
        let shape = self.shape_spec()?;
        let width = self.shapes.width(shape);
        let left = self.b.len() - self.i;
        if n > left / width {
            return Err(self.error(format!("length {n} past the end ({left} bytes left)")));
        }
        self.deeper()?;
        let start = stack.len();
        for _ in 0..n {
            self.deeper()?;
            let record = self.shaped_entries_ref(arena, stack, shape, width)?;
            stack.push(record);
            self.depth -= 1;
        }
        self.depth -= 1;
        let items = arena.alloc_slice_copy(&stack[start..]);
        stack.truncate(start);
        Ok(ValueRef::List(items))
    }

    /// A packed list of ints (`t` is [`tag::INTS`]) or floats.
    fn packed_ref<'a>(&mut self, arena: &'a Bump, t: u8) -> Result<ValueRef<'a>, DecodeError> {
        let items: Vec<ValueRef<'a>> = match t {
            tag::INTS => {
                let n = self.count(1)?;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(ValueRef::Int(unzigzag(self.varint()?)));
                }
                items
            }
            tag::FLOATS => self
                .packed_floats::<F64_BYTES>()?
                .map(|f| ValueRef::Float(f64::from_le_bytes(*f)))
                .collect(),
            _ => self
                .packed_floats::<F32_BYTES>()?
                .map(|f| ValueRef::Float(f64::from(f32::from_le_bytes(*f))))
                .collect(),
        };
        Ok(ValueRef::List(arena.alloc_slice_copy(&items)))
    }
}

/// Whether `entries` has `key` already (when it doesn't, `key` is noted in
/// `set`, if there are enough entries to keep one).
fn repeats<'a>(
    entries: &bumpalo::collections::Vec<'_, (&'a str, ValueRef<'_>)>,
    key: &'a str,
    set: &mut Option<HashSet<&'a str, QuickBuild>>,
) -> bool {
    if let Some(set) = set {
        return !set.insert(key);
    }
    if entries.iter().any(|(k, _)| *k == key) {
        return true;
    }
    if entries.len() >= SCAN_MAX {
        let mut all: HashSet<&str, QuickBuild> = entries.iter().map(|(k, _)| *k).collect();
        all.insert(key);
        *set = Some(all);
    }
    false
}
