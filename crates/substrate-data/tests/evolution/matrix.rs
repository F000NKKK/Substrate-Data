//! The compatibility matrix, written from the rows the tests ran.

use std::fmt::Write;

use crate::verdict::{Cells, Group, Row};
use crate::{enums, fields, nested, numbers, options};

/// Writing to a `String` cannot fail.
const WRITES: &str = "a String takes any write";

/// Every row, in the order the document shows them.
pub(crate) fn rows() -> Vec<Row> {
    [
        fields::ROWS,
        numbers::ROWS,
        options::ROWS,
        enums::ROWS,
        nested::ROWS,
    ]
    .into_iter()
    .flatten()
    .map(|row| row())
    .collect()
}

fn title(group: Group) -> &'static str {
    match group {
        Group::Fields => "Fields of a record",
        Group::Numbers => "Numbers",
        Group::Options => "Optional and required",
        Group::Enums => "Enums",
        Group::Nested => "A record that grows, held in another type",
    }
}

fn cell(cells: Cells) -> String {
    let frame = cells.frame.label();
    if cells.binary == cells.text {
        format!("{frame} / {}", cells.binary.label())
    } else {
        let (binary, text) = (cells.binary.label(), cells.text.label());
        format!("{frame} / binary {binary}, text {text}")
    }
}

/// The matrix as Markdown: a table per group.
pub(crate) fn render(rows: &[Row]) -> String {
    let mut out = String::new();
    for group in [
        Group::Fields,
        Group::Numbers,
        Group::Options,
        Group::Enums,
        Group::Nested,
    ] {
        writeln!(out, "#### {}\n", title(group)).expect(WRITES);
        out.push_str("| Change | New reader, old data | Old reader, new data |\n|---|---|---|\n");
        for row in rows.iter().filter(|row| row.group == group) {
            writeln!(
                out,
                "| {} | {} | {} |",
                row.change,
                cell(row.new_reads_old),
                cell(row.old_reads_new)
            )
            .expect(WRITES);
        }
        out.push('\n');
    }
    out
}
