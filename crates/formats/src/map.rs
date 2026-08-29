//! `.map` — tile map.
//!
//! Layout (confirmed against all 81 baseline blobs): a 3-byte header
//! `[ver: u8][w: u8][h: u8]` followed by exactly `w * h` tile-id bytes, so the
//! file length is `3 + w*h`. `ver` varies across the corpus (e.g. 1, 14), so the
//! parser does not pin it.

use crate::FormatError;

/// A parsed tile map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    /// Format/version byte (not interpreted).
    pub ver: u8,
    /// Map width in tiles.
    pub w: u8,
    /// Map height in tiles.
    pub h: u8,
    /// Row-major `w*h` tile ids.
    pub tiles: Vec<u8>,
}

/// Parse a `.map` blob. See [module docs](self).
pub fn parse(input: &[u8]) -> Result<Map, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    if input.len() < 3 {
        return Err(FormatError::Truncated {
            what: "map header",
            offset: 0,
            needed: 3,
            have: input.len(),
        });
    }
    let ver = input[0];
    let w = input[1];
    let h = input[2];
    // 3 + w*h cannot overflow usize (both u8), but keep it explicit.
    let expected = 3usize + (w as usize) * (h as usize);
    if input.len() != expected {
        return Err(FormatError::Inconsistent {
            what: "map length != 3 + w*h",
        });
    }
    let tiles = input[3..].to_vec();
    Ok(Map { ver, w, h, tiles })
}
