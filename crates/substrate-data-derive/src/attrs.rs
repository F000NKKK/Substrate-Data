//! [`Attrs`]: what `#[data(...)]` says, parsed.

use syn::{LitStr, Type};

use crate::attr_name;
use crate::case::Case;
use crate::marker::{Fallback, Marker};

/// What `#[data(...)]` says on a container, field or variant.
#[derive(Default)]
pub(crate) struct Attrs {
    /// Its key or name as written.
    pub(crate) rename: Option<String>,
    /// The container's case for its fields' or variants' names.
    pub(crate) rename_all: Option<Case>,
    /// Optional, and what it is when it isn't there.
    pub(crate) default: Option<Fallback>,
    /// The key naming a tagged enum's variant.
    pub(crate) tag: Option<String>,
    /// The field's codec.
    pub(crate) with: Option<Type>,
    /// The numbers the field takes.
    pub(crate) range: Option<(f64, f64)>,
    /// The container's path of `substrate_data` (a crate re-exporting it).
    pub(crate) krate: Option<syn::Path>,
    /// The options that take no value: `skip`, `other`, `view`.
    pub(crate) markers: Vec<Marker>,
}

impl Attrs {
    /// Whether it says `marker`.
    pub(crate) fn has(&self, marker: Marker) -> bool {
        self.markers.contains(&marker)
    }
}

/// What the `#[data(...)]` attributes of `list` say.
pub(crate) fn attrs(list: &[syn::Attribute]) -> syn::Result<Attrs> {
    let mut a = Attrs::default();
    for attr in list.iter().filter(|at| at.path().is_ident(attr_name::DATA)) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(attr_name::RENAME) {
                a.rename = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident(attr_name::RENAME_ALL) {
                a.rename_all = Some(Case::parse(&meta.value()?.parse::<LitStr>()?)?);
            } else if meta.path.is_ident(attr_name::TAG) {
                a.tag = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident(attr_name::DEFAULT) {
                a.default = Some(if meta.input.peek(syn::Token![=]) {
                    Fallback::Call(meta.value()?.parse::<LitStr>()?.parse()?)
                } else {
                    Fallback::Default
                });
            } else if meta.path.is_ident(attr_name::RANGE) {
                a.range = Some(parse_range(&meta.value()?.parse::<LitStr>()?)?);
            } else if meta.path.is_ident(attr_name::WITH) {
                a.with = Some(meta.value()?.parse::<LitStr>()?.parse()?);
            } else if meta.path.is_ident(attr_name::SKIP) {
                a.markers.push(Marker::Skip);
            } else if meta.path.is_ident(attr_name::VIEW) {
                a.markers.push(Marker::View);
            } else if meta.path.is_ident(attr_name::OTHER) {
                a.markers.push(Marker::Other);
            } else if meta.path.is_ident(attr_name::CRATE) {
                a.krate = Some(meta.value()?.parse::<LitStr>()?.parse()?);
            } else {
                return Err(meta
                    .error("unknown `data` attribute (rename, rename_all, default, skip, tag, with, range, other, crate, view)"));
            }
            Ok(())
        })?;
    }
    Ok(a)
}

/// `"lo..=hi"`: the numbers a field takes.
fn parse_range(lit: &LitStr) -> syn::Result<(f64, f64)> {
    let text = lit.value();
    let parsed = text.split_once("..=").and_then(|(lo, hi)| {
        Some((
            lo.trim().parse::<f64>().ok()?,
            hi.trim().parse::<f64>().ok()?,
        ))
    });
    parsed.ok_or_else(|| syn::Error::new_spanned(lit, "range = \"lo..=hi\" (numbers)"))
}
