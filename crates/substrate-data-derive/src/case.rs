//! [`Case`]: how names are written (`rename_all`), and the conversions.

use syn::{Ident, LitStr};

use crate::attr_name;

/// `rename_all`'s case: fields (a struct's) or variants (an enum's) in
/// lower camel case. The one case there is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Case {
    /// `statBar`.
    Camel,
}

impl Case {
    /// The case `lit` names.
    pub(crate) fn parse(lit: &LitStr) -> syn::Result<Case> {
        if lit.value() == attr_name::CAMEL_CASE {
            Ok(Case::Camel)
        } else {
            Err(syn::Error::new_spanned(
                lit,
                "rename_all = \"camelCase\" is the one case",
            ))
        }
    }
}

/// A field's name as written: as in Rust, or in `case`.
pub(crate) fn field_key(ident: &Ident, case: Option<Case>) -> String {
    let name = ident.to_string().trim_start_matches("r#").to_string();
    match case {
        None => name,
        Some(Case::Camel) => camel(&name),
    }
}

/// A variant's name as written: in snake case, or in `case`.
pub(crate) fn variant_key(ident: &Ident, case: Option<Case>) -> String {
    let name = ident.to_string();
    match case {
        None => snake(&name),
        Some(Case::Camel) => camel(&name),
    }
}

/// The words of a name: split at `_` and before each capital.
fn words(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in name.split('_').filter(|p| !p.is_empty()) {
        let mut start = 0;
        for (i, c) in part.char_indices().skip(1) {
            if c.is_uppercase() {
                out.push(part[start..i].to_string());
                start = i;
            }
        }
        out.push(part[start..].to_string());
    }
    out
}

/// `stat_bar` / `StatBar` → `statBar`.
fn camel(name: &str) -> String {
    let mut out = String::new();
    for (i, w) in words(name).iter().enumerate() {
        let mut chars = w.chars();
        if let Some(first) = chars.next() {
            if i == 0 {
                out.extend(first.to_lowercase());
            } else {
                out.extend(first.to_uppercase());
            }
            out.push_str(chars.as_str());
        }
    }
    out
}

/// `StatBar` → `stat_bar`.
fn snake(name: &str) -> String {
    words(name)
        .iter()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join("_")
}
