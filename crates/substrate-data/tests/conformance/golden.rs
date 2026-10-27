//! The golden files against the implementation.

use std::collections::HashSet;

use crate::case::Case;
use crate::{
    check, containers, evolution, faults_binary, faults_frame, faults_text, frames, io_cases,
    packed, store, syntax, values, widths,
};

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
