# substrate-data-derive

`#[derive(Schema)]` for substrate-data.

**Layer and context:** shared kernel · data

- **For:** generating `Schema` for structs and enums: reading with paths, writing, binary and frame forms, the shape description, and `#[data(...)]` options (`rename`, `default`, `skip`, `range`, `tag`, `view`).
- **Not for:** the model, the forms and the schema traits themselves; they live in [`substrate-data`](../substrate-data), which re-exports the derive. Depend on that crate, not on this one.

## Usage

```rust
use substrate_data::Schema;

#[derive(Schema, Debug, PartialEq)]
struct Project {
    /// The project's name.
    name: String,
    #[data(default)]
    archived: bool,
}

let project: Project = substrate_data::from_text(r#"{ name: "site" }"#).unwrap().value;
assert_eq!(project, Project { name: "site".into(), archived: false });
```

The full list of attributes is in the [crate documentation](https://docs.rs/substrate-data-derive) and [docs/format.md](../../docs/format.md#schemas).

## Depends on / used by

- Depends on: no workspace crate (`substrate-data` is a dev-dependency only, to test what the derive expands against)
- Used by: [`substrate-data`](../substrate-data)

## License

MIT OR Apache-2.0, see [LICENSE-MIT](../../LICENSE-MIT) and [LICENSE-APACHE](../../LICENSE-APACHE).
