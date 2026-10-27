//! The matrix in `docs/evolution.md`, checked against the rows the tests run.

use std::path::PathBuf;

use crate::matrix;

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
