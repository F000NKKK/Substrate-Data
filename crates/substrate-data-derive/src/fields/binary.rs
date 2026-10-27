//! Fields in the binary form, written and read straight.

use proc_macro2::TokenStream as Tokens;
use quote::quote;

use crate::codegen::Codegen;
use crate::fields::NamedField;
use crate::marker::Fallback;

/// `fields` (bound by name) written straight as a map.
pub(crate) fn encode_fields(cg: &Codegen, fields: &[NamedField]) -> Tokens {
    let krate = &cg.krate;
    let written: Vec<_> = fields.iter().filter(|f| !f.skip).collect();
    let counts = written.iter().map(|f| {
        let (ident, ty, w) = (&f.ident, &f.ty, &f.with);
        quote! { + usize::from(!<#w as #krate::with::With<#ty>>::skip(#ident)) }
    });
    let puts = written.iter().map(|f| {
        let (ident, key, ty, w) = (&f.ident, &f.key, &f.ty, &f.with);
        quote! {
            if !<#w as #krate::with::With<#ty>>::skip(#ident) {
                __sd_w.key(#key);
                <#w as #krate::with::With<#ty>>::encode(#ident, __sd_w);
            }
        }
    });
    quote! {
        __sd_w.map(0 #(#counts)*);
        #(#puts)*
    }
}

/// `ctor { ... }` read straight from a map — `None` (the slow way) for a
/// key it doesn't know, or a field with a range to check.
pub(crate) fn decode_fields(cg: &Codegen, fields: &[NamedField], ctor: &Tokens) -> Option<Tokens> {
    let krate = &cg.krate;
    if fields.iter().any(|f| f.range.is_some()) {
        return None;
    }
    let read: Vec<_> = fields.iter().filter(|f| !f.skip).collect();
    // Each field's slot, named apart from anything a default may call.
    let slot = |f: &NamedField| quote::format_ident!("__sd_field_{}", f.ident);
    let slots = read.iter().map(|f| {
        let (s, ty) = (slot(f), &f.ty);
        quote! { let mut #s: ::std::option::Option<#ty> = ::std::option::Option::None; }
    });
    let arms = read.iter().map(|f| {
        let (s, key, ty, w) = (slot(f), &f.key, &f.ty, &f.with);
        quote! { #key => #s = ::std::option::Option::Some(<#w as #krate::with::With<#ty>>::decode(__sd_d)?), }
    });
    let inits = fields.iter().map(|f| {
        let (ident, ty, w) = (&f.ident, &f.ty, &f.with);
        let value = if f.skip {
            quote! { ::std::default::Default::default() }
        } else {
            let missing = match &f.default {
                None => quote! { <#w as #krate::with::With<#ty>>::absent()? },
                Some(Fallback::Default) => quote! { ::std::default::Default::default() },
                Some(Fallback::Call(path)) => quote! { #path() },
            };
            let s = slot(f);
            quote! { match #s { ::std::option::Option::Some(__sd_v) => __sd_v, ::std::option::Option::None => #missing } }
        };
        quote! { #ident: #value }
    });
    Some(quote! {
        let __sd_n = __sd_d.map()?;
        #(#slots)*
        for _ in 0..__sd_n {
            match __sd_d.key()? {
                #(#arms)*
                _ => return ::std::option::Option::None,
            }
        }
        __sd_d.leave();
        ::std::option::Option::Some(#ctor { #(#inits),* })
    })
}
