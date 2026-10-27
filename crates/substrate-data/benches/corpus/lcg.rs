//! A tiny linear congruential generator and the vocabulary the corpus draws on.

/// Words of the long texts.
const WORDS: [&str; 48] = [
    "project", "document", "folder", "archive", "report", "draft", "review", "release", "branch",
    "merge", "owner", "reader", "editor", "viewer", "member", "invite", "quota", "limit", "backup",
    "restore", "upload", "download", "rename", "share", "public", "private", "shared", "recent",
    "pinned", "starred", "deleted", "version", "history", "comment", "thread", "notes", "tasks",
    "label", "filter", "search", "index", "cache", "queue", "sync", "export", "import", "format",
    "layout",
];

/// First names of the users.
pub(crate) const NAMES: [&str; 16] = [
    "alice", "bob", "carol", "dave", "erin", "frank", "grace", "heidi", "ivan", "judy", "mallory",
    "niaj", "olivia", "peggy", "rupert", "sybil",
];

/// Folders the files and docs live in.
const FOLDERS: [&str; 8] = [
    "projects/alpha",
    "projects/beta",
    "projects/gamma",
    "docs/guides",
    "docs/specs",
    "files/images",
    "files/sounds",
    "files/archive",
];

/// Roles a user has.
const ROLES: [&str; 4] = ["owner", "editor", "viewer", "guest"];

/// Tags a doc carries.
const TAGS: [&str; 10] = [
    "draft", "final", "urgent", "internal", "public", "legacy", "review", "todo", "done", "idea",
];

/// Strings with characters that are not plain ASCII, as a text field holds them.
const UNICODE: [&str; 6] = [
    "naïve café — résumé",
    "日本語のテキスト",
    "Привет, мир",
    "emoji 😀 and 🚀 together",
    "tab\there, \"quoted\", back\\slash",
    "line one\nline two",
];

/// A xorshift generator: the same numbers on every run and machine, and no
/// dependency.
#[derive(Debug, Clone)]
pub(crate) struct Lcg(u64);

impl Lcg {
    /// A generator that starts from `seed`.
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// The next 64 bits.
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let x = self.0;
        (x ^ (x >> 33)).wrapping_mul(0xFF51_AFD7_ED55_8CCD) ^ (x >> 29)
    }

    /// A number below `n` (`n > 0`).
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// A float in `[0, 1)`.
    pub(crate) fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// One of `items`.
    pub(crate) fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    /// A word.
    pub(crate) fn word(&mut self) -> &'static str {
        self.pick(&WORDS)
    }

    /// A user's first name.
    pub(crate) fn name(&mut self) -> &'static str {
        self.pick(&NAMES)
    }

    /// A folder.
    pub(crate) fn folder(&mut self) -> &'static str {
        self.pick(&FOLDERS)
    }

    /// A role.
    pub(crate) fn role(&mut self) -> &'static str {
        self.pick(&ROLES)
    }

    /// A tag.
    pub(crate) fn tag(&mut self) -> &'static str {
        self.pick(&TAGS)
    }

    /// A string with non-ASCII characters, quotes or line breaks.
    pub(crate) fn unicode(&mut self) -> &'static str {
        self.pick(&UNICODE)
    }

    /// Between `min` and `min + spread - 1` words.
    pub(crate) fn phrase(&mut self, min: usize, spread: usize) -> String {
        let n = min + self.below(spread);
        self.words(n)
    }

    /// `n` words, separated by spaces.
    pub(crate) fn words(&mut self, n: usize) -> String {
        let mut out = String::new();
        for i in 0..n {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(self.word());
        }
        out
    }
}
