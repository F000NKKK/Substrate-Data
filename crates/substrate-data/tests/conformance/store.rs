//! The golden files on disk: written on request, compared otherwise.

use std::fs;
use std::path::{Path, PathBuf};

use crate::case::Case;
use crate::json::document;

/// The directories of raw files and their extensions.
const RAW_DIRS: [(&str, &str); 5] = [
    ("binary", "sdat"),
    ("binary_v2", "sdat"),
    ("frames", "frame"),
    ("frames_v2", "frame"),
    ("io", "stream"),
];

/// The manifest's file name.
const MANIFEST: &str = "cases.json";

/// `conformance/` at the repository root.
pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

/// Every file the cases make: path under `conformance/`, bytes.
pub(crate) fn expected(cases: &[Case]) -> Vec<(String, Vec<u8>)> {
    let manifest: Vec<_> = cases.iter().map(Case::to_json).collect();
    let mut files = vec![(MANIFEST.to_owned(), document(&manifest).into_bytes())];
    files.extend(cases.iter().flat_map(Case::files));
    files
}

/// The raw files on disk, as paths under `conformance/`.
fn on_disk() -> Vec<String> {
    let mut found = Vec::new();
    for (dir, ext) in RAW_DIRS {
        let Ok(entries) = fs::read_dir(root().join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == ext) {
                found.push(format!("{dir}/{}", entry.file_name().to_string_lossy()));
            }
        }
    }
    found.sort();
    found
}

/// Rewrites every golden file from `files` and removes the stale ones.
pub(crate) fn bless(files: &[(String, Vec<u8>)]) {
    let root = root();
    for (dir, _) in RAW_DIRS {
        fs::create_dir_all(root.join(dir)).expect("a directory");
    }
    for stale in on_disk() {
        fs::remove_file(root.join(stale)).expect("removing a stale file");
    }
    for (path, bytes) in files {
        fs::write(root.join(path), bytes).expect("writing a golden file");
    }
}

/// Fails when a golden file differs from `files`, is missing or is extra.
pub(crate) fn compare(files: &[(String, Vec<u8>)]) {
    let root = root();
    let mut problems = Vec::new();
    for (path, bytes) in files {
        match fs::read(root.join(path)) {
            Ok(on_disk) if on_disk == *bytes => {}
            Ok(_) => problems.push(format!("differs: {path}")),
            Err(_) => problems.push(format!("missing: {path}")),
        }
    }
    let names: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    for extra in on_disk() {
        if !names.contains(&extra.as_str()) {
            problems.push(format!("not made by any case: {extra}"));
        }
    }
    assert!(
        problems.is_empty(),
        "the conformance vectors drifted from the implementation ({} files). A changed byte is \
         a format break. If it is deliberate, regenerate with \
         `CONFORMANCE_BLESS=1 cargo test -p substrate-data --test conformance` and review the \
         diff.\n{}",
        problems.len(),
        problems
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The manifest as parsed JSON.
pub(crate) fn manifest() -> Vec<serde_json::Value> {
    let text = fs::read_to_string(root().join(MANIFEST)).expect("cases.json");
    serde_json::from_str(&text).expect("cases.json is JSON")
}

/// A raw file by its path in the manifest.
pub(crate) fn raw(path: &str) -> Vec<u8> {
    fs::read(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A raw file already on disk, if there is one: what a case written before
/// version 2 keeps, byte for byte (a vector is never rewritten).
pub(crate) fn legacy(path: &str) -> Option<Vec<u8>> {
    fs::read(root().join(path)).ok()
}
