//! Sprite-assembly scripts — the extensionless `*/spr/*` blobs
//! (`enm/spr/*` 32, `npc/spr/*` 16, `boss/spr/*` 9 — loader `ce.e()`).
//!
//! Layout (validated 57/57, outer framing **and** inner payload): each file is a
//! flat stream of length-prefixed records that reconciles exactly to EOF:
//!
//! ```text
//! record  := [grp: u8] [idx: u8] [len: u8] [payload: len bytes]
//! payload := [u8 group_count] + group_count × ( [u8 cell_count] + cell_count × CELL )
//! ```
//!
//! `grp`/`idx` group and index the animation frame; the payload is a
//! [`cell`](crate::cell) group list — the same grammar as the `.eif` bare-payload
//! family — and consumes exactly its `len` bytes. The earlier recon note that the
//! payload used "5-byte parts" was wrong; the corpus confirms 4-byte cells across
//! all 57 files. Cell fields (`dx`/`dy`/variant/img) are kept raw for later phases.
//!
//! Scope note: the `c*/s/*` character sprites use a *different*, suffix-routed
//! container and live in [`crate::csprite`], not here.

use crate::cell::{parse_bare, Group};
use crate::{FormatError, Reader};

/// One sprite-assembly record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteRecord {
    /// Frame group id.
    pub grp: u8,
    /// Frame index within the group.
    pub idx: u8,
    /// Decoded frame-body groups (`[u8 cell_count]` + cells each).
    pub groups: Vec<Group>,
}

/// A parsed `*/spr/*` sprite-assembly script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    /// Records in file order.
    pub records: Vec<SpriteRecord>,
}

/// Parse a `*/spr/*` sprite-assembly blob. See [module docs](self).
pub fn parse(input: &[u8]) -> Result<Sprite, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);
    let mut records = Vec::new();
    while !r.is_empty() {
        let grp = r.u8("sprite record grp")?;
        let idx = r.u8("sprite record idx")?;
        let len = r.u8("sprite record len")? as usize;
        if len == 0 {
            return Err(FormatError::BadField {
                what: "sprite record payload length == 0",
            });
        }
        let payload = r.take(len, "sprite record payload")?;
        // The payload is itself a bare group list that must consume exactly `len`.
        let groups = parse_bare(payload)?;
        records.push(SpriteRecord { grp, idx, groups });
    }
    if records.is_empty() {
        return Err(FormatError::Inconsistent {
            what: "sprite has no records",
        });
    }
    Ok(Sprite { records })
}
