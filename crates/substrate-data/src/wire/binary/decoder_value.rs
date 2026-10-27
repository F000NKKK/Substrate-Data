//! [`Decoder`] reading a value as an owned [`Value`].

use super::decoder::{F32_BYTES, F64_BYTES, MAP_ENTRY_MIN};
use super::{DecodeError, Decoder, tag, unzigzag};
use crate::model::value::{Map, Ref, RefKey, Value};

/// The int a tag of its own is (`tag` at least [`tag::INT_INLINE`]).
pub(super) fn inline_int(t: u8) -> i64 {
    i64::from(t - tag::INT_INLINE) + super::INLINE_INT_MIN
}

impl Decoder<'_> {
    /// The next value, owned.
    ///
    /// # Errors
    ///
    /// [`DecodeError`] when the bytes aren't a value.
    pub fn value(&mut self) -> Result<Value, DecodeError> {
        let t = self.byte()?;
        Ok(match t {
            tag::NULL => Value::Null,
            tag::FALSE => Value::Bool(false),
            tag::TRUE => Value::Bool(true),
            tag::INT => Value::Int(unzigzag(self.varint()?)),
            tag::FLOAT => Value::Float(f64::from_le_bytes(self.fixed()?)),
            tag::FLOAT32 => Value::Float(f64::from(f32::from_le_bytes(self.fixed()?))),
            tag::STR => Value::Str(self.string()?.to_owned()),
            tag::LIST => self.list_value()?,
            tag::MAP => self.map_value()?,
            tag::REF => {
                let (kind, id) = self.reference()?;
                Value::Ref(Ref::new(kind, RefKey::spelled(id)))
            }
            tag::BYTES => Value::Bytes(self.length_bytes()?.to_vec()),
            tag::STREAM => Value::List(self.stream_values()?),
            tag::INTS | tag::FLOATS | tag::FLOATS32 => self.packed_value(t)?,
            tag::MAP_SHAPE if self.v2 => {
                let shape = self.shape_spec()?;
                self.shaped_map_value(shape)?
            }
            tag::RECORDS if self.v2 => self.records_value()?,
            t if self.is_inline_int(t) => Value::Int(inline_int(t)),
            t if self.is_indexed(t) => Value::Str(self.indexed(t)?.to_owned()),
            _ => {
                self.i -= 1;
                return Err(self.error(format!("unknown tag {t}")));
            }
        })
    }

    fn list_value(&mut self) -> Result<Value, DecodeError> {
        let n = self.count(1)?;
        self.deeper()?;
        let mut items = Vec::with_capacity(n);
        for _ in 0..n {
            items.push(self.value()?);
        }
        self.depth -= 1;
        Ok(Value::List(items))
    }

    fn map_value(&mut self) -> Result<Value, DecodeError> {
        let n = self.count(MAP_ENTRY_MIN)?;
        self.deeper()?;
        let mut map = Map::with_capacity(n);
        for _ in 0..n {
            let at = self.i;
            let i = self.string_index()?;
            if map.index_of(self.strings[i]).is_some() {
                return Err(DecodeError::duplicate_key(self.strings[i], at));
            }
            let k = self.shared_key_at(i);
            map.push_unchecked(k, self.value()?);
        }
        self.depth -= 1;
        Ok(Value::Map(map))
    }

    /// A map of shape `shape`: its values, one per key.
    fn shaped_map_value(&mut self, shape: usize) -> Result<Value, DecodeError> {
        let n = self.shape_width(shape)?;
        self.deeper()?;
        let map = self.shaped_entries(shape, n)?;
        self.depth -= 1;
        Ok(Value::Map(map))
    }

    fn shaped_entries(&mut self, shape: usize, n: usize) -> Result<Map, DecodeError> {
        let mut map = Map::with_capacity(n);
        for at in 0..n {
            let k = self.shared_key_at(self.shapes.key(shape, at));
            map.push_unchecked(k, self.value()?);
        }
        Ok(map)
    }

    /// A list of maps of one shape: a count, the shape, the values.
    fn records_value(&mut self) -> Result<Value, DecodeError> {
        let n = self.count(1)?;
        let shape = self.shape_spec()?;
        let width = self.shapes.width(shape);
        let left = self.b.len() - self.i;
        if n > left / width {
            return Err(self.error(format!("length {n} past the end ({left} bytes left)")));
        }
        self.deeper()?;
        let mut items = Vec::with_capacity(n);
        for _ in 0..n {
            self.deeper()?;
            items.push(Value::Map(self.shaped_entries(shape, width)?));
            self.depth -= 1;
        }
        self.depth -= 1;
        Ok(Value::List(items))
    }

    /// A packed list of ints (`t` is [`tag::INTS`]) or floats.
    fn packed_value(&mut self, t: u8) -> Result<Value, DecodeError> {
        Ok(Value::List(match t {
            tag::INTS => {
                let n = self.count(1)?;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(Value::Int(unzigzag(self.varint()?)));
                }
                items
            }
            tag::FLOATS => self
                .packed_floats::<F64_BYTES>()?
                .map(|f| Value::Float(f64::from_le_bytes(*f)))
                .collect(),
            _ => self
                .packed_floats::<F32_BYTES>()?
                .map(|f| Value::Float(f64::from(f32::from_le_bytes(*f))))
                .collect(),
        }))
    }
}
