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
mod faults_v2;
mod forms_v2;
mod frames;
mod golden;
mod hex;
mod io_cases;
mod json;
mod packed;
mod render;
mod schemas;
mod store;
mod syntax;
mod typed;
#[path = "../support/v1.rs"]
mod v1;
mod values;
mod widths;
