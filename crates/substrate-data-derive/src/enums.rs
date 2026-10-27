//! Enums: a tagged one (`#[data(tag = "type")]`, its variants by name
//! under the tag, fields or a wrapped struct alongside, any other wrapped
//! value under `substrate_data::schema::WRAPPED`) or one of names.

use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{Fields, Ident, Type};

use crate::attrs::{Attrs, attrs};
use crate::case::variant_key;
use crate::codegen::Codegen;
use crate::docs::docs;
use crate::fields::{NamedField, field_shapes, named_fields, read_fields, write_fields};
use crate::marker::Marker;

/// An enum of unit variants (`names`: each one's ident and key): a string.
pub(crate) fn expand_unit_enum(
    cg: &Codegen,
    names: &[(&Ident, &String)],
    shape: &Tokens,
) -> Tokens {
    let frame = unit_frame(cg, names);
    let value = unit_value(cg, names);
    quote! { #shape #frame #value }
}

/// A unit enum in a frame: packed by position (variants are only ever appended).
fn unit_frame(cg: &Codegen, names: &[(&Ident, &String)]) -> Tokens {
    let krate = &cg.krate;
    let indices: Vec<_> = (0..names.len() as u64).collect();
    let numbers = names
        .iter()
        .zip(&indices)
        .map(|((ident, _), i)| quote! { Self::#ident => #i, });
    let by_number = names
        .iter()
        .zip(&indices)
        .map(|((ident, _), i)| quote! { #i => ::std::option::Option::Some(Self::#ident), });
    quote! {
        fn pack(&self, p: &mut #krate::frame::Packer) {
            p.put_u(match self { #(#numbers)* });
        }
        fn unpack(u: &mut #krate::frame::Unpacker<'_>) -> ::std::option::Option<Self> {
            match u.uint()? {
                #(#by_number)*
                _ => ::std::option::Option::None,
            }
        }
    }
}

/// A unit enum as a value and in the binary form: its variant's name.
fn unit_value(cg: &Codegen, names: &[(&Ident, &String)]) -> Tokens {
    let krate = &cg.krate;
    let keys = names.iter().map(|(_, key)| key);
    let reads: Vec<_> = names
        .iter()
        .map(|(ident, key)| quote! { #key => ::std::option::Option::Some(Self::#ident), })
        .collect();
    let writes: Vec<_> = names
        .iter()
        .map(|(ident, key)| quote! { Self::#ident => #key, })
        .collect();
    quote! {
        fn read(v: &#krate::ValueRef<'_>, cx: &mut #krate::Reader<'_>) -> ::std::option::Option<Self> {
            let #krate::ValueRef::Str(s) = v else {
                cx.expected(#krate::schema::Expected::Name, v);
                return ::std::option::Option::None;
            };
            match *s {
                #(#reads)*
                other => {
                    cx.unknown_name(other, &[#(#keys),*]);
                    ::std::option::Option::None
                }
            }
        }
        fn write(&self) -> #krate::Value {
            #krate::Value::Str(::std::string::String::from(match self { #(#writes)* }))
        }
        fn encode(&self, w: &mut #krate::binary::Writer) {
            w.str(match self { #(#writes)* });
        }
        fn write_text(&self, w: &mut #krate::text::Writer) {
            w.str(match self { #(#writes)* });
        }
        fn decode(d: &mut #krate::binary::Decoder<'_>) -> ::std::option::Option<Self> {
            match d.str()? {
                #(#reads)*
                _ => ::std::option::Option::None,
            }
        }
    }
}

/// A variant as the derive sees it.
struct Variant {
    ident: Ident,
    key: String,
    doc: String,
    kind: VariantKind,
    other: bool,
}

enum VariantKind {
    Unit,
    Named(Vec<NamedField>),
    Wrap(Box<Type>),
}

pub(crate) fn expand_enum(
    cg: &Codegen,
    e: &syn::DataEnum,
    top: &Attrs,
    shape_name: &str,
    doc: &str,
) -> syn::Result<Tokens> {
    let mut vs = variants(cg, e, top)?;
    // The catch-all variant, if any: not a name of its own.
    let other = vs.iter().position(|v| v.other).map(|i| vs.remove(i));
    let shape = enum_shape(cg, &vs, top.tag.as_deref(), (shape_name, doc));
    let Some(tag) = &top.tag else {
        let names: Vec<(&Ident, &String)> = vs.iter().map(|v| (&v.ident, &v.key)).collect();
        return Ok(expand_unit_enum(cg, &names, &shape));
    };
    Ok(tagged(cg, &vs, other.as_ref(), tag, &shape))
}

/// The enum's variants, checked: fields need a tag, `other` wraps a type.
fn variants(cg: &Codegen, e: &syn::DataEnum, top: &Attrs) -> syn::Result<Vec<Variant>> {
    let mut vs = Vec::new();
    for v in &e.variants {
        let a = attrs(&v.attrs)?;
        let kind = match &v.fields {
            Fields::Unit => VariantKind::Unit,
            Fields::Named(n) => VariantKind::Named(named_fields(&cg.fields_as_written(), n)?),
            Fields::Unnamed(u) if u.unnamed.len() == 1 => {
                VariantKind::Wrap(Box::new(u.unnamed[0].ty.clone()))
            }
            Fields::Unnamed(_) => {
                return Err(syn::Error::new_spanned(
                    &v.ident,
                    "Schema: a variant wraps one struct at most",
                ));
            }
        };
        if a.has(Marker::Other) && (top.tag.is_none() || !matches!(kind, VariantKind::Wrap(_))) {
            return Err(syn::Error::new_spanned(
                &v.ident,
                "Schema: `other` is a tagged enum's variant wrapping a type that reads the whole object, tag included",
            ));
        }
        let other = a.has(Marker::Other);
        vs.push(Variant {
            key: a.rename.unwrap_or_else(|| variant_key(&v.ident, cg.case)),
            ident: v.ident.clone(),
            doc: docs(&v.attrs),
            kind,
            other,
        });
    }
    Ok(vs)
}

/// The enum's `shape()`.
fn enum_shape(
    cg: &Codegen,
    vs: &[Variant],
    tag: Option<&str>,
    (shape_name, doc): (&str, &str),
) -> Tokens {
    let krate = &cg.krate;
    let variant_shapes = vs.iter().map(|v| {
        let (key, variant_doc) = (&v.key, &v.doc);
        let fields = match &v.kind {
            VariantKind::Unit => quote! { ::std::vec::Vec::new() },
            VariantKind::Named(f) => field_shapes(cg, f),
            VariantKind::Wrap(ty) => quote! {
                match <#ty as #krate::Schema>::shape() {
                    #krate::Shape::Struct(s) => s.fields,
                    _ => ::std::vec![#krate::schema::Field {
                        name: #krate::schema::WRAPPED,
                        doc: "",
                        shape: <#ty as #krate::Schema>::shape,
                        required: <#ty as #krate::Schema>::absent().is_none(),
                        range: ::std::option::Option::None,
                    }],
                }
            },
        };
        quote! { #krate::schema::Variant { name: #key, doc: #variant_doc, fields: #fields } }
    });
    let tag_tokens = if let Some(t) = tag {
        quote! { ::std::option::Option::Some(#t) }
    } else {
        quote! { ::std::option::Option::None }
    };
    quote! {
        fn shape() -> #krate::Shape {
            #krate::Shape::Enum(#krate::schema::EnumShape {
                name: #shape_name,
                doc: #doc,
                tag: #tag_tokens,
                variants: ::std::vec![#(#variant_shapes),*],
            })
        }
    }
}

/// A tagged enum's `read` and `write`: the tag names the variant.
fn tagged(
    cg: &Codegen,
    vs: &[Variant],
    other: Option<&Variant>,
    tag: &str,
    shape: &Tokens,
) -> Tokens {
    let krate = &cg.krate;
    let keys: Vec<&String> = vs.iter().map(|v| &v.key).collect();
    let reads = vs.iter().map(|v| tagged_read(cg, v, tag));
    let writes = vs.iter().map(|v| tagged_write(cg, v, tag));
    let (fallback, other_write) = match other {
        Some(Variant {
            ident,
            kind: VariantKind::Wrap(ty),
            ..
        }) => (
            quote! { _ => <#ty as #krate::Schema>::read(v, cx).map(Self::#ident), },
            quote! { Self::#ident(inner) => { m = match #krate::Schema::write(inner) { #krate::Value::Map(fields) => fields, _ => m }; } },
        ),
        _ => (
            quote! {
                other => {
                    cx.at(#krate::Seg::Key(::std::string::String::from(#tag)), |cx| cx.unknown_name(other, &[#(#keys),*]));
                    ::std::option::Option::None
                }
            },
            quote! {},
        ),
    };
    quote! {
        #shape
        fn read(v: &#krate::ValueRef<'_>, cx: &mut #krate::Reader<'_>) -> ::std::option::Option<Self> {
            let map = cx.map(v)?;
            let ::std::option::Option::Some(kind) = map.get(#tag) else {
                cx.problem_of(#krate::Problem::MissingField, ::std::format!("missing `{}` ({})", #tag, [#(#keys),*].map(|k| ::std::format!("`{k}`")).join(", ")));
                return ::std::option::Option::None;
            };
            let #krate::ValueRef::Str(kind) = kind else {
                cx.at(#krate::Seg::Key(::std::string::String::from(#tag)), |cx| cx.expected(#krate::schema::Expected::Name, kind));
                return ::std::option::Option::None;
            };
            match *kind {
                #(#reads)*
                #fallback
            }
        }
        fn write(&self) -> #krate::Value {
            let mut m = #krate::Map::new();
            match self { #(#writes)* #other_write }
            #krate::Value::Map(m)
        }
    }
}

/// A variant read under its tag's name.
fn tagged_read(cg: &Codegen, v: &Variant, tag: &str) -> Tokens {
    let krate = &cg.krate;
    let (ident, key) = (&v.ident, &v.key);
    let body = match &v.kind {
        VariantKind::Unit => quote! {
            cx.with_allowed(#tag, |cx| cx.unknown_keys(map, &[]));
            ::std::option::Option::Some(Self::#ident)
        },
        VariantKind::Named(fields) => {
            let read = read_fields(fields, &quote! { Self::#ident });
            quote! { cx.with_allowed(#tag, |cx| { #read }) }
        }
        VariantKind::Wrap(ty) => quote! {
            if #krate::schema::flattens::<#ty>() {
                cx.with_allowed(#tag, |cx| <#ty as #krate::Schema>::read(v, cx)).map(Self::#ident)
            } else {
                cx.with_allowed(#tag, |cx| {
                    cx.unknown_keys(map, &[#krate::schema::WRAPPED]);
                    cx.own_keys(|cx| cx.field_with::<#krate::with::Plain, #ty>(map, #krate::schema::WRAPPED))
                })
                .map(Self::#ident)
            }
        },
    };
    quote! { #key => { #body } }
}

/// A variant written with its tag's name.
fn tagged_write(cg: &Codegen, v: &Variant, tag: &str) -> Tokens {
    let krate = &cg.krate;
    let (ident, key) = (&v.ident, &v.key);
    match &v.kind {
        VariantKind::Unit => quote! {
            Self::#ident => {
                m.insert(#tag, #krate::Value::from(#key));
            }
        },
        VariantKind::Named(fields) => {
            let bindings: Vec<_> = fields.iter().map(NamedField::binding).collect();
            let puts = write_fields(cg, fields);
            quote! {
                Self::#ident { #(#bindings),* } => {
                    m.insert(#tag, #krate::Value::from(#key));
                    #puts
                }
            }
        }
        VariantKind::Wrap(ty) => quote! {
            Self::#ident(inner) => {
                m.insert(#tag, #krate::Value::from(#key));
                match #krate::Schema::write(inner) {
                    #krate::Value::Map(fields) if #krate::schema::flattens::<#ty>() => {
                        for (k, v) in fields {
                            m.insert(k, v);
                        }
                    }
                    value => {
                        if !#krate::Schema::skip(inner) {
                            m.insert(#krate::schema::WRAPPED, value);
                        }
                    }
                }
            }
        },
    }
}
