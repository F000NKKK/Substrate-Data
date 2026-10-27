//! Doc comments as a shape's docs: paragraphs kept, lines joined.

use crate::attr_name;

/// The `///` docs of `list`: paragraphs keep their breaks, the lines of
/// one paragraph join with spaces.
pub(crate) fn docs(list: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in list.iter().filter(|a| a.path().is_ident(attr_name::DOC)) {
        if let syn::Meta::NameValue(nv) = &attr.meta
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
        {
            lines.push(s.value().trim().to_string());
        }
    }
    let mut out = String::new();
    for line in lines {
        if line.is_empty() {
            if !out.ends_with("\n\n") && !out.is_empty() {
                out.push_str("\n\n");
            }
        } else {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push(' ');
            }
            out.push_str(&line);
        }
    }
    out.trim_end().to_string()
}
