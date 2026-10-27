//! What survives a change to a derived type, per form.
//!
//! Each row of the matrix in `docs/evolution.md` is a change to a type with
//! values written by one version and read by the other, in the frame form and
//! in the named forms (binary and text). The document's matrix is written from
//! these rows: `EVOLUTION_BLESS=1 cargo test --test evolution` rewrites it.

mod base;
mod enums;
mod fields;
mod guarantees;
mod matrix;
mod nested;
mod numbers;
mod options;
mod verdict;

use std::path::PathBuf;

const START: &str = "<!-- matrix:start -->\n";
const END: &str = "<!-- matrix:end -->";

fn document() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/evolution.md")
}

#[test]
fn the_document_shows_the_matrix_the_tests_ran() {
    let matrix = matrix::render(&matrix::rows());
    let text = std::fs::read_to_string(document()).unwrap();
    let (head, rest) = text.split_once(START).unwrap();
    let (_, tail) = rest.split_once(END).unwrap();
    let fresh = format!("{head}{START}\n{matrix}{END}{tail}");
    if std::env::var_os("EVOLUTION_BLESS").is_some() {
        std::fs::write(document(), &fresh).unwrap();
        return;
    }
    assert_eq!(
        text, fresh,
        "docs/evolution.md is out of date: EVOLUTION_BLESS=1"
    );
}
