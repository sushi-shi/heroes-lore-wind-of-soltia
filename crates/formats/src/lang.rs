//! `lang/language.*` — offset-indexed Latin-1 string blob.
//!
//! Layout (derived and confirmed against `language.fr-FR`, the only lang blob in
//! the baseline JAR — ~174 KB):
//!
//! - `[size: u32 BE]` at offset 0, equal to `file_len - 4` (the size of
//!   everything after this word). A strong, truncation-sensitive invariant.
//! - An ascending table of `u32 BE` absolute offsets, starting at byte 4 and
//!   running up to `offsets[0]` (the first offset points at the start of the
//!   string data, so the table occupies `[4, offsets[0])` and has
//!   `(offsets[0] - 4) / 4` entries).
//! - Concatenated Latin-1 string data from `offsets[0]` to EOF. String `i` spans
//!   `offsets[i] .. offsets[i+1]` (the last runs to EOF). Fields inside a string
//!   use `&|` … `;` delimiters; Phase 1 returns raw slices and does not split
//!   them.
//!
//! Note: `size == file_len - 4` was derived from a single blob (n = 1); it holds
//! exactly and is kept as a check. Non-ascending or out-of-range offsets, a bad
//! size word, or a table that does not divide evenly are all rejected.

use crate::{FormatError, Reader};

/// A parsed language string table. Holds the raw bytes plus the offset index so
/// individual strings can be sliced on demand.
#[derive(Debug, Clone)]
pub struct Lang {
    data: Vec<u8>,
    /// Absolute byte offsets of each string (ascending). `offsets[0]` is the
    /// start of the string region.
    offsets: Vec<u32>,
}

/// Parse a `lang/language.*` blob. See [module docs](self).
pub fn parse(input: &[u8]) -> Result<Lang, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);
    let size = r.u32_be("lang size word")? as usize;
    if size != input.len().saturating_sub(4) {
        return Err(FormatError::Inconsistent {
            what: "lang size word != file_len - 4",
        });
    }

    let first = r.u32_be("lang first offset")? as usize;
    if first < 4 || first > input.len() || !(first - 4).is_multiple_of(4) {
        return Err(FormatError::BadField {
            what: "lang first offset (table end) misaligned or out of range",
        });
    }
    let n_entries = (first - 4) / 4;

    let mut offsets = Vec::with_capacity(n_entries);
    // We already consumed the first offset; record it, then read the rest.
    offsets.push(first as u32);
    let mut prev = first as u32;
    for _ in 1..n_entries {
        let off = r.u32_be("lang offset")?;
        if off < prev {
            return Err(FormatError::Inconsistent {
                what: "lang offsets not ascending",
            });
        }
        if off as usize > input.len() {
            return Err(FormatError::Inconsistent {
                what: "lang offset past end of blob",
            });
        }
        offsets.push(off);
        prev = off;
    }

    Ok(Lang {
        data: input.to_vec(),
        offsets,
    })
}

impl Lang {
    /// Number of indexed strings.
    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    /// Whether the table has no strings.
    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    /// Raw (Latin-1) bytes of string `i`, or `None` if out of range. String `i`
    /// runs from `offsets[i]` to `offsets[i+1]` (last runs to EOF).
    pub fn raw(&self, i: usize) -> Option<&[u8]> {
        let start = *self.offsets.get(i)? as usize;
        let end = self
            .offsets
            .get(i + 1)
            .map(|&o| o as usize)
            .unwrap_or(self.data.len());
        self.data.get(start..end)
    }

    /// String `i` decoded as Latin-1 (every byte maps to the same code point).
    pub fn get(&self, i: usize) -> Option<String> {
        self.raw(i).map(|b| b.iter().map(|&c| c as char).collect())
    }
}
