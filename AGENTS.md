# AGENTS.md — Substrate Data

- This repository is domain-neutral. Nothing here may name a game, an
  engine or a product: no Minecraft ids, no engine math types, no editor
  specifics. Domain types belong in the consumer's adapter crate
  (`impl Schema` + `Shape::Named`).
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
