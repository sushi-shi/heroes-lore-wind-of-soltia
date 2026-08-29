//! Bare-payload sprite scripts — the `.eif`, `*/atef/*`, `enm/die/*` and
//! `c3/s/ea*` family (loader `ce.a(true, bytes, 0, …)`).
//!
//! Layout (reversed from `ce`, validated 39/39 across the baseline: 18 `.eif`,
//! 7 `boss/atef`, 8 `enm/atef`, 3 `enm/die`, 3 `c3/s/ea`): the whole file **is**
//! one [`cell`](crate::cell) group list — `[u8 group_count]` then `group_count`
//! groups, each `[u8 cell_count]` then `cell_count` × 4-byte CELL — consuming
//! exactly the file. There is no record wrapper (that is what makes it "bare").
//!
//! The earlier Phase-1 recon modelled `.eif` as fixed `i8` pairs / a `0x00`-marker
//! count; both are wrong (no fixed-record model reconciles, and `0x00` is a
//! legitimate `dx`/`dy` delta, not a terminator). The group-list grammar above is
//! the real layout and is shared with `sprite` payloads and the `csprite` BARE
//! variant. A truncated file (a group promising more cells than remain) or one
//! with trailing bytes is rejected.

use crate::cell::Group;
use crate::FormatError;

/// A parsed bare-payload effect/sprite script (one group list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eif {
    /// The decoded groups (`[u8 cell_count]` + cells), in file order.
    pub groups: Vec<Group>,
}

/// Parse a bare-payload blob (`.eif`/`atef`/`die`/`ea`). See [module docs](self).
pub fn parse(input: &[u8]) -> Result<Eif, FormatError> {
    Ok(Eif {
        groups: crate::cell::parse_bare(input)?,
    })
}

impl Eif {
    /// Total number of cells across all groups.
    pub fn cell_count(&self) -> usize {
        self.groups.iter().map(|g| g.cells.len()).sum()
    }
}
