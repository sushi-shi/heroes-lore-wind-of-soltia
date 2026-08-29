//! Character-sprite scripts — the `c*/s/*` container (loader `bu`, looping over
//! `{"/c1/s/","/c2/s/","/c3/s/"}`).
//!
//! There are **three grammars**, routed by the file's basename suffix (not by any
//! in-file byte — see the risk note below). All three are built from the shared
//! 4-byte [`cell`](crate::cell) primitive and were validated 20/20 against the
//! baseline (14 FLAT, 3 NESTED, 3 BARE):
//!
//! - **FLAT** (`a`, `b`, `hA`, `hB`, `w`, `s` — 14 files): a stream of records
//!   `[u8 b8][u8 b9][u8 b10][u8 n]` + `n` × 4-byte CELL, tiling to EOF.
//! - **NESTED** (`e` — 3 files: `c1/s/e`, `c2/s/e`, `c3/s/e`; the `bu` `b3==2`
//!   branch): records `[u8 b8][u8 b9][u8 b10][u8 n]` + `n` × group, where each
//!   group is `[u8 cell_count]` + cells; tiling to EOF.
//! - **BARE** (`ea2`, `ea3`, `ea4` — 3 files): the [`crate::eif`] bare-payload
//!   grammar — the whole file is one group list.
//!
//! ⚠ **Suffix routing is mandatory.** The grammar is keyed by filename, not by a
//! discriminator byte, and the FLAT `a` files *also* parse cleanly under NESTED
//! (confirmed for `c3/s/a`). A caller must pick FLAT for `{a,b,hA,hB,w,s}`,
//! NESTED for `e`, and BARE for `ea*`; use [`variant_for_name`] to do so.

use crate::cell::{read_group, Cell, Group};
use crate::{FormatError, Reader};

/// Which `c*/s/*` grammar to parse a blob under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// `[b8][b9][b10][n]` + `n` cells per record (suffixes `a,b,hA,hB,w,s`).
    Flat,
    /// `[b8][b9][b10][n]` + `n` groups per record (suffix `e`).
    Nested,
    /// Whole file is one bare group list (suffixes `ea*`).
    Bare,
}

/// One FLAT record: a 3-byte header then a flat run of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatRecord {
    /// The record's 3 leading header bytes (`b8`, `b9`, `b10`).
    pub header: [u8; 3],
    /// The record's cells.
    pub cells: Vec<Cell>,
}

/// One NESTED record: a 3-byte header then a run of cell groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRecord {
    /// The record's 3 leading header bytes (`b8`, `b9`, `b10`).
    pub header: [u8; 3],
    /// The record's groups (`[u8 cell_count]` + cells each).
    pub groups: Vec<Group>,
}

/// A parsed `c*/s/*` blob, tagged by the grammar it was parsed under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharSprite {
    /// FLAT records.
    Flat(Vec<FlatRecord>),
    /// NESTED records.
    Nested(Vec<NestedRecord>),
    /// A single bare group list.
    Bare(Vec<Group>),
}

/// Pick the grammar for a `c*/s/*` blob from its zip-entry name (or basename).
/// `ea*` → BARE, exactly `e` → NESTED, everything else (`a,b,hA,hB,w,s`) → FLAT.
pub fn variant_for_name(name: &str) -> Variant {
    let base = name.rsplit('/').next().unwrap_or(name);
    if base.starts_with("ea") {
        Variant::Bare
    } else if base == "e" {
        Variant::Nested
    } else {
        Variant::Flat
    }
}

/// Read the common 3-byte header + `n` count of a FLAT/NESTED record.
fn read_record_head(r: &mut Reader<'_>) -> Result<([u8; 3], usize), FormatError> {
    let b = r.take(4, "c*/s record header")?;
    Ok(([b[0], b[1], b[2]], b[3] as usize))
}

/// Parse a `c*/s/*` blob under the given [`Variant`]. See [module docs](self).
pub fn parse(input: &[u8], variant: Variant) -> Result<CharSprite, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    match variant {
        Variant::Bare => Ok(CharSprite::Bare(crate::cell::parse_bare(input)?)),
        Variant::Flat => {
            let mut r = Reader::new(input);
            let mut records = Vec::new();
            while !r.is_empty() {
                let (header, n) = read_record_head(&mut r)?;
                let mut cells = Vec::with_capacity(n);
                for _ in 0..n {
                    let c = r.take(4, "c*/s flat cell")?;
                    cells.push([c[0], c[1], c[2], c[3]]);
                }
                records.push(FlatRecord { header, cells });
            }
            Ok(CharSprite::Flat(records))
        }
        Variant::Nested => {
            let mut r = Reader::new(input);
            let mut records = Vec::new();
            while !r.is_empty() {
                let (header, n) = read_record_head(&mut r)?;
                let mut groups = Vec::with_capacity(n);
                for _ in 0..n {
                    groups.push(read_group(&mut r, "c*/s nested group cell count")?);
                }
                records.push(NestedRecord { header, groups });
            }
            Ok(CharSprite::Nested(records))
        }
    }
}

/// Convenience: route by name, then parse.
pub fn parse_named(name: &str, input: &[u8]) -> Result<CharSprite, FormatError> {
    parse(input, variant_for_name(name))
}
