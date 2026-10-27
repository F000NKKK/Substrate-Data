//! The derive's output for a type: its `Schema` impl, its view, its fields.

use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::attrs::{Attrs, attrs};
use crate::codegen::{Codegen, default_path};
use crate::docs::docs;
use crate::enums;
use crate::struct_schema::{expand_named, expand_newtype, fields_of};
use crate::view::view_of;

/// The derive's output for `input`: its container attributes read, the
/// code generated.
pub(crate) fn derive(input: &DeriveInput) -> syn::Result<Tokens> {
    let mut container = attrs(&input.attrs)?;
    // An enum with data in its variants is tagged, by `type` unless it says.
    if let Data::Enum(e) = &input.data
        && container.tag.is_none()
        && e.variants.iter().any(|v| !matches!(v.fields, Fields::Unit))
    {
        container.tag = Some(DEFAULT_TAG.to_owned());
    }
    let path = container.krate.take().unwrap_or_else(default_path);
    let cg = Codegen::new(&path, container.rename_all);
    expand(&cg, input, &container)
}

/// The tag of an enum with data that doesn't name one: what
/// `substrate_data::names::Word::Type` writes.
const DEFAULT_TAG: &str = "type";

/// The `Schema` impl (and view and fields) of `input`.
fn expand(cg: &Codegen, input: &DeriveInput, top: &Attrs) -> syn::Result<Tokens> {
    let krate = &cg.krate;
    let name = &input.ident;
    let (impl_g, ty_g, where_g) = input.generics.split_for_impl();
    let shape_name = top.rename.clone().unwrap_or_else(|| name.to_string());
    let doc = docs(&input.attrs);
    let body = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => expand_named(cg, named, &shape_name, &doc)?,
            Fields::Unnamed(u) if u.unnamed.len() == 1 => expand_newtype(cg, &u.unnamed[0].ty),
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "Schema: a struct needs named fields or exactly one unnamed field",
                ));
            }
        },
        Data::Enum(e) => enums::expand_enum(cg, e, top, &shape_name, &doc)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                name,
                "Schema: unions aren't supported",
            ));
        }
    };
    let view = view_of(cg, input, top)?;
    let fields = fields_of(cg, input)?;
    Ok(quote! {
        impl #impl_g #krate::Schema for #name #ty_g #where_g {
            #body
        }
        #view
        #fields
    })
}
