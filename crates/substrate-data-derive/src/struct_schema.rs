//! Structs: one with named fields is an object, one with one unnamed field
//! is that field's format.

use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type};

use crate::codegen::Codegen;
use crate::fields::{
    decode_fields, encode_fields, field_shapes, named_fields, pack_fields, read_fields,
    unpack_fields, write_fields,
};

/// A struct with named fields: an object.
pub(crate) fn expand_named(
    cg: &Codegen,
    named: &syn::FieldsNamed,
    shape_name: &str,
    doc: &str,
) -> syn::Result<Tokens> {
    let krate = &cg.krate;
    let fields = named_fields(cg, named)?;
    let shapes = field_shapes(cg, &fields);
    let read = read_fields(&fields, &quote! { Self });
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let write = write_fields(cg, &fields);
    let encode = encode_fields(cg, &fields);
    let decode = decode_fields(cg, &fields, &quote! { Self }).map(|body| {
        quote! {
            fn decode(__sd_d: &mut #krate::binary::Decoder<'_>) -> ::std::option::Option<Self> {
                #body
            }
        }
    });
    let pack = pack_fields(cg, &fields);
    let unpack = unpack_fields(cg, &fields, &quote! { Self });
    Ok(quote! {
        // A skipped field is bound and not encoded.
        #[allow(unused_variables, reason = "a skipped field is bound and not used")]
        fn encode(&self, __sd_w: &mut #krate::binary::Writer) {
            let Self { #(#idents),* } = self;
            #encode
        }
        #decode
        // A skipped field is bound and not packed.
        #[allow(unused_variables, reason = "a skipped field is bound and not used")]
        fn pack(&self, __sd_p: &mut #krate::frame::Packer) {
            let Self { #(#idents),* } = self;
            #pack
        }
        fn unpack(__sd_u: &mut #krate::frame::Unpacker<'_>) -> ::std::option::Option<Self> {
            #unpack
        }
        fn shape() -> #krate::Shape {
            #krate::Shape::Struct(#krate::schema::StructShape { name: #shape_name, doc: #doc, fields: #shapes })
        }
        fn read(v: &#krate::ValueRef<'_>, cx: &mut #krate::Reader<'_>) -> ::std::option::Option<Self> {
            let map = cx.map(v)?;
            #read
        }
        // A skipped field is bound and not written.
        #[allow(unused_variables, reason = "a skipped field is bound and not used")]
        fn write(&self) -> #krate::Value {
            let Self { #(#idents),* } = self;
            let mut m = #krate::Map::new();
            #write
            #krate::Value::Map(m)
        }
    })
}

/// A struct's fields by their Rust names, as files write them (`field!`).
pub(crate) fn fields_of(cg: &Codegen, input: &DeriveInput) -> syn::Result<Tokens> {
    let krate = &cg.krate;
    let name = &input.ident;
    let (impl_g, ty_g, where_g) = input.generics.split_for_impl();
    let Data::Struct(syn::DataStruct {
        fields: Fields::Named(named),
        ..
    }) = &input.data
    else {
        return Ok(quote! {});
    };
    let pairs = named_fields(cg, named)?.into_iter().map(|f| {
        let ident = f.ident.to_string();
        let ident = ident.trim_start_matches("r#");
        let key = &f.key;
        quote! { (#ident, #key) }
    });
    Ok(quote! {
        impl #impl_g #krate::schema::Fields for #name #ty_g #where_g {
            const FIELDS: &'static [(&'static str, &'static str)] = &[#(#pairs),*];
        }
    })
}

/// A struct of one unnamed field: its field's schema throughout.
pub(crate) fn expand_newtype(cg: &Codegen, ty: &Type) -> Tokens {
    let krate = &cg.krate;
    quote! {
        fn shape() -> #krate::Shape { <#ty as #krate::Schema>::shape() }
        fn read(v: &#krate::ValueRef<'_>, cx: &mut #krate::Reader<'_>) -> ::std::option::Option<Self> {
            <#ty as #krate::Schema>::read(v, cx).map(Self)
        }
        fn write(&self) -> #krate::Value { #krate::Schema::write(&self.0) }
        fn encode(&self, w: &mut #krate::binary::Writer) { #krate::Schema::encode(&self.0, w) }
        fn decode(d: &mut #krate::binary::Decoder<'_>) -> ::std::option::Option<Self> {
            <#ty as #krate::Schema>::decode(d).map(Self)
        }
        fn pack(&self, p: &mut #krate::frame::Packer) { #krate::Schema::pack(&self.0, p) }
        fn unpack(u: &mut #krate::frame::Unpacker<'_>) -> ::std::option::Option<Self> {
            <#ty as #krate::Schema>::unpack(u).map(Self)
        }
        fn absent() -> ::std::option::Option<Self> { <#ty as #krate::Schema>::absent().map(Self) }
        fn skip(&self) -> bool { #krate::Schema::skip(&self.0) }
    }
}
