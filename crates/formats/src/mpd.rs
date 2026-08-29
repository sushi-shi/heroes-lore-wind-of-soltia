//! `.mpd` — signature-stripped PNG sprite atlas.
//!
//! Recon hypothesis (confirmed, with one correction): the bytes begin at the
//! PNG `IHDR` chunk (`00 00 00 0d 49 48 44 52 …`), i.e. the 8-byte PNG magic is
//! stripped. Prepending the magic yields valid PNG chunk framing.
//!
//! Correction discovered against the corpus: an `.mpd` is **not** a single PNG.
//! All 179 baseline blobs are a *concatenation* of several sub-images — repeated
//! `IHDR[,PLTE,tRNS],IDAT` runs — and the trailing `IEND` chunk is stripped too.
//! So the parser walks the whole length-prefixed chunk stream and requires it to
//! reconcile exactly to EOF; it does **not** require `IEND`.
//!
//! We read the first `IHDR` (width, height, bit depth, color type) and validate
//! chunk framing (each chunk: `u32 len` + 4-byte type + data + 4-byte CRC) enough
//! to reject non-PNG / truncated input. Pixel/CRC-value decoding is Phase 3.

use crate::{FormatError, Reader};

/// The 8-byte PNG signature that an `.mpd` has had stripped from its front.
pub const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Parsed `.mpd` header (from the first sub-image's `IHDR`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mpd {
    /// Width from the first `IHDR`.
    pub width: u32,
    /// Height from the first `IHDR`.
    pub height: u32,
    /// PNG bit depth byte.
    pub bit_depth: u8,
    /// PNG color type byte.
    pub color_type: u8,
    /// Number of concatenated sub-images (number of `IHDR` chunks seen).
    pub subimages: usize,
}

/// Parse an `.mpd` blob (PNG magic stripped). See [module docs](self).
pub fn parse(input: &[u8]) -> Result<Mpd, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    // The stream must start at an IHDR chunk: length 13 then the type "IHDR".
    if input.len() < 8 || input[0..4] != [0x00, 0x00, 0x00, 0x0d] || &input[4..8] != b"IHDR" {
        return Err(FormatError::BadMagic { what: "mpd IHDR" });
    }

    let mut r = Reader::new(input);
    let mut first: Option<(u32, u32, u8, u8)> = None;
    let mut subimages = 0usize;

    // Walk chunks: [u32 len][4 type][len data][4 crc], reconciling to EOF.
    while !r.is_empty() {
        let len = r.u32_be("chunk length")? as usize;
        let ctype = r.take(4, "chunk type")?;
        let is_ihdr = ctype == b"IHDR";
        let data = r.take(len, "chunk data")?;
        r.skip(4, "chunk crc")?;

        if is_ihdr {
            // IHDR must be exactly 13 bytes: w(u32) h(u32) depth colortype
            // compression filter interlace.
            if len != 13 {
                return Err(FormatError::BadField {
                    what: "mpd IHDR length",
                });
            }
            let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
            let bit_depth = data[8];
            let color_type = data[9];
            if first.is_none() {
                first = Some((width, height, bit_depth, color_type));
            }
            subimages += 1;
        }
    }

    let (width, height, bit_depth, color_type) = first.ok_or(FormatError::BadField {
        what: "mpd has no IHDR",
    })?;

    Ok(Mpd {
        width,
        height,
        bit_depth,
        color_type,
        subimages,
    })
}

/// Byte offsets of every `IHDR` chunk in an `.mpd` (i.e. the start offset of each
/// concatenated sub-image). Walks the length-prefixed chunk stream and requires
/// it to reconcile exactly to EOF, so a truncated/garbage blob is rejected.
///
/// Used by the `mph -> mpd` cross-format check: every `.mph` record `offset` must
/// be one of these values.
pub fn ihdr_offsets(input: &[u8]) -> Result<Vec<usize>, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    if input.len() < 8 || input[0..4] != [0x00, 0x00, 0x00, 0x0d] || &input[4..8] != b"IHDR" {
        return Err(FormatError::BadMagic { what: "mpd IHDR" });
    }
    let mut r = Reader::new(input);
    let mut offsets = Vec::new();
    while !r.is_empty() {
        let chunk_start = r.pos();
        let len = r.u32_be("chunk length")? as usize;
        let ctype = r.take(4, "chunk type")?;
        let is_ihdr = ctype == b"IHDR";
        r.skip(len, "chunk data")?;
        r.skip(4, "chunk crc")?;
        if is_ihdr {
            offsets.push(chunk_start);
        }
    }
    Ok(offsets)
}
