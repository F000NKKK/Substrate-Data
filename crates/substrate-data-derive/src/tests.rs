//! The derive refuses what it can't expand, saying why and where.

use syn::{DeriveInput, parse_quote};

use crate::expand::derive;

/// `input` expanded; the message of the refusal if it isn't.
fn outcome(input: &DeriveInput) -> Result<String, String> {
    derive(input)
        .map(|tokens| tokens.to_string())
        .map_err(|e| e.to_string())
}

/// `input` is refused with a message that says `want`.
fn assert_refused(input: &DeriveInput, want: &str) {
    match outcome(input) {
        Ok(tokens) => panic!("expanded: {tokens}"),
        Err(said) => assert!(said.contains(want), "{said:?} doesn't say {want:?}"),
    }
}

/// `input` expands to code that has all of `parts`.
fn assert_expands(input: &DeriveInput, parts: &[&str]) {
    match outcome(input) {
        Ok(code) => {
            for part in parts {
                assert!(code.contains(part), "{part:?} isn't in {code}");
            }
        }
        Err(said) => panic!("refused: {said}"),
    }
}

#[test]
fn an_unknown_attribute_lists_the_known_ones() {
    let input = parse_quote! { struct A { #[data(colour = "x")] a: u8 } };
    assert_refused(&input, "unknown `data` attribute (rename, rename_all");
}

#[test]
fn attribute_values_are_checked() {
    let text = "range = \"lo..=hi\" (numbers)";
    assert_refused(
        &parse_quote! { struct A { #[data(range = "1..9")] a: u8 } },
        text,
    );
    assert_refused(
        &parse_quote! { struct A { #[data(range = "a..=9")] a: u8 } },
        text,
    );
    assert_refused(
        &parse_quote! { #[data(rename_all = "snake_case")] struct A { a: u8 } },
        "rename_all = \"camelCase\" is the one case",
    );
    assert_refused(
        &parse_quote! { struct A { #[data(rename = 3)] a: u8 } },
        "expected string literal",
    );
    assert_refused(
        &parse_quote! { struct A { #[data(with = "not a type!")] a: u8 } },
        "expected",
    );
}

#[test]
fn only_structs_with_named_fields_or_one_field_and_enums_derive() {
    let need = "Schema: a struct needs named fields or exactly one unnamed field";
    assert_refused(&parse_quote! { struct A(u8, u8); }, need);
    assert_refused(&parse_quote! { struct A; }, need);
    assert_refused(
        &parse_quote! { union A { a: u8 } },
        "Schema: unions aren't supported",
    );
    assert_expands(&parse_quote! { struct A(u8); }, &["Schema for A"]);
}

#[test]
fn variants_wrap_one_value_at_most() {
    assert_refused(
        &parse_quote! { #[data(tag = "t")] enum E { A(u8, u8) } },
        "Schema: a variant wraps one struct at most",
    );
}

#[test]
fn other_is_a_tagged_variant_wrapping_a_type() {
    let rule = "`other` is a tagged enum's variant";
    // An enum with data is tagged by `type`, so `other` finds its tag.
    assert_expands(
        &parse_quote! { enum E { #[data(other)] A(u8) } },
        &["Schema for E"],
    );
    assert_refused(&parse_quote! { enum E { #[data(other)] A } }, rule);
    assert_refused(
        &parse_quote! { #[data(tag = "t")] enum E { #[data(other)] A } },
        rule,
    );
    assert_refused(
        &parse_quote! { #[data(tag = "t")] enum E { #[data(other)] A { x: u8 } } },
        rule,
    );
    assert_expands(
        &parse_quote! { #[data(tag = "t")] enum E { B, #[data(other)] A(Inner) } },
        &["Schema for E"],
    );
}

#[test]
fn a_view_is_for_a_struct_with_named_fields_and_no_generics() {
    assert_refused(
        &parse_quote! { #[data(view)] struct A<T> { a: T } },
        "Schema: `view` is for a struct without generics",
    );
    let named = "Schema: `view` is for a struct with named fields";
    assert_refused(&parse_quote! { #[data(view)] struct A(u8); }, named);
    assert_refused(&parse_quote! { #[data(view)] enum E { A } }, named);
    assert_expands(
        &parse_quote! { #[data(view)] pub struct A { a: String } },
        &["AView"],
    );
}

#[test]
fn enums_with_data_are_tagged_by_type_unless_they_say() {
    assert_expands(&parse_quote! { enum E { A { x: u8 } } }, &["\"type\""]);
    assert_expands(
        &parse_quote! { #[data(tag = "kind")] enum E { A { x: u8 } } },
        &["\"kind\""],
    );
    assert_expands(
        &parse_quote! { enum E { First, SecondOne } },
        &["\"first\"", "\"second_one\""],
    );
}

#[test]
fn names_follow_rename_all_and_rename() {
    assert_expands(
        &parse_quote! {
            #[data(rename_all = "camelCase")]
            struct A { first_name: u8, #[data(rename = "kept_as_is")] other_name: u8, r#type: u8 }
        },
        &["\"firstName\"", "\"kept_as_is\"", "\"type\""],
    );
    assert_expands(
        &parse_quote! {
            #[data(rename_all = "camelCase", tag = "type")]
            enum E { SharedLink { link_id: u8 } }
        },
        &["\"sharedLink\"", "\"link_id\""],
    );
}

#[test]
fn doc_comments_become_the_shapes_docs() {
    assert_expands(
        &parse_quote! {
            /// First line
            /// continues here.
            ///
            ///
            /// A second paragraph.
            struct A { a: u8 }
        },
        &["First line continues here.\\n\\nA second paragraph."],
    );
}

#[test]
fn the_crate_attribute_names_where_the_runtime_is() {
    assert_expands(
        &parse_quote! { #[data(crate = "my_sdk::data")] struct A { a: u8 } },
        &["my_sdk :: data :: Schema for A"],
    );
}

#[test]
fn fields_are_listed_by_their_rust_names_for_the_field_macro() {
    assert_expands(
        &parse_quote! { struct A { r#type: u8, #[data(rename = "x")] y: u8 } },
        &["Fields for A", "(\"type\" , \"type\")", "(\"y\" , \"x\")"],
    );
}
