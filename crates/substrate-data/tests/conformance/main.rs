//! Language-neutral conformance vectors.
//!
//! `conformance/cases.json` and the raw files next to it are the golden
//! data a port in another language checks itself against; this test makes
//! sure the Rust reference still produces exactly them, and that the files
//! say what they claim. `CONFORMANCE_BLESS=1` rewrites them from the
//! current implementation, on purpose.

#![expect(
    clippy::panic,
    reason = "a conformance test reports a failure by panicking, in helpers as in tests"
)]

mod case;
mod check;
mod containers;
mod evolution;
mod faults_binary;
mod faults_frame;
mod faults_text;
mod frames;
mod hex;
mod io_cases;
mod json;
mod packed;
mod render;
mod schemas;
mod store;
mod syntax;
mod typed;
mod values;
mod widths;

use std::collections::HashSet;

use case::Case;

/// Every case, in manifest order.
fn all_cases() -> Vec<Case> {
    let mut all = Vec::new();
    all.extend(values::cases());
    all.extend(containers::cases());
    all.extend(packed::cases());
    all.extend(syntax::cases());
    all.extend(frames::cases());
    all.extend(evolution::cases());
    all.extend(widths::cases());
    all.extend(io_cases::cases());
    all.extend(faults_text::cases());
    all.extend(faults_binary::cases());
    all.extend(faults_frame::cases());
    let mut names = HashSet::new();
    for case in &all {
        assert!(
            names.insert(case.name.clone()),
            "two cases named {}",
            case.name
        );
    }
    all
}

#[test]
fn the_golden_files_are_what_the_implementation_makes_and_say_what_they_claim() {
    let files = store::expected(&all_cases());
    if std::env::var("CONFORMANCE_BLESS").is_ok_and(|v| v == "1") {
        store::bless(&files);
    } else {
        store::compare(&files);
    }
    check::run();
}
