# Craftbook

All Rust code in this repository follows Craftbook:
https://github.com/F000NKKK/craftbook. Read its `README.md` and
`rust/01..09-*.md` before writing code. A rule is not relaxed here; the
only exceptions are the ones Craftbook defines (`craftbook.toml`
`[[exempt]]` with a reason, performance kernels).

Key limits (hard limits; targets are lower):

| What | Hard limit |
|---|---|
| Production lines in a `.rs` file (without its test module) | 400 (target 250) |
| Inline `#[cfg(test)]` module | 150, then `tests.rs` |
| `lib.rs` / `mod.rs` of a split module | 100, wiring only: docs, `mod`, `pub use`, aliases, macros |
| Files in one module directory | 12 |
| Lines in a function body (tests too) | 50 (target 25) |
| Parameters, `self` included | 5 (target 4) |
| Nested blocks in a method (free function +1) | 3 |
| Cognitive complexity | 15 |

And the rules review keeps asking for:

- One primary item per file, the file named after it; companions only
  (its error, builder, iterator, private helpers).
- Typed errors: an enum per operation family, saying what failed with what
  input; no `unwrap` outside tests; nothing swallowed.
- Strings are data, never structure: ids, kinds, keys, codes and closed
  sets are newtypes or enums, parsed once at the boundary and rendered
  back with `Display`; no string keys, no `format!`-built ids.
- No global mutable state, no managers; `#[expect(…, reason = "…")]`, never
  `#[allow]`.
- DDD layering (chapter 7): this repository is a shared-kernel library —
  pure value types and codecs, no I/O beyond `std::io::{Read, Write}`
  streams handed in by the caller.

Checks: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `python3 <craftbook>/tools/craftcheck check
. --baseline .craftcheck-baseline`, `cargo test --workspace --all-features`.
