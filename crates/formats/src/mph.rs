//! `.mph` — atlas frame index, paired with the same-stem `.mpd` atlas.
//!
//! Layout (reversed from loader `br`/`f147`, validated against all 179 baseline
//! blobs, 1,831 frames):
//!
//! - Header (8 bytes): `[flags: u32 BE][count: u32 BE]`. Only the low byte of
//!   `flags` is meaningful — a small bitfield (`bit 0x04` = palette-remap needed,
//!   `bit 0x08` = a shared palette is appended after the records); `count` is the
//!   frame count.
//! - Then `count` frame records of **8 bytes** each, big-endian throughout:
//!   `[u16 mpd_index][u32 offset][u16 chunk_bitmask]`.
//!     * `mpd_index` selects the `<stem>_<k>.mpd` file (almost always `0` — only
//!       `_0.mpd` ships).
//!     * `offset` is the byte offset of this frame's sub-image inside that `.mpd`;
//!       it lands exactly on a stripped-PNG `IHDR` chunk (see the cross-format
//!       `mph -> mpd` corpus check).
//!     * `chunk_bitmask` records which optional PNG chunks the frame carries.
//! - Optional trailer: when `flags & 0x08`, a shared `PLTE` (+`tRNS`) PNG chunk
//!   stream is appended verbatim after the records, running to EOF. The strict
//!   "`8 + count*8 == len`" equality therefore holds only without a trailer; with
//!   one, the trailing bytes must form a well-formed chunk stream that reconciles
//!   exactly to EOF. Truncated records or a malformed trailer are rejected.

use crate::{FormatError, Reader};

/// One decoded 8-byte `.mph` frame record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MphRecord {
    /// Which `<stem>_<k>.mpd` file this frame lives in (`k`).
    pub mpd_index: u16,
    /// Byte offset of the frame's sub-image (an `IHDR`) within that `.mpd`.
    pub offset: u32,
    /// Bitmask of the optional PNG chunks this frame carries.
    pub chunk_bitmask: u16,
}

/// Header + decoded frame records of an `.mph`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mph {
    /// Raw 32-bit header word preceding the count (a small flag bitfield).
    pub flags: u32,
    /// Declared frame count.
    pub count: u32,
    /// The `count` decoded 8-byte frame records.
    pub frames: Vec<MphRecord>,
    /// Number of trailing bytes after the records (the PNG palette stream, if any).
    pub trailer_len: usize,
}

/// Parse an `.mph` blob. See [module docs](self).
pub fn parse(input: &[u8]) -> Result<Mph, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);
    let flags = r.u32_be("mph flags")?;
    let count = r.u32_be("mph count")?;

    let mut frames = Vec::new();
    for _ in 0..count {
        let rec = r.take(8, "mph frame record")?;
        frames.push(MphRecord {
            mpd_index: u16::from_be_bytes([rec[0], rec[1]]),
            offset: u32::from_be_bytes([rec[2], rec[3], rec[4], rec[5]]),
            chunk_bitmask: u16::from_be_bytes([rec[6], rec[7]]),
        });
    }

    // Any remaining bytes must be a valid PNG chunk stream to EOF (the appended
    // shared palette). Absent a trailer, this loop does not run.
    let trailer_start = r.pos();
    while !r.is_empty() {
        let len = r.u32_be("mph trailer chunk length")? as usize;
        r.skip(4, "mph trailer chunk type")?;
        r.skip(len, "mph trailer chunk data")?;
        r.skip(4, "mph trailer chunk crc")?;
    }
    let trailer_len = input.len() - trailer_start;

    Ok(Mph {
        flags,
        count,
        frames,
        trailer_len,
    })
}
