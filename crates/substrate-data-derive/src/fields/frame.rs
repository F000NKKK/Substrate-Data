//! Fields in a frame: packed in order, unpacked with defaults for the missing.

use proc_macro2::TokenStream as Tokens;
use quote::quote;

use crate::codegen::Codegen;
use crate::fields::NamedField;
use crate::marker::Fallback;

/// `fields` (bound by name) packed into a frame: a record, every field in order.
pub(crate) fn pack_fields(cg: &Codegen, fields: &[NamedField]) -> Tokens {
    let krate = &cg.krate;
    let puts = fields.iter().filter(|f| !f.skip).map(|f| {
        let (ident, ty, w) = (&f.ident, &f.ty, &f.with);
        quote! { <#w as #krate::with::With<#ty>>::pack(#ident, __sd_p); }
    });
    quote! {
        let __sd_at = __sd_p.begin();
        #(#puts)*
        __sd_p.end(__sd_at);
    }
}

/// `ctor { ... }` out of a frame's record: the fields it has in order, the
/// rest their defaults; fields it has that this type doesn't know skipped.
/// A frame comes from a writer of the same schema: ranges aren't checked again.
pub(crate) fn unpack_fields(cg: &Codegen, fields: &[NamedField], ctor: &Tokens) -> Tokens {
    let krate = &cg.krate;
    let slot = |f: &NamedField| quote::format_ident!("__sd_field_{}", f.ident);
    let gets = fields.iter().filter(|f| !f.skip).map(|f| {
        let (s, ty, w) = (slot(f), &f.ty, &f.with);
        quote! {
            let #s: ::std::option::Option<#ty> = if __sd_u.more(__sd_end) {
                ::std::option::Option::Some(<#w as #krate::with::With<#ty>>::unpack(__sd_u)?)
            } else {
                ::std::option::Option::None
            };
        }
    });
    let inits = fields.iter().map(|f| {
        let (ident, ty, w) = (&f.ident, &f.ty, &f.with);
        let value = if f.skip {
            quote! { ::std::default::Default::default() }
        } else {
            let s = slot(f);
            let missing = match &f.default {
                None => quote! { <#w as #krate::with::With<#ty>>::absent()? },
                Some(Fallback::Default) => quote! { ::std::default::Default::default() },
                Some(Fallback::Call(path)) => quote! { #path() },
            };
            quote! { match #s { ::std::option::Option::Some(__sd_v) => __sd_v, ::std::option::Option::None => #missing } }
        };
        quote! { #ident: #value }
    });
    quote! {
        let __sd_end = __sd_u.begin()?;
        #(#gets)*
        __sd_u.leave(__sd_end)?;
        ::std::option::Option::Some(#ctor { #(#inits),* })
    }
}
