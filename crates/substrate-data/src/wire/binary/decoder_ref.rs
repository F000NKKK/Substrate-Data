//! [`Decoder`] reading a value borrowed: strings point into the bytes,
//! lists and maps go in an arena.

use bumpalo::Bump;

use super::decoder::{F32_BYTES, F64_BYTES, MAP_ENTRY_MIN};
use super::seen_keys::SeenKeys;
use super::{DecodeError, Decoder, tag, unzigzag};
use crate::model::borrowed::{MapRef, ValueRef};

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
        Ok(match self.byte()? {
            tag::NULL => ValueRef::Null,
            tag::FALSE => ValueRef::Bool(false),
            tag::TRUE => ValueRef::Bool(true),
            tag::INT => ValueRef::Int(unzigzag(self.varint()?)),
            tag::FLOAT => ValueRef::Float(f64::from_le_bytes(self.fixed()?)),
            tag::FLOAT32 => ValueRef::Float(f64::from(f32::from_le_bytes(self.fixed()?))),
            t @ (tag::FLOATS | tag::FLOATS32) => self.floats_ref(arena, t == tag::FLOATS)?,
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
            tag::INTS => self.ints_ref(arena)?,
            tag::STREAM => {
                let mut d = Decoder::body(self.stream_body()?);
                d.depth = self.depth;
                let n = d.count(1)?;
                let items = d.items_ref(arena, stack, n)?;
                d.end()?;
                ValueRef::List(items)
            }
            t => {
                self.i -= 1;
                return Err(self.error(format!("unknown tag {t}")));
            }
        })
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
        let mut seen = SeenKeys::default();
        for _ in 0..n {
            let at = self.i;
            let k = self.string()?;
            if !seen.insert(k) {
                return Err(DecodeError::duplicate_key(k, at));
            }
            entries.push((k, self.value_ref_in(arena, stack)?));
        }
        self.depth -= 1;
        Ok(ValueRef::Map(MapRef(entries.into_bump_slice())))
    }

    fn ints_ref<'a>(&mut self, arena: &'a Bump) -> Result<ValueRef<'a>, DecodeError> {
        let n = self.count(1)?;
        let mut items = bumpalo::collections::Vec::with_capacity_in(n, arena);
        for _ in 0..n {
            items.push(ValueRef::Int(unzigzag(self.varint()?)));
        }
        Ok(ValueRef::List(items.into_bump_slice()))
    }

    /// A list of floats: eight bytes each (`wide`) or four.
    fn floats_ref<'a>(&mut self, arena: &'a Bump, wide: bool) -> Result<ValueRef<'a>, DecodeError> {
        let items: Vec<ValueRef<'a>> = if wide {
            self.packed_floats::<F64_BYTES>()?
                .map(|f| ValueRef::Float(f64::from_le_bytes(*f)))
                .collect()
        } else {
            self.packed_floats::<F32_BYTES>()?
                .map(|f| ValueRef::Float(f64::from(f32::from_le_bytes(*f))))
                .collect()
        };
        Ok(ValueRef::List(arena.alloc_slice_copy(&items)))
    }
}
