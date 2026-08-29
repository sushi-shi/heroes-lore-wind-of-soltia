//! `hlws-formats` — parsers for the custom J2ME resource formats used by
//! *Heroes Lore: Wind of Soltia* (Qplaze, 2007).
//!
//! Every parser is total on arbitrary input: it returns [`Result<_, FormatError>`]
//! and never panics, overflows, or reads out of bounds on malformed, truncated,
//! or empty data. Parsers validate structure (headers, length prefixes, chunk
//! framing) hard enough to reject non-conforming bytes, but they deliberately do
//! **not** decode payload semantics that belong to later phases (PNG pixels, the
//! event VM opcode set, sprite draw order, etc.).
//!
//! The format hypotheses were validated against every applicable blob in the
//! baseline JAR (`_originals/Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`). Where
//! a Phase-1 recon hypothesis did not match reality, the parser was corrected to
//! the observed layout; those corrections are documented on the relevant module.
//!
//! One module per format (plus one shared primitive):
//! - [`mpd`]  signature-stripped, concatenated sub-PNG sprite atlas
//! - [`map`]  tile map (`ver`,`w`,`h` header + `w*h` tile ids)
//! - [`mph`]  atlas frame index paired with a `.mpd` atlas: records decoded to
//!   `[mpd_index][offset][chunk_bitmask]` (+ optional trailing shared palette)
//! - [`tdf`]  count-prefixed table of ascii numeric lang-ids
//! - [`cell`] shared 4-byte sprite CELL + the "bare payload" group-list grammar
//! - [`sprite`] `*/spr/*` sprite-assembly record stream (`[grp][idx][len]` + payload)
//! - [`eif`]  bare-payload sprite scripts (`.eif`/`atef`/`die`/`ea` family)
//! - [`csprite`] `c*/s/*` character sprites — FLAT/NESTED/BARE, routed by suffix
//! - [`lang`] offset-indexed Latin-1 string blob
//! - [`item`] item tables (`itm/NN`, `forshop` record framing; `mixtbl` recipes)
//! - [`media`] wrapped standard media (`PNG`, `MIDI`, `WAV`) — magic validation
//! - [`evt`]  per-map event container + decoded EventScript VM bytecode

pub mod cell;
pub mod csprite;
pub mod eif;
pub mod evt;
pub mod item;
pub mod lang;
pub mod map;
pub mod media;
pub mod mpd;
pub mod mph;
pub mod sprite;
pub mod tdf;

/// The single error type returned by every parser in this crate.
///
/// It is a hand-rolled enum (no `thiserror`) so the crate stays dependency-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// Input was empty but the format requires at least a header.
    Empty,
    /// Input ended before a structure that the header/length said must be there.
    /// `needed` bytes were required at `offset`, but only `have` were available.
    Truncated {
        /// What was being read when the input ran out.
        what: &'static str,
        /// Byte offset at which the read was attempted.
        offset: usize,
        /// Number of bytes required from `offset`.
        needed: usize,
        /// Number of bytes actually available from `offset`.
        have: usize,
    },
    /// A required magic number / signature did not match.
    BadMagic {
        /// What signature was expected.
        what: &'static str,
    },
    /// A header field held a value the format does not allow.
    BadField {
        /// Which field was invalid.
        what: &'static str,
    },
    /// A length/offset field pointed past the end of the input or otherwise did
    /// not reconcile with the actual byte count.
    Inconsistent {
        /// What did not reconcile.
        what: &'static str,
    },
    /// A field that must be ascii text held a non-ascii (or otherwise out of
    /// range) byte.
    NotAscii {
        /// Which field held the offending byte.
        what: &'static str,
    },
    /// Integer overflow while computing an offset/length from input-derived
    /// values (treated as malformed input rather than a panic).
    Overflow {
        /// Where the overflow occurred.
        what: &'static str,
    },
}

impl core::fmt::Display for FormatError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FormatError::Empty => write!(f, "input is empty"),
            FormatError::Truncated {
                what,
                offset,
                needed,
                have,
            } => write!(
                f,
                "truncated reading {what} at offset {offset}: needed {needed} bytes, have {have}"
            ),
            FormatError::BadMagic { what } => write!(f, "bad magic/signature for {what}"),
            FormatError::BadField { what } => write!(f, "invalid header field: {what}"),
            FormatError::Inconsistent { what } => write!(f, "inconsistent structure: {what}"),
            FormatError::NotAscii { what } => write!(f, "non-ascii byte in {what}"),
            FormatError::Overflow { what } => write!(f, "integer overflow computing {what}"),
        }
    }
}

impl std::error::Error for FormatError {}

/// A tiny cursor over a byte slice with checked, non-panicking reads. Shared by
/// the parser modules so offset arithmetic is overflow-safe in one place.
pub(crate) struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Read `n` bytes, advancing the cursor. Returns `Truncated` if fewer than
    /// `n` bytes remain.
    pub(crate) fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], FormatError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(FormatError::Overflow { what })?;
        if end > self.buf.len() {
            return Err(FormatError::Truncated {
                what,
                offset: self.pos,
                needed: n,
                have: self.remaining(),
            });
        }
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    pub(crate) fn u8(&mut self, what: &'static str) -> Result<u8, FormatError> {
        Ok(self.take(1, what)?[0])
    }

    pub(crate) fn u32_be(&mut self, what: &'static str) -> Result<u32, FormatError> {
        let b = self.take(4, what)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Skip `n` bytes without returning them.
    pub(crate) fn skip(&mut self, n: usize, what: &'static str) -> Result<(), FormatError> {
        self.take(n, what).map(|_| ())
    }
}
