//! A frame read by its [`Shape`] alone, as a [`Value`] — what a tool (an
//! IDE's message inspector) shows of a message it has no code for, and the
//! rule a reader in another language follows: what packs itself is read as
//! packed, everything else as its binary form.

use super::Unpacker;
use crate::model::value::{Map, Value};
use crate::schema::{EnumShape, Shape, StructShape};

/// Bits of a 32-bit float.
const F32_BITS: u32 = 32;

/// The frame `bytes` read as a value of `shape`.
#[must_use]
pub fn read_shaped(bytes: &[u8], shape: &Shape) -> Option<Value> {
    let mut u = Unpacker::open(bytes)?;
    let v = unpack_shaped(&mut u, shape)?;
    u.at_end().then_some(v)
}

/// The next value, of `shape`.
pub fn unpack_shaped(u: &mut Unpacker<'_>, shape: &Shape) -> Option<Value> {
    Some(match shape {
        Shape::Bool => Value::Bool(u.bool()?),
        Shape::Int { .. } => Value::Int(u.int()?),
        Shape::Float { bits } if *bits == F32_BITS => Value::Float(f64::from(u.f32()?)),
        Shape::Float { .. } => Value::Float(u.f64()?),
        // Plain text, as the model spells it (a leading `@` doubled).
        Shape::Str => Value::Str(crate::typed::escape_at(u.str()?)),
        Shape::Atom => Value::Str(crate::typed::escape_at(
            crate::Atom::from_number(u32::try_from(u.uint()?).ok()?)?.as_str(),
        )),
        // A named type packs as what it's stored as.
        Shape::Named(named) => unpack_shaped(u, &named.stored())?,
        // A reference of declared kinds packs only its id, as its id's
        // shape says; one of any kind, its kind and id as text.
        Shape::Ref { kinds, id } => Value::Ref(unpack_ref(u, kinds, &id())?),
        Shape::Bytes => Value::Bytes(u.bytes()?.to_vec()),
        Shape::Optional(inner) => optional(u, &inner())?,
        Shape::List(item) => list(u, &item())?,
        Shape::Struct(s) => record(u, s)?,
        Shape::Enum(e) if e.tag.is_none() => names(u, e)?,
        _ => u.tagged::<Value>()?,
    })
}

/// An optional value: whether it's there, then it.
fn optional(u: &mut Unpacker<'_>, inner: &Shape) -> Option<Value> {
    if u.bool()? {
        unpack_shaped(u, inner)
    } else {
        Some(Value::Null)
    }
}

/// A reference: of declared `kinds`, only its id (packed as `id` says);
/// of any kind, its kind and id as text.
fn unpack_ref(u: &mut Unpacker<'_>, kinds: &[&str], id: &Shape) -> Option<crate::Ref> {
    let Some(kind) = kinds.first() else {
        return Some(crate::Ref::new(
            crate::KindName::new(u.str()?).ok()?,
            crate::RefKey::new(u.str()?).ok()?,
        ));
    };
    Some(crate::Ref::new(
        crate::KindName::new(kind).ok()?,
        crate::RefKey::new(&unpack_id(u, id)?).ok()?,
    ))
}

/// A reference's id packed as `shape`, spelled.
fn unpack_id(u: &mut Unpacker<'_>, shape: &Shape) -> Option<String> {
    match shape {
        Shape::Int { .. } => Some(u.int()?.to_string()),
        Shape::Named(named) => unpack_id(u, &named.stored()),
        _ => Some(u.str()?.to_owned()),
    }
}

/// A list: numbers packed, anything else value by value.
fn list(u: &mut Unpacker<'_>, item: &Shape) -> Option<Value> {
    Some(Value::List(match item {
        Shape::Int { min, .. } if *min < 0 => {
            u.uints(|v| Some(Value::Int(crate::binary::unzigzag(v))))?
        }
        // A 64-bit unsigned list keeps its bits as the model's `Int` does
        // (`u64::MAX` is `Int(-1)`); a narrower one only holds what fits.
        Shape::Int { max, .. } if *max == i64::MAX => {
            u.uints(|v| Some(Value::Int(v.cast_signed())))?
        }
        Shape::Int { .. } => u.uints(|v| i64::try_from(v).ok().map(Value::Int))?,
        Shape::Float { bits } if *bits == F32_BITS => u
            .f32s()?
            .into_iter()
            .map(|f| Value::Float(f64::from(f)))
            .collect(),
        Shape::Float { .. } => u.f64s()?.into_iter().map(Value::Float).collect(),
        _ => {
            let (n, end) = u.list()?;
            let items = (0..n)
                .map(|_| unpack_shaped(u, item))
                .collect::<Option<_>>()?;
            u.skip_to(end)?;
            items
        }
    }))
}

/// A record: its fields in order, as a map by their names; fields the shape
/// doesn't know skipped, missing ones left out.
fn record(u: &mut Unpacker<'_>, s: &StructShape) -> Option<Value> {
    let end = u.begin()?;
    let mut m = Map::new();
    for field in &s.fields {
        if !u.more(end) {
            break;
        }
        m.insert(field.name, unpack_shaped(u, &(field.shape)())?);
    }
    u.leave(end)?;
    Some(Value::Map(m))
}

/// An enum of names: its variant's name, by index.
fn names(u: &mut Unpacker<'_>, e: &EnumShape) -> Option<Value> {
    let i = usize::try_from(u.uint()?).ok()?;
    Some(Value::Str(e.variants.get(i)?.name.to_owned()))
}
