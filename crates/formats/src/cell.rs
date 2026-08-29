//! Shared sprite-cell grammar used by the sprite/effect/character-sprite families.
//!
//! Several formats are built from the same two primitives (loader `ce`):
//!
//! - **CELL** — a fixed 4-byte record `[i8 dx][i8 dy][u8 variant_flag][u8 img_index]`.
//!   `dx`/`dy` are signed pixel offsets; `variant_flag != 0` selects the
//!   palette-swapped image bank, else the base bank; `img_index` is a frame index
//!   into the paired `.mph`/`.mpd` atlas. Phase 1 keeps the raw 4 bytes and does
//!   not interpret them further (draw order / bank selection is a later phase).
//! - **group list** (the "bare payload") — `[u8 group_count]` then `group_count`
//!   groups, each `[u8 cell_count]` then `cell_count` CELLs.
//!
//! This module holds those primitives once so `sprite` (`*/spr/*` payloads),
//! `eif` (the `.eif`/`atef`/`die`/`ea` bare-payload family) and `csprite`
//! (`c*/s/*`) all decode cells identically. The grammar was validated against the
//! whole baseline corpus (57 spr payloads, 39 bare-payload files, 20 `c*/s/*`).

use crate::{FormatError, Reader};

/// A 4-byte sprite cell: `[i8 dx][i8 dy][u8 variant_flag][u8 img_index]` (raw).
pub type Cell = [u8; 4];

/// One group of cells: a `[u8 cell_count]`-prefixed run of [`Cell`]s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The cells of this group, in file order.
    pub cells: Vec<Cell>,
}

/// Read one group `[u8 cell_count]` + `cell_count` × 4-byte CELL from `r`.
pub(crate) fn read_group(r: &mut Reader<'_>, what: &'static str) -> Result<Group, FormatError> {
    let count = r.u8(what)? as usize;
    let mut cells = Vec::with_capacity(count);
    for _ in 0..count {
        let b = r.take(4, "cell")?;
        cells.push([b[0], b[1], b[2], b[3]]);
    }
    Ok(Group { cells })
}

/// Read a group list `[u8 group_count]` + `group_count` × group from `r`
/// (advancing the cursor; does not require it to reach EOF).
pub(crate) fn read_group_list(r: &mut Reader<'_>) -> Result<Vec<Group>, FormatError> {
    let count = r.u8("group count")? as usize;
    let mut groups = Vec::with_capacity(count);
    for _ in 0..count {
        groups.push(read_group(r, "group cell count")?);
    }
    Ok(groups)
}

/// Parse an entire slice as a **bare payload**: a single group list that must
/// consume exactly `input` (no trailing bytes). Rejects empty input, truncated
/// groups, and trailers.
pub fn parse_bare(input: &[u8]) -> Result<Vec<Group>, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);
    let groups = read_group_list(&mut r)?;
    if !r.is_empty() {
        return Err(FormatError::Inconsistent {
            what: "bare payload has trailing bytes after its group list",
        });
    }
    Ok(groups)
}
