//! References read: of the kinds a field takes, their ids as the kinds' id type.

use super::{Kinds, RefId};
use crate::diagnostic::Problem;
use crate::model::borrowed::ValueRef;
use crate::model::value::{KindName, Ref, RefKey};
use crate::schema::{Expected, Reader, Schema, Shape};
use crate::typed;

/// `v` as a reference to one of `kinds` (any kind if empty), written
/// `@kind:id` — a bare id is an error that says how to write it. The id
/// is read untyped ([`read_id`] reads it as a kind's id type).
pub fn read_ref(v: &ValueRef<'_>, cx: &mut Reader<'_>, kinds: &[&str]) -> Option<Ref> {
    let r = match v {
        ValueRef::Ref { kind, id } => Ref::new(*kind, RefKey::spelled(id)),
        ValueRef::Str(s) if s.starts_with(crate::names::TYPED) => spelled_ref(cx, s, kinds)?,
        ValueRef::Str(s) => {
            let kind = kinds.first().copied().unwrap_or("kind");
            cx.problem_of(
                Problem::NotAReference,
                format!("`{s}`: name it by a reference — `@{kind}:{s}`"),
            );
            return None;
        }
        other => {
            cx.expected(Expected::Reference, other);
            return None;
        }
    };
    if !kinds.is_empty() && !kinds.contains(&r.kind.as_str()) {
        cx.problem_of(
            Problem::WrongKind,
            format!(
                "`{r}` is a reference to {} — this takes {}",
                r.kind,
                wanted(kinds)
            ),
        );
        return None;
    }
    cx.note_ref(r.clone());
    Some(r)
}

/// The kinds a reference may be of, as they're written: "`@user:` or `@group:`".
fn wanted(kinds: &[&str]) -> String {
    if kinds.is_empty() {
        return "`@kind:id`".into();
    }
    kinds
        .iter()
        .map(|k| format!("`@{k}:`"))
        .collect::<Vec<_>>()
        .join(" or ")
}

/// A reference spelled in a string (`"@kind:id"`), or a problem.
fn spelled_ref(cx: &mut Reader<'_>, s: &str, kinds: &[&str]) -> Option<Ref> {
    match typed::classify(s) {
        Ok(typed::Spelled::Ref(r)) => Some(r),
        Ok(other) => {
            cx.problem_of(
                Problem::Expected,
                format!(
                    "expected a reference ({}), found {}",
                    wanted(kinds),
                    other.kind()
                ),
            );
            None
        }
        Err(e) => {
            cx.problem_of(Problem::BadTyped, e.to_string());
            None
        }
    }
}

/// `id` (of a reference of `kind`) as an `I`, or a problem.
fn parse_id<I: RefId>(cx: &mut Reader<'_>, kind: KindName, id: &str) -> Option<I> {
    I::parse(id).or_else(|| {
        cx.problem_of(
            Problem::BadId,
            format!(
                "`@{kind}:{id}`: `{id}` isn't a {kind} id (expected {})",
                I::EXPECTED
            ),
        );
        None
    })
}

/// `v` as a reference to one of `K`'s kinds, its id read as `K::Id` — as
/// written, or as the [`crate::Resolve`] rewrote it.
pub fn read_id<K: Kinds>(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<K::Id> {
    // The usual case, a reference of a kind it takes, nothing kept or resolved.
    if let ValueRef::Ref { kind, id } = v
        && K::NAMES.contains(&kind.as_str())
        && !cx.keeps_found()
        && !cx.resolves()
    {
        return parse_id(cx, *kind, id);
    }
    let r = read_ref(v, cx, K::NAMES)?;
    let id = parse_id::<K::Id>(cx, r.kind, r.id.as_str())?;
    if !cx.resolves() {
        return Some(id);
    }
    let kind = r.kind;
    let key = cx.take_ref(r)?;
    parse_id(cx, kind, key.as_str())
}

/// The shape of a reference to `K`.
#[must_use]
pub fn ref_shape<K: Kinds>() -> Shape {
    Shape::Ref {
        kinds: K::NAMES,
        id: <K::Id as Schema>::shape,
    }
}
