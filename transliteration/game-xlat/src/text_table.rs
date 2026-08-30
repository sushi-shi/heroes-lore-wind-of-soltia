//! Transliterated from `java/src/main/java/defpackage/TextTable.java`
//! (original `z.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! A parsed `.tdf` table-def: a compact list mapping a small local index to a
//! global [`StringTable`](crate::string_table) string id. The file is `[u8 count]`
//! then `count` records of `[u16 BE len][len ASCII bytes]`, where the ASCII bytes
//! are the decimal text of the string id. The table holds no text itself — [`get`]
//! resolves an entry through the loaded `StringTable` at read time. UI screens
//! (shop, blacksmith, help, hero/guardian panels, the class-select chain's labels,
//! …) each load their own `.tdf` and index it by button/line number.
//!
//! `TextTable` has **no `static` fields** — `stringIds`/`count` are per-instance —
//! so it contributes no `java/reconstruction/ownership.tsv` rows (see the obf-class
//! map note there). A [`TextTableState`] instance is carried by whatever owns a
//! `TextTable` (`AssetCache.commonText` / `heroText`, `Item.typeNames`,
//! `Armor.attributeNames`); those owners are not yet ported, so no `Game` field
//! holds one this increment.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `z.<init>:(Ljava/lang/String;)V => [iinc,iand,i2s,iinc,iand,ishl,iinc,iand,iadd,iadd,iinc]`
//! (constructor), `z.a:(I)[C => []` (get — no arithmetic).

use crate::asset_cache;
use crate::game::Game;
use crate::string_table;
use j2me_jvm::ishl;

/// Java `z` / `TextTable` instance state. `string_ids` (`int[] stringIds`) holds
/// the global `StringTable` id for each local entry; `count` (`short count`) is the
/// leading `u8` count byte of the `.tdf`.
#[derive(Debug, Default, Clone)]
pub struct TextTableState {
    /// `private int[] stringIds;` — resolved string id per local entry.
    pub string_ids: Vec<i32>,
    /// `public short count;` — number of entries.
    pub count: i16,
}

/// `public TextTable(String basePath) throws IOException`
/// (`z.<init>:(Ljava/lang/String;)V`): reads `basePath + ".tdf"` and parses the
/// `[u8 count]` header + `count` `[u16 BE len][len ASCII]` records into
/// [`TextTableState::string_ids`]. The `IOException` propagates on device (an
/// uncaught throw would terminate); here `AssetCache.readResource` panics loud if
/// the resource is absent, matching an uncaught read failure.
pub fn construct(g: &mut Game, base_path: &str) -> TextTableState {
    // byte[] data = AssetCache.readResource(new StringBuffer().append(basePath).append(".tdf").toString());
    let path = format!("{base_path}.tdf");
    let data: Vec<i8> = asset_cache::read_resource(g, &path)
        .unwrap_or_else(|| panic!("TextTable: resource not found: {path}"));
    // int pos = 0 + 1;
    let mut pos: i32 = 0i32.wrapping_add(1);
    // this.count = (short) (data[0] & 255);
    let count: i16 = ((data[0] as i32) & 255) as i16;
    // this.stringIds = new int[this.count];
    let mut string_ids: Vec<i32> = vec![0i32; count as usize];
    // for (int i = 0; i < this.count; i++) { ... }
    let mut i: i32 = 0;
    while i < (count as i32) {
        // int lenHi = pos;
        let len_hi: i32 = pos;
        // int lenLo = pos + 1;
        let len_lo: i32 = pos.wrapping_add(1);
        // int textStart = lenLo + 1;
        let text_start: i32 = len_lo.wrapping_add(1);
        // int asciiLen = ((data[lenHi] & 255) << 8) + (data[lenLo] & 255);
        let ascii_len: i32 = ishl((data[len_hi as usize] as i32) & 255, 8)
            .wrapping_add((data[len_lo as usize] as i32) & 255);
        // this.stringIds[i] = Integer.parseInt(new String(data, textStart, asciiLen).trim());
        string_ids[i as usize] = parse_ascii_int(&data, text_start, ascii_len);
        // pos = textStart + asciiLen;
        pos = text_start.wrapping_add(ascii_len);
        // i++
        i = i.wrapping_add(1);
    }
    TextTableState { string_ids, count }
}

/// `public final char[] get(int index)` (`z.a:(I)[C => []`): resolves local entry
/// `index` to its localized string via the loaded `StringTable`, converting the
/// `';'` record separator to a newline and returning it as UTF-16 code units (the
/// `char[]` form the `FontManager` renderer consumes).
pub fn get(g: &Game, t: &TextTableState, index: i32) -> Vec<u16> {
    // StringTable.instance.get(this.stringIds[index])
    let s: Vec<u16> = string_table::get(&g.string_table, t.string_ids[index as usize]);
    // .replace(';', '\n')  — ';' == 59, '\n' == 10 (over UTF-16 code units)
    // .toCharArray()
    s.into_iter()
        .map(|c| if c == 59 { 10 } else { c })
        .collect()
}

/// `Integer.parseInt(new String(data, offset, length).trim())` — `new String(byte[],
/// int, int)` decodes with the platform default charset; the `.tdf` records are
/// ASCII decimal digits, so each byte maps 1:1 to its Latin-1 char. An unparsable
/// record would throw `NumberFormatException` in the (unguarded) constructor — an
/// uncaught throw terminates, reproduced here as a panic.
fn parse_ascii_int(data: &[i8], text_start: i32, ascii_len: i32) -> i32 {
    let mut s = String::with_capacity(ascii_len as usize);
    let mut k: i32 = 0;
    while k < ascii_len {
        // data[textStart + k] as an unsigned byte → Latin-1 char (ASCII digits identical).
        let b: u8 = data[(text_start.wrapping_add(k)) as usize] as u8;
        s.push(b as char);
        k = k.wrapping_add(1);
    }
    s.trim()
        .parse::<i32>()
        .unwrap_or_else(|_| panic!("TextTable: NumberFormatException parsing {s:?}"))
}
