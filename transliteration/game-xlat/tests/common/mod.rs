//! Shared oracle support: read the baseline JAR's resource blobs into memory at
//! test time (never into the repo tree) and expose them by name/predicate.
//!
//! The baseline JAR is git-ignored and read only at test time. If it is missing,
//! the helpers panic loudly — a corpus oracle must FAIL when `_originals/` is
//! absent, never skip to a false green (GATES.md R4).

#![allow(dead_code)] // each oracle binary uses a subset of the helpers.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Baseline JAR filename under `_originals/`.
pub const BASELINE_JAR: &str = "Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar";

/// Every (name, bytes) entry of the baseline JAR, read once per test process.
pub struct Jar {
    entries: Vec<(String, Vec<u8>)>,
}

impl Jar {
    /// Bytes of one entry by exact zip name, or `None` if absent.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b.as_slice())
    }

    /// All (name, bytes) whose name satisfies `pred`, sorted by name.
    pub fn matching(&self, pred: impl Fn(&str) -> bool) -> Vec<(String, Vec<u8>)> {
        let mut out: Vec<(String, Vec<u8>)> = self
            .entries
            .iter()
            .filter(|(n, _)| pred(n))
            .cloned()
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/crates/heroes-lore-wind-of-soltia-game-xlat
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root is two levels above the crate manifest")
        .to_path_buf()
}

fn extract() -> Jar {
    let jar_path = repo_root().join("_originals").join(BASELINE_JAR);
    let file = fs::File::open(&jar_path).unwrap_or_else(|e| {
        panic!(
            "baseline JAR not found at {} ({e}); it must be materialized into \
             _originals/ (run `just bootstrap`). A corpus oracle never skips.",
            jar_path.display()
        )
    });
    let mut archive =
        zip::ZipArchive::new(file).unwrap_or_else(|e| panic!("opening JAR as zip: {e}"));

    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("zip entry");
        if entry.is_dir() {
            continue;
        }
        let name = match entry.enclosed_name() {
            Some(p) => p.to_string_lossy().replace('\\', "/"),
            None => continue,
        };
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).expect("read zip entry");
        entries.push((name, buf));
    }
    assert!(
        !entries.is_empty(),
        "extracted zero entries from the baseline JAR — extraction is broken"
    );
    Jar { entries }
}

/// Process-wide, lazily-extracted JAR.
pub fn jar() -> &'static Jar {
    static JAR: OnceLock<Jar> = OnceLock::new();
    JAR.get_or_init(extract)
}

/// Reinterpret host bytes as Java `byte[]` (`Vec<i8>`) — the convention the
/// transliteration reads. Signedness is preserved bit-for-bit.
pub fn to_i8(bytes: &[u8]) -> Vec<i8> {
    bytes.iter().map(|&b| b as i8).collect()
}
