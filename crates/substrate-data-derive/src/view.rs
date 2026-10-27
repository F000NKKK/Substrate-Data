//! Frame views: a struct's `NameView<'a>` (`#[data(view)]`), an enum of
//! names' view of itself.

use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::attrs::Attrs;
use crate::codegen::Codegen;
use crate::fields::{NamedField, named_fields};
use crate::marker::Marker;

/// `NameView<'a>` and `Name`'s `View` (`#[data(view)]`): every field
/// viewed (a codec's field read whole), a field an older writer didn't
/// have making the view `None` — read the owned way then.
fn view_items(cg: &Codegen, input: &DeriveInput, fields: &[NamedField]) -> syn::Result<Tokens> {
    let krate = &cg.krate;
    let (name, vis) = (&input.ident, &input.vis);
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            name,
            "Schema: `view` is for a struct without generics",
        ));
    }
    let view = quote::format_ident!("{}View", name);
    let shown: Vec<&NamedField> = fields.iter().filter(|f| !f.skip).collect();
    let types = shown.iter().map(|f| {
        let (ident, ty) = (&f.ident, &f.ty);
        if f.codec {
            quote! { pub #ident: #ty, }
        } else {
            quote! { pub #ident: <#ty as #krate::frame::View<'a>>::View, }
        }
    });
    let reads = shown.iter().map(|f| {
        let (ident, ty, w) = (&f.ident, &f.ty, &f.with);
        let read = if f.codec {
            quote! { <#w as #krate::with::With<#ty>>::unpack(__sd_u)? }
        } else {
            quote! { <#ty as #krate::frame::View<'a>>::unpack_view(__sd_u)? }
        };
        quote! { let #ident = if __sd_u.more(__sd_end) { #read } else { return ::std::option::Option::None }; }
    });
    let idents = shown.iter().map(|f| &f.ident);
    let doc =
        format!("[`{name}`] viewed in a frame: strings borrowed, packed numbers read in place.");
    Ok(quote! {
        #[doc = #doc]
        #vis struct #view<'a> {
            #(#types)*
            _frame: ::std::marker::PhantomData<&'a ()>,
        }

        impl<'a> #krate::frame::View<'a> for #name {
            type View = #view<'a>;
            type ListView = #krate::frame::List<'a, Self>;
            fn unpack_view(__sd_u: &mut #krate::frame::Unpacker<'a>) -> ::std::option::Option<#view<'a>> {
                let __sd_end = __sd_u.begin()?;
                #(#reads)*
                __sd_u.leave(__sd_end)?;
                ::std::option::Option::Some(#view { #(#idents,)* _frame: ::std::marker::PhantomData })
            }
            fn unpack_list_view(__sd_u: &mut #krate::frame::Unpacker<'a>) -> ::std::option::Option<Self::ListView> {
                #krate::frame::list(__sd_u)
            }
        }
    })
}

/// The type's view of frames: a struct's (`#[data(view)]`), an enum of names' (itself).
pub(crate) fn view_of(cg: &Codegen, input: &DeriveInput, top: &Attrs) -> syn::Result<Tokens> {
    let krate = &cg.krate;
    let name = &input.ident;
    Ok(match (&input.data, top.has(Marker::View)) {
        (
            Data::Struct(syn::DataStruct {
                fields: Fields::Named(named),
                ..
            }),
            true,
        ) => view_items(cg, input, &named_fields(cg, named)?)?,
        (_, true) => {
            return Err(syn::Error::new_spanned(
                name,
                "Schema: `view` is for a struct with named fields",
            ));
        }
        (Data::Enum(e), false)
            if top.tag.is_none()
                && input.generics.params.is_empty()
                && e.variants.iter().all(|v| matches!(v.fields, Fields::Unit)) =>
        {
            quote! {
                impl<'a> #krate::frame::View<'a> for #name {
                    type View = Self;
                    type ListView = ::std::vec::Vec<Self>;
                    fn unpack_view(u: &mut #krate::frame::Unpacker<'a>) -> ::std::option::Option<Self> {
                        <Self as #krate::Schema>::unpack(u)
                    }
                    fn unpack_list_view(u: &mut #krate::frame::Unpacker<'a>) -> ::std::option::Option<::std::vec::Vec<Self>> {
                        <Self as #krate::Schema>::unpack_list(u)
                    }
                }
            }
        }
        _ => quote! {},
    })
}
