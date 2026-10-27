//! Fields in the value form: their shapes, read out of a map, written into one.

use proc_macro2::TokenStream as Tokens;
use quote::quote;

use crate::codegen::Codegen;
use crate::fields::NamedField;
use crate::marker::Fallback;

/// `Vec<Field>` describing `fields`.
pub(crate) fn field_shapes(cg: &Codegen, fields: &[NamedField]) -> Tokens {
    let krate = &cg.krate;
    let items = fields.iter().filter(|f| !f.skip).map(|f| {
        let (key, doc, ty, w) = (&f.key, &f.doc, &f.ty, &f.with);
        let has_default = f.default.is_some();
        let range = if let Some((lo, hi)) = f.range {
            quote! { ::std::option::Option::Some((#lo, #hi)) }
        } else {
            quote! { ::std::option::Option::None }
        };
        quote! {
            #krate::schema::Field {
                name: #key,
                doc: #doc,
                shape: <#w as #krate::with::With<#ty>>::shape,
                required: !#has_default && <#w as #krate::with::With<#ty>>::absent().is_none(),
                range: #range,
            }
        }
    });
    quote! { ::std::vec![#(#items),*] }
}

/// Reads `fields` out of `map` (in scope) into `ctor { ... }`.
pub(crate) fn read_fields(fields: &[NamedField], ctor: &Tokens) -> Tokens {
    let keys = fields.iter().filter(|f| !f.skip).map(|f| &f.key);
    let lets = fields.iter().map(|f| {
        let (ident, ty, key, w) = (&f.ident, &f.ty, &f.key, &f.with);
        if f.skip {
            return quote! { let #ident: ::std::option::Option<#ty> = ::std::option::Option::Some(::std::default::Default::default()); };
        }
        match &f.default {
            None => quote! { let #ident = cx.field_with::<#w, #ty>(map, #key); },
            Some(Fallback::Default) => quote! { let #ident = cx.field_or_with::<#w, #ty>(map, #key, ::std::default::Default::default); },
            Some(Fallback::Call(path)) => quote! { let #ident = cx.field_or_with::<#w, #ty>(map, #key, #path); },
        }
    });
    let ranges = fields.iter().filter_map(|f| {
        let (ident, key) = (&f.ident, &f.key);
        let (lo, hi) = f.range?;
        Some(quote! { let #ident = cx.in_range(#key, #ident, #lo, #hi); })
    });
    let inits = fields.iter().map(|f| {
        let ident = &f.ident;
        quote! { #ident: #ident? }
    });
    quote! {
        cx.unknown_keys(map, &[#(#keys),*]);
        cx.own_keys(|cx| {
            #(#lets)*
            #(#ranges)*
            ::std::option::Option::Some(#ctor { #(#inits),* })
        })
    }
}

/// Inserts `fields` (bound by name) into `m` (in scope).
pub(crate) fn write_fields(cg: &Codegen, fields: &[NamedField]) -> Tokens {
    let krate = &cg.krate;
    let puts = fields.iter().filter(|f| !f.skip).map(|f| {
        let (ident, key, ty, w) = (&f.ident, &f.key, &f.ty, &f.with);
        quote! {
            if !<#w as #krate::with::With<#ty>>::skip(#ident) {
                m.insert(#key, <#w as #krate::with::With<#ty>>::write(#ident));
            }
        }
    });
    quote! { #(#puts)* }
}
