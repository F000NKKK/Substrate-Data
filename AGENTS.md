# AGENTS.md — Substrate Data

- This repository is domain-neutral. Nothing here may name a product or
  borrow one consumer's vocabulary — no consumer's ids, math types or
  editor specifics, not even in examples, tests or benches (examples use
  one neutral domain: users, docs, files, projects). Domain types belong
  in the consumer's adapter crate (`impl Schema` + `Shape::Named`).
- `docs/format.md` is the specification. A change to the text grammar,
  binary tags, frame layout, `Shape` description keys or diagnostic codes
  updates it in the same commit. Binary tags and frame rules only grow:
  never renumber a tag, never reorder frame fields.
- Malformed input (text, binary, frames) is an error or `None`, never a
  panic or an unbounded allocation; add a test for each new decoder path.
- A `Value::Str` is a string as spelled (see `model/value.rs`): plain text
  starting with `@` is stored with it doubled.
- Before pushing: `cargo fmt --all`, `cargo clippy --workspace
  --all-features --all-targets -- -D warnings`, `cargo test --workspace
  --all-features`, `cargo bench --no-run`.

## Engineering standard

Rust code here follows **Craftbook**, our engineering standard:
https://github.com/F000NKKK/craftbook. Read its `README.md` and the relevant
`rust/` chapter before writing or reviewing code; the short version is in
[`.claude/rules/craftbook.md`](.claude/rules/craftbook.md). Where this
repository's docs disagree with Craftbook, Craftbook wins and the local doc
is fixed in the same change.
