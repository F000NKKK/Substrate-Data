//! [`Codegen`]: what the code a derive generates depends on, passed along.

use proc_macro2::TokenStream as Tokens;
use quote::quote;

use crate::case::Case;

/// The derive running now: where `substrate_data` is, and how the
/// container names its fields or variants.
pub(crate) struct Codegen {
    /// The path of `substrate_data`: `::substrate_data`, or the container's
    /// `#[data(crate = "...")]` (a crate that re-exports it).
    pub(crate) krate: Tokens,
    /// The container's `rename_all`.
    pub(crate) case: Option<Case>,
}

impl Codegen {
    /// For a container with `path` and `case`.
    pub(crate) fn new(path: &syn::Path, case: Option<Case>) -> Codegen {
        Codegen {
            krate: quote!(#path),
            case,
        }
    }

    /// The same, fields named as in Rust: an enum's `rename_all` names its
    /// variants, not their fields.
    pub(crate) fn fields_as_written(&self) -> Codegen {
        Codegen {
            krate: self.krate.clone(),
            case: None,
        }
    }
}

/// Where `substrate_data` is for the crate deriving: its own dependency (or
/// the crate itself). A crate that has it only through a re-export says
/// where with `#[data(crate = "my_sdk::data")]`.
pub(crate) fn default_path() -> syn::Path {
    use proc_macro_crate::{FoundCrate, crate_name};
    match crate_name("substrate-data") {
        Ok(FoundCrate::Name(name)) => {
            let name = quote::format_ident!("{name}");
            syn::parse_quote!(::#name)
        }
        _ => syn::parse_quote!(::substrate_data),
    }
}
