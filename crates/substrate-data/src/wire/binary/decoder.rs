//! [`Decoder`]: reads the binary form — as a [`Value`] here, borrowed as a
//! [`ValueRef`] (`decoder_ref`), or step by step by a schema reading itself
//! (`decoder_straight`).

use super::{
    DecodeError, MAGIC, MAX_DEPTH, NEW_STRING, STREAM_LEN_BYTES, VERSION, read_varint, tag,
    unzigzag,
};
use crate::model::value::{KindName, Map, Ref, RefKey, Value};
use crate::sync::Arc;

/// Strings a decoder makes room for at once: a message's keys and values.
pub(super) const STRINGS_START: usize = 16;

/// Bytes of a float, and of a 32-bit one.
pub(super) const F64_BYTES: usize = 8;
pub(super) const F32_BYTES: usize = 4;

/// Bytes a map entry takes at least: its key and its value's tag.
pub(super) const MAP_ENTRY_MIN: usize = 2;

/// Reads the binary form. The step-by-step reads ([`Decoder::int`],
/// [`Decoder::map`] …) answer `None` for anything but what
/// [`crate::Schema::encode`] writes there — the schema then reads the
/// slow way, which says what's wrong.
#[derive(Debug)]
pub struct Decoder<'b> {
    pub(super) b: &'b [u8],
    pub(super) i: usize,
    pub(super) strings: Vec<&'b str>,
    /// Map keys made so far, by table index: each key is allocated once.
    pub(super) keys: Vec<Option<Arc<str>>>,
    pub(super) depth: usize,
}

impl<'b> Decoder<'b> {
    /// After the header: the root value next.
    ///
    /// # Errors
    ///
    /// [`DecodeError`] when the header isn't `SDAT` and this version.
    pub fn open(bytes: &'b [u8]) -> Result<Self, DecodeError> {
        let mut d = Decoder {
            b: bytes,
            i: 0,
            strings: Vec::with_capacity(STRINGS_START),
            keys: Vec::new(),
            depth: 0,
        };
        if !bytes.starts_with(MAGIC) {
            return Err(d.error("not Substrate Data (no `SDAT` header)"));
        }
        d.i = MAGIC.len();
        let version = d.byte()?;
        if version != VERSION {
            return Err(d.error(format!(
                "Substrate Data version {version}, this reads version {VERSION}"
            )));
        }
        Ok(d)
    }

    /// A stream's body ([`crate::Stream`]): its count and values, its strings its own.
    pub(crate) fn body(bytes: &'b [u8]) -> Self {
        Decoder {
            b: bytes,
            i: 0,
            strings: Vec::with_capacity(STRINGS_START),
            keys: Vec::new(),
            depth: 0,
        }
    }

    /// A stream's body: its bytes, read past.
    pub(super) fn stream_body(&mut self) -> Result<&'b [u8], DecodeError> {
        let len = u32::from_le_bytes(self.fixed::<STREAM_LEN_BYTES>()?) as usize;
        self.take(len)
    }

    /// A stream's values, decoded as a list.
    pub(super) fn stream_values(&mut self) -> Result<Vec<Value>, DecodeError> {
        let mut d = Decoder::body(self.stream_body()?);
        d.depth = self.depth;
        let n = d.count(1)?;
        let mut items = Vec::with_capacity(n);
        for _ in 0..n {
            items.push(d.value()?);
        }
        d.end()?;
        Ok(items)
    }

    /// Whether every byte was read.
    #[must_use]
    pub fn at_end(&self) -> bool {
        self.i == self.b.len()
    }

    pub(super) fn end(&self) -> Result<(), DecodeError> {
        if self.at_end() {
            Ok(())
        } else {
            Err(self.error("bytes after the value"))
        }
    }

    pub(super) fn error(&self, message: impl Into<String>) -> DecodeError {
        DecodeError {
            message: message.into(),
            offset: self.i,
        }
    }

    pub(super) fn byte(&mut self) -> Result<u8, DecodeError> {
        let b = *self
            .b
            .get(self.i)
            .ok_or_else(|| self.error("unexpected end"))?;
        self.i += 1;
        Ok(b)
    }

    pub(super) fn varint(&mut self) -> Result<u64, DecodeError> {
        read_varint(self.b, &mut self.i)
            .ok_or_else(|| self.error("a varint cut short or longer than 64 bits"))
    }

    /// A length, checked against what's left: each of the `count` things takes at least `min` bytes.
    pub(super) fn count(&mut self, min: usize) -> Result<usize, DecodeError> {
        let n = self.varint()?;
        let left = self.b.len() - self.i;
        match usize::try_from(n) {
            Ok(n) if n <= left / min.max(1) => Ok(n),
            _ => Err(self.error(format!("length {n} past the end ({left} bytes left)"))),
        }
    }

    pub(super) fn fixed<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let bytes = self
            .b
            .get(self.i..self.i + N)
            .ok_or_else(|| self.error("unexpected end"))?;
        self.i += N;
        bytes.try_into().map_err(|_| self.error("unexpected end"))
    }

    /// `len` bytes.
    pub(super) fn take(&mut self, len: usize) -> Result<&'b [u8], DecodeError> {
        let end = self
            .i
            .checked_add(len)
            .filter(|&e| e <= self.b.len())
            .ok_or_else(|| self.error("bytes past the end"))?;
        let out = &self.b[self.i..end];
        self.i = end;
        Ok(out)
    }

    /// A packed list of floats `N` bytes wide: each one's bytes, read in place (no alignment asked).
    pub(super) fn packed_floats<const N: usize>(
        &mut self,
    ) -> Result<std::slice::Iter<'b, [u8; N]>, DecodeError> {
        let n = self.count(N)?;
        let bytes = self.take(n * N)?;
        Ok(bytes.as_chunks::<N>().0.iter())
    }

    /// A length and that many bytes.
    pub(super) fn length_bytes(&mut self) -> Result<&'b [u8], DecodeError> {
        let len =
            usize::try_from(self.varint()?).map_err(|_| self.error("a length beyond memory"))?;
        self.take(len)
    }

    /// A string: new (read, and added to the table) or the table's.
    pub(super) fn string(&mut self) -> Result<&'b str, DecodeError> {
        let i = self.string_index()?;
        Ok(self.strings[i])
    }

    pub(super) fn string_index(&mut self) -> Result<usize, DecodeError> {
        let v = self.varint()?;
        if v & NEW_STRING == 0 {
            let i = (v >> 1) as usize;
            return if i < self.strings.len() {
                Ok(i)
            } else {
                Err(self.error(format!("string {i} isn't in the table")))
            };
        }
        let len = usize::try_from(v >> 1).map_err(|_| self.error("a length beyond memory"))?;
        let at = self.i;
        let s = std::str::from_utf8(self.take(len)?).map_err(|_| DecodeError {
            message: "a string isn't UTF-8".into(),
            offset: at,
        })?;
        self.strings.push(s);
        Ok(self.strings.len() - 1)
    }

    /// A map key: shared with every other map that has it.
    pub(super) fn shared_key(&mut self) -> Result<Arc<str>, DecodeError> {
        let i = self.string_index()?;
        if self.keys.len() <= i {
            self.keys.resize(i + 1, None);
        }
        Ok(self.keys[i]
            .get_or_insert_with(|| self.strings[i].into())
            .clone())
    }

    /// A reference's kind and id, each checked: a malformed one is an error.
    pub(super) fn reference(&mut self) -> Result<(KindName, &'b str), DecodeError> {
        let kind_at = self.i;
        let kind = KindName::new(self.string()?).map_err(|e| DecodeError {
            message: e.to_string(),
            offset: kind_at,
        })?;
        let id_at = self.i;
        let id = self.string()?;
        if !RefKey::is_valid(id) {
            return Err(DecodeError {
                message: format!("`{id}` isn't a reference's id"),
                offset: id_at,
            });
        }
        Ok((kind, id))
    }

    pub(super) fn deeper(&mut self) -> Result<(), DecodeError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.error(format!("nested deeper than {MAX_DEPTH} levels")));
        }
        Ok(())
    }

    /// The next value, owned.
    ///
    /// # Errors
    ///
    /// [`DecodeError`] when the bytes aren't a value.
    pub fn value(&mut self) -> Result<Value, DecodeError> {
        Ok(match self.byte()? {
            tag::NULL => Value::Null,
            tag::FALSE => Value::Bool(false),
            tag::TRUE => Value::Bool(true),
            tag::INT => Value::Int(unzigzag(self.varint()?)),
            tag::FLOAT => Value::Float(f64::from_le_bytes(self.fixed()?)),
            tag::FLOAT32 => Value::Float(f64::from(f32::from_le_bytes(self.fixed()?))),
            tag::FLOATS => Value::List(
                self.packed_floats::<F64_BYTES>()?
                    .map(|f| Value::Float(f64::from_le_bytes(*f)))
                    .collect(),
            ),
            tag::FLOATS32 => Value::List(
                self.packed_floats::<F32_BYTES>()?
                    .map(|f| Value::Float(f64::from(f32::from_le_bytes(*f))))
                    .collect(),
            ),
            tag::STR => Value::Str(self.string()?.to_owned()),
            tag::LIST => self.list_value()?,
            tag::MAP => self.map_value()?,
            tag::REF => {
                let (kind, id) = self.reference()?;
                Value::Ref(Ref::new(kind, RefKey::spelled(id)))
            }
            tag::BYTES => Value::Bytes(self.length_bytes()?.to_vec()),
            tag::INTS => self.ints_value()?,
            tag::STREAM => Value::List(self.stream_values()?),
            t => {
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
            let k = self.shared_key()?;
            map.push_unchecked(k, self.value()?);
        }
        self.depth -= 1;
        Ok(Value::Map(map))
    }

    fn ints_value(&mut self) -> Result<Value, DecodeError> {
        let n = self.count(1)?;
        let mut items = Vec::with_capacity(n);
        for _ in 0..n {
            items.push(Value::Int(unzigzag(self.varint()?)));
        }
        Ok(Value::List(items))
    }
}
