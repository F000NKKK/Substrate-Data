//! Fields in the binary form, written and read straight.

use proc_macro2::TokenStream as Tokens;
use quote::quote;

use crate::codegen::Codegen;
use crate::fields::NamedField;
use crate::marker::Fallback;

/// Most fields a struct's keys are found again by a bit each ([`encode_fields`]).
const MASKED_MAX: usize = 64;

/// `fields` (bound by name) written straight as a map: its keys as a
/// shape (written once, named after), then each value.
pub(crate) fn encode_fields(cg: &Codegen, fields: &[NamedField]) -> Tokens {
    let krate = &cg.krate;
    let written: Vec<_> = fields.iter().filter(|f| !f.skip).collect();
    if written.is_empty() {
        return quote! { __sd_w.map(0); };
    }
    let puts = written.iter().map(|f| {
        let (ident, ty, w) = (&f.ident, &f.ty, &f.with);
        quote! {
            if !<#w as #krate::with::With<#ty>>::skip(#ident) {
                <#w as #krate::with::With<#ty>>::encode(#ident, __sd_w);
            }
        }
    });
    let record = if written.len() <= MASKED_MAX {
        masked_record(cg, &written)
    } else {
        listed_record(cg, &written)
    };
    quote! {
        #record
        #(#puts)*
    }
}

/// The keys the fields have, as a static slice and a bit for each one present.
fn masked_record(cg: &Codegen, written: &[&NamedField]) -> Tokens {
    let krate = &cg.krate;
    let keys = written.iter().map(|f| &f.key);
    let bits = written.iter().enumerate().map(|(i, f)| {
        let (ident, ty, w) = (&f.ident, &f.ty, &f.with);
        quote! {
            if !<#w as #krate::with::With<#ty>>::skip(#ident) {
                __sd_present |= 1u64 << #i;
            }
        }
    });
    let len = written.len();
    quote! {
        static __SD_KEYS: [&str; #len] = [#(#keys),*];
        let mut __sd_present = 0u64;
        #(#bits)*
        __sd_w.record_masked(&__SD_KEYS, __sd_present);
    }
}

/// The keys the fields have, listed on the stack, for a struct with too many fields to number.
fn listed_record(cg: &Codegen, written: &[&NamedField]) -> Tokens {
    let krate = &cg.krate;
    let capacity = written.len();
    let keys = written.iter().map(|f| {
        let (ident, key, ty, w) = (&f.ident, &f.key, &f.ty, &f.with);
        quote! {
            if !<#w as #krate::with::With<#ty>>::skip(#ident) {
                __sd_keys[__sd_n] = #key;
                __sd_n += 1;
            }
        }
    });
    quote! {
        let mut __sd_keys: [&str; #capacity] = [""; #capacity];
        let mut __sd_n = 0usize;
        #(#keys)*
        __sd_w.record(&__sd_keys[..__sd_n]);
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
