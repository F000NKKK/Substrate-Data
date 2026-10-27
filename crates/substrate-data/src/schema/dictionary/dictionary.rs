//! [`Dictionary`]: a set of values by kind and id, a resolver that expands
//! references to them.

use std::collections::BTreeMap;

use super::entries::Entries;
use crate::model::borrowed::ValueRef;
use crate::model::value::{KindName, Map, Path, Ref, RefKey, Seg, Value};
use crate::schema::{Reader, Resolve, Schema, Shape, Unresolved, is_metadata_key};

/// The values references expand to — the words of a vocabulary, the texts
/// of a language, the entries of a table — kept per kind and id, in the
/// order they were added. It is a [`Resolve`]: a reference of a kind it
/// holds is checked against its ids and, where a field takes a value
/// ([`with::Expand`](crate::with::Expand)), expanded to the value; a
/// reference of a kind it doesn't hold is left alone.
///
/// It is a [`Schema`] too, written `{ <kind>: { <id>: <value>, … }, … }`, so
/// a tool loads it from text, a value or the binary form like any
/// document (an empty kind, `{ word: {} }`, holds nothing and says every
/// `@word:…` is missing). Keys that are notes (`$…`, `_…`, `comment`) are
/// skipped, as in any object.
///
/// ```
/// use substrate_data::schema::{Dictionary, from_text, from_value_with};
/// use substrate_data::{Ref, Value};
///
/// let mut dictionary: Dictionary =
///     from_text(r#"{ project: { alpha: { name: "Alpha" } } }"#).unwrap().value;
/// let alpha = Ref::parse("project", "alpha").unwrap();
/// assert_eq!(dictionary.get(&alpha).unwrap().get("name").unwrap().as_str(), Some("Alpha"));
///
/// // As a resolver, it expands what a field reads through `Expand`.
/// #[derive(substrate_data::Schema)]
/// struct Card {
///     #[data(with = "substrate_data::with::Expand<substrate_data::with::Plain>")]
///     project: Value,
/// }
/// let doc: Value = from_text(r#"{ project: "@project:alpha" }"#).unwrap().value;
/// let card = from_value_with::<Card>(&doc, &mut dictionary).unwrap().value;
/// assert_eq!(card.project.get("name").unwrap().as_str(), Some("Alpha"));
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dictionary {
    kinds: BTreeMap<KindName, Entries>,
}

impl Dictionary {
    /// A dictionary holding nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `value` as the value of `id` of `kind`, returning the one it
    /// replaced.
    pub fn insert(&mut self, kind: KindName, id: RefKey, value: Value) -> Option<Value> {
        self.kinds.entry(kind).or_default().insert(id, value)
    }

    /// Holds `kind` (with no values if there are none): its references are
    /// now checked, and one to an id there isn't is missing.
    pub fn declare(&mut self, kind: KindName) {
        self.kinds.entry(kind).or_default();
    }

    /// Whether the dictionary holds `kind`.
    #[must_use]
    pub fn holds(&self, kind: KindName) -> bool {
        self.kinds.contains_key(&kind)
    }

    /// The value `r` names.
    #[must_use]
    pub fn get(&self, r: &Ref) -> Option<&Value> {
        self.kinds.get(&r.kind)?.get(&r.id)
    }

    /// The kinds held, by name.
    pub fn kinds(&self) -> impl Iterator<Item = KindName> + '_ {
        self.kinds.keys().copied()
    }

    /// `kind`'s values in the order they were added (none if it isn't held).
    pub fn entries(&self, kind: KindName) -> impl Iterator<Item = (&RefKey, &Value)> {
        self.kinds.get(&kind).into_iter().flat_map(Entries::iter)
    }

    /// How many values it holds, all kinds together.
    #[must_use]
    pub fn len(&self) -> usize {
        self.kinds.values().map(Entries::len).sum()
    }

    /// Whether it holds no value.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Resolve for Dictionary {
    fn reference(&mut self, r: &Ref, _path: &Path) -> Result<RefKey, Unresolved> {
        match self.kinds.get(&r.kind) {
            Some(entries) if entries.get(&r.id).is_none() => Err(Unresolved::Missing(r.clone())),
            _ => Ok(r.id.clone()),
        }
    }

    fn expand(&mut self, r: &Ref, _path: &Path) -> Result<Option<Value>, Unresolved> {
        let Some(entries) = self.kinds.get(&r.kind) else {
            return Ok(None);
        };
        entries
            .get(&r.id)
            .cloned()
            .map(Some)
            .ok_or_else(|| Unresolved::Missing(r.clone()))
    }
}

/// A kind's object read into `dictionary`; `None` if it has problems.
fn read_kind(
    dictionary: &mut Dictionary,
    kind: &str,
    v: &ValueRef<'_>,
    cx: &mut Reader<'_>,
) -> Option<()> {
    let kind = KindName::new(kind)
        .map_err(|e| cx.problem_of(crate::Problem::BadTyped, e.to_string()))
        .ok()?;
    let entries = cx.map(v)?;
    dictionary.declare(kind);
    let mut ok = true;
    for (id, value) in entries.iter().filter(|(id, _)| !is_metadata_key(id)) {
        let read = cx.at(Seg::Key(id.to_string()), |cx| {
            RefKey::new(id)
                .map_err(|e| cx.problem_of(crate::Problem::BadId, e.to_string()))
                .ok()
        });
        match read {
            Some(key) => {
                dictionary.insert(kind, key, value.to_value());
            }
            None => ok = false,
        }
    }
    ok.then_some(())
}

fn words() -> Shape {
    Shape::Map(<Value as Schema>::shape)
}

impl Schema for Dictionary {
    fn shape() -> Shape {
        Shape::Map(words)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        let kinds = cx.map(v)?;
        let mut out = Dictionary::new();
        let mut ok = true;
        for (kind, entries) in kinds.iter().filter(|(k, _)| !is_metadata_key(k)) {
            let read = cx.at(Seg::Key(kind.to_string()), |cx| {
                read_kind(&mut out, kind, entries, cx)
            });
            ok &= read.is_some();
        }
        ok.then_some(out)
    }
    fn write(&self) -> Value {
        let kinds = self.kinds.iter().map(|(kind, entries)| {
            let values: Map = entries
                .iter()
                .map(|(id, v)| (id.as_str().to_owned(), v.clone()))
                .collect();
            (kind.as_str().to_owned(), Value::Map(values))
        });
        Value::Map(kinds.collect())
    }
}
