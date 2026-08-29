//! Item tables — `itm/00` … `itm/23`, `itm/forshop`, and `itm/mixtbl`.
//!
//! ## `itm/NN` and `itm/forshop` — length-framed record tables (loader `ce.m69a`)
//!
//! Layout (validated 25/25): the file is a sequence of records `[u8 rec_len]` +
//! `rec_len` bytes, tiling exactly to EOF (`Σ (1 + rec_len) == file_len`).
//! [`parse`] validates that framing and returns each record's raw content.
//!
//! Inside a record the content is *heterogeneous* and is deliberately **not**
//! segmented here: `itm/NN` records begin with a type byte then two length-prefixed
//! ascii id refs (`[type][u8 len][ascii digits][u8 len][ascii digits]` + stats,
//! often `0xFF`-terminated), whereas `itm/forshop` records omit the leading type
//! byte and start directly at the first id ref. Because the stats section length
//! varies, Phase 1 pins only the outer framing (which is universal) and keeps the
//! record bytes raw; per-field decoding is a later phase.
//!
//! **Id indirection (do not resolve to text in Phase 1).** The embedded id refs
//! — like the ascii strings in [`crate::tdf`] — are ASCII-decimal integers that
//! index the `lang` string table ([`crate::lang`]); the item's on-screen
//! name/description text is whatever that table (the mislabeled `language.fr-FR`,
//! English in the EN build) holds at those ids. This crate encodes the structural
//! indirection only and never resolves an id to its string content.
//!
//! ## `itm/mixtbl` — crafting/mix table (loader `ad`)
//!
//! Layout (validated, 15 entries tiling to EOF): a sequence of entries
//! `[u8 ingredient_count]` + `ingredient_count` × `[u8 type][u8 subtype]` +
//! `[u8 result_type][u8 result_subtype]`. Parsed by [`parse_mixtbl`].

use crate::{FormatError, Reader};

/// A parsed `itm/NN` / `itm/forshop` table: its records' raw content, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Raw content of each `[u8 rec_len]`-framed record, tiling to EOF.
    pub records: Vec<Vec<u8>>,
}

/// Parse an `itm/NN` or `itm/forshop` blob. See [module docs](self).
pub fn parse(input: &[u8]) -> Result<Item, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);
    let mut records = Vec::new();
    while !r.is_empty() {
        let len = r.u8("item record length")? as usize;
        let content = r.take(len, "item record content")?.to_vec();
        records.push(content);
    }
    if records.is_empty() {
        return Err(FormatError::Inconsistent {
            what: "item table has no records",
        });
    }
    Ok(Item { records })
}

/// One crafting recipe from `itm/mixtbl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixEntry {
    /// The `[type, subtype]` pairs of the required ingredients.
    pub ingredients: Vec<[u8; 2]>,
    /// The crafted result as `[result_type, result_subtype]`.
    pub result: [u8; 2],
}

/// A parsed `itm/mixtbl` crafting table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixTable {
    /// The recipes, in file order.
    pub entries: Vec<MixEntry>,
}

/// Parse the `itm/mixtbl` crafting table. See [module docs](self).
pub fn parse_mixtbl(input: &[u8]) -> Result<MixTable, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);
    let mut entries = Vec::new();
    while !r.is_empty() {
        let count = r.u8("mixtbl ingredient count")? as usize;
        let mut ingredients = Vec::with_capacity(count);
        for _ in 0..count {
            let p = r.take(2, "mixtbl ingredient")?;
            ingredients.push([p[0], p[1]]);
        }
        let res = r.take(2, "mixtbl result")?;
        entries.push(MixEntry {
            ingredients,
            result: [res[0], res[1]],
        });
    }
    if entries.is_empty() {
        return Err(FormatError::Inconsistent {
            what: "mixtbl has no entries",
        });
    }
    Ok(MixTable { entries })
}
