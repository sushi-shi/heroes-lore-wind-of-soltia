//! Read raw members out of an official build's JAR, in-process.
//!
//! The transliteration's classpath seam ([`ResourceBank`](heroes_lore_wind_of_soltia_game_xlat::resources::ResourceBank))
//! is keyed by the JAR entry name (`"img/logo.mph"`, `"img/logo.mpd"`, …) and holds
//! the raw bytes as Java `byte` (`i8`); the game's `getResourceAsStream("/img/logo…")`
//! reads the entry whose name is that path without the leading slash (the bank
//! strips one `/` before matching). So this reader materialises EVERY entry into the
//! bank verbatim — exactly what `tests/first_frame.rs`'s `load_resources` does —
//! rather than guessing which members `PngMerger`/`AssetCache` will touch. Deflate is
//! the only codec the JARs use, so no display, subprocess, or extra codec is needed.

use std::io::Read;
use std::path::Path;

use heroes_lore_wind_of_soltia_game_xlat::resources::ResourceBank;

/// Everything that can go wrong materializing the JAR (loud, never a panic).
#[derive(Debug)]
pub enum JarError {
    /// The JAR file could not be read from disk.
    Io(std::io::Error),
    /// The bytes are not a readable ZIP/JAR archive.
    Archive(zip::result::ZipError),
    /// The archive opened but yielded zero usable entries (extraction is broken).
    Empty,
}

impl std::fmt::Display for JarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JarError::Io(e) => write!(f, "reading the JAR file: {e}"),
            JarError::Archive(e) => write!(f, "opening the JAR as a ZIP archive: {e}"),
            JarError::Empty => write!(f, "the JAR yielded zero entries — extraction is broken"),
        }
    }
}

impl std::error::Error for JarError {}

/// Load every entry of the JAR at `path` into `bank`, keyed by its zip name
/// (backslashes normalised to `/`, directories skipped) with bytes reinterpreted as
/// Java `byte[]` (`i8`). Fails loudly (R1/R2) — a missing or corrupt archive is an
/// error, never a silent empty result.
pub fn load_into_bank(path: &Path, bank: &mut ResourceBank) -> Result<(), JarError> {
    let bytes = std::fs::read(path).map_err(JarError::Io)?;
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(JarError::Archive)?;

    let mut count = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(JarError::Archive)?;
        if entry.is_dir() {
            continue;
        }
        let name = match entry.enclosed_name() {
            Some(p) => p.to_string_lossy().replace('\\', "/"),
            None => continue,
        };
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).map_err(JarError::Io)?;
        bank.insert(name, buf.into_iter().map(|b| b as i8).collect());
        count += 1;
    }
    if count == 0 {
        return Err(JarError::Empty);
    }
    Ok(())
}
