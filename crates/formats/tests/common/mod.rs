//! Shared test support: extract the baseline JAR's resource blobs into an OS
//! temp dir (never the repo tree) and expose them by name.
//!
//! The baseline JAR is git-ignored and read at test time only; extracted blobs
//! land under the system temp dir, so no resource bytes are ever written into the
//! repository.

#![allow(dead_code)] // each test binary uses a subset of the helpers

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Baseline JAR filename (relative to the repo's `_originals/`).
pub const BASELINE_JAR: &str = "Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar";

/// Extracted resource blobs from the baseline JAR.
pub struct Fixtures {
    root: PathBuf,
    /// Zip-entry names (files only), forward-slash separated.
    names: Vec<String>,
}

impl Fixtures {
    /// Read one blob's bytes by its zip-entry name.
    pub fn read(&self, name: &str) -> Vec<u8> {
        fs::read(self.root.join(name))
            .unwrap_or_else(|e| panic!("reading extracted blob {name}: {e}"))
    }

    /// Read one blob's bytes by name, or `None` if the JAR has no such entry.
    pub fn get(&self, name: &str) -> Option<Vec<u8>> {
        if self.names.iter().any(|n| n == name) {
            Some(self.read(name))
        } else {
            None
        }
    }

    /// All (name, bytes) pairs whose name satisfies `pred`.
    pub fn matching(&self, pred: impl Fn(&str) -> bool) -> Vec<(String, Vec<u8>)> {
        let mut out: Vec<(String, Vec<u8>)> = self
            .names
            .iter()
            .filter(|n| pred(n))
            .map(|n| (n.clone(), self.read(n)))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/crates/formats
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root is two levels above the crate manifest")
        .to_path_buf()
}

fn extract() -> Fixtures {
    let jar_path = repo_root().join("_originals").join(BASELINE_JAR);
    let file = fs::File::open(&jar_path).unwrap_or_else(|e| {
        panic!(
            "baseline JAR not found at {} ({e}); it must be materialized into _originals/",
            jar_path.display()
        )
    });
    let mut archive =
        zip::ZipArchive::new(file).unwrap_or_else(|e| panic!("opening JAR as zip: {e}"));

    // Unique per-process temp dir under the OS temp root (outside the repo tree).
    let root = std::env::temp_dir().join(format!("hlws_formats_fixtures_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temp fixtures dir");

    let mut names = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("zip entry");
        if entry.is_dir() {
            continue;
        }
        let rel = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => continue, // skip unsafe/absolute names
        };
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).expect("read zip entry");
        let out = root.join(&rel);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).expect("create fixture subdir");
        }
        fs::write(&out, &buf).expect("write fixture");
        names.push(rel.to_string_lossy().replace('\\', "/"));
    }
    assert!(
        !names.is_empty(),
        "extracted zero entries from the baseline JAR — extraction is broken"
    );
    Fixtures { root, names }
}

/// Process-wide, lazily-extracted fixtures.
pub fn fixtures() -> &'static Fixtures {
    static FIXTURES: OnceLock<Fixtures> = OnceLock::new();
    FIXTURES.get_or_init(extract)
}

/// Basename (portion after the last `/`) of a zip-entry name.
pub fn basename(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

/// True if the basename has no extension dot (an "extensionless" resource).
pub fn is_extensionless(name: &str) -> bool {
    !basename(name).contains('.')
}
