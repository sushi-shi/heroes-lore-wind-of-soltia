//! `.tdf` — count-prefixed table of ascii numeric id strings.
//!
//! Layout (confirmed against all 19 baseline blobs): a `count: u8` byte, then
//! `count` records of `[u16 BE len][len ascii bytes]` (all observed lengths fit in
//! the low byte, so the high byte is `00`). The records must reconcile exactly to
//! the end of the file.
//!
//! **Id indirection (do not resolve to text in Phase 1).** Each ascii string is an
//! ASCII-decimal integer that indexes the `lang` string table
//! ([`crate::lang`]) — a name/description reference, *not* the text itself. The
//! displayed text is whatever the `lang` blob (the mislabeled `language.fr-FR`,
//! English in the EN build) holds at that id. This module returns the id strings
//! as-is and never resolves one to its string content. Same indirection as the
//! ids embedded in [`crate::item`] records.

use crate::{FormatError, Reader};

/// A parsed `.tdf` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tdf {
    /// The ascii id strings, in file order.
    pub ids: Vec<String>,
}

/// Parse a `.tdf` blob. See [module docs](self).
pub fn parse(input: &[u8]) -> Result<Tdf, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);
    let count = r.u8("tdf count")?;
    let mut ids = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let hi = r.u8("tdf record prefix")?;
        if hi != 0 {
            return Err(FormatError::BadField {
                what: "tdf record high length byte != 0",
            });
        }
        let len = r.u8("tdf record length")? as usize;
        let bytes = r.take(len, "tdf record string")?;
        if !bytes.iter().all(|b| b.is_ascii_graphic()) {
            return Err(FormatError::NotAscii { what: "tdf id" });
        }
        // Safe: verified ascii-graphic above.
        ids.push(String::from_utf8_lossy(bytes).into_owned());
    }
    if !r.is_empty() {
        return Err(FormatError::Inconsistent {
            what: "tdf has trailing bytes after records",
        });
    }
    Ok(Tdf { ids })
}
