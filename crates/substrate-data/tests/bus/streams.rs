//! Streams: a big list written whole, read lazily.

use substrate_data::{Schema, Value, binary};

/// A big list kept written: items as they come, read when asked.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Folder {
    pub(crate) name: String,
    pub(crate) owners: Vec<String>,
    pub(crate) entries: substrate_data::Stream<Entry>,
    pub(crate) after: String,
}

#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Entry {
    pub(crate) id: String,
    pub(crate) count: u32,
}

#[test]
fn streams_go_whole_and_read_lazily() {
    let entries: substrate_data::Stream<Entry> = (0..500)
        .map(|i| Entry {
            id: format!("docs/{}", ["intro", "faq"][i % 2]),
            count: u32::try_from(i).unwrap_or(u32::MAX),
        })
        .collect();
    // Strings the rest of the document has too: the stream's are its own.
    let folder = Folder {
        name: "docs/intro".into(),
        owners: vec!["docs/faq".into()],
        entries,
        after: "docs/intro".into(),
    };
    let bytes = substrate_data::to_bytes(&folder);
    let back: Folder = substrate_data::read_bytes(&bytes).unwrap();
    assert_eq!(back, folder);
    assert_eq!(
        back.entries.iter().nth(3),
        Some(Entry {
            id: "docs/faq".into(),
            count: 3
        })
    );
    // The value way reads it as a list; the text way writes and reads it too.
    let value = binary::decode(&bytes).unwrap();
    assert_eq!(
        value
            .get("entries")
            .and_then(|b| b.as_list())
            .map(<[Value]>::len),
        Some(500)
    );
    let text = substrate_data::to_text(&folder);
    assert_eq!(
        substrate_data::from_text::<Folder>(&text).unwrap().value,
        folder
    );
    // Cut short anywhere, it's an error, never a panic.
    for len in 0..bytes.len() {
        assert!(
            substrate_data::read_bytes::<Folder>(&bytes[..len]).is_err(),
            "truncated at {len}"
        );
    }
}
