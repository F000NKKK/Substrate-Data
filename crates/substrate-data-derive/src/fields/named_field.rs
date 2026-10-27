//! [`NamedField`]: a struct's (or variant's) named field, its attributes read.

use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{Ident, Type};

use crate::attrs::attrs;
use crate::case::field_key;
use crate::codegen::Codegen;
use crate::docs::docs;
use crate::marker::{Fallback, Marker};

/// A named field as the derive sees it.
pub(crate) struct NamedField {
    /// Its Rust name.
    pub(crate) ident: Ident,
    /// Its type.
    pub(crate) ty: Type,
    /// Its key as written.
    pub(crate) key: String,
    /// Its docs.
    pub(crate) doc: String,
    /// See [`crate::attrs::Attrs`]'s.
    pub(crate) default: Option<Fallback>,
    /// Never read or written.
    pub(crate) skip: bool,
    /// The codec: `with = "…"`, else `Plain` (the type's own schema).
    pub(crate) with: Type,
    /// Whether `with` names a codec (else it's the type's own schema).
    pub(crate) codec: bool,
    /// The numbers it takes.
    pub(crate) range: Option<(f64, f64)>,
}

impl NamedField {
    /// Its pattern when a value is taken apart: `ident`, or `ident: _` when
    /// skipped, so the generated code never binds a field it doesn't use.
    pub(crate) fn binding(&self) -> Tokens {
        let ident = &self.ident;
        if self.skip {
            quote! { #ident: _ }
        } else {
            quote! { #ident }
        }
    }
}

/// `fields` as the derive sees them.
pub(crate) fn named_fields(
    cg: &Codegen,
    fields: &syn::FieldsNamed,
) -> syn::Result<Vec<NamedField>> {
    let krate = &cg.krate;
    fields
        .named
        .iter()
        .map(|f| {
            let a = attrs(&f.attrs)?;
            let Some(ident) = f.ident.clone() else {
                return Err(syn::Error::new_spanned(
                    f,
                    "Schema: a named field without a name",
                ));
            };
            let skip = a.has(Marker::Skip);
            let key = a.rename.unwrap_or_else(|| field_key(&ident, cg.case));
            Ok(NamedField {
                ident,
                ty: f.ty.clone(),
                key,
                doc: docs(&f.attrs),
                skip,
                default: a.default,
                codec: a.with.is_some(),
                with: a
                    .with
                    .unwrap_or_else(|| syn::parse_quote!(#krate::with::Plain)),
                range: a.range,
            })
        })
        .collect()
}
