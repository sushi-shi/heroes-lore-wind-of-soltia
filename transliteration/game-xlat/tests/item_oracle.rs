//! Item-record cross-check oracle ("two implementations, one truth").
//!
//! The strict transliteration of the item hierarchy
//! ([`heroes_lore_wind_of_soltia_game_xlat::item`] / `equipment` / `armor` /
//! `weapon`, `ad -> e -> t -> l`) parses every real `itm/NN` record from
//! `_originals/…v207.jar`. It is cross-checked against the INDEPENDENT,
//! separately-reversed record parser in
//! [`heroes_lore_wind_of_soltia_formats::item`] (framing) AND an in-test,
//! separately-coded field walk:
//!
//!  1. `heroes_lore_wind_of_soltia_formats::item::parse` frames each file into its
//!     `[u8 recLen][recLen bytes]` records — the independent record boundaries.
//!  2. The transliteration's `parse_record(item, .., false, content, 1)` (the same
//!     dispatch `Item.create`/`load` reaches) must consume EXACTLY `content.len()`
//!     — the field-driven parser agrees with the independent framing on every blob.
//!  3. An in-test field walk (name/desc lang-id, little-endian price, and the
//!     equipment/armor/weapon stat bytes) is re-derived independently and must equal
//!     the transliteration's parsed fields.
//!
//! Non-vacuity (GATES.md R3): >= 24 `itm/NN` files, a record-count floor, and every
//! concrete subclass (Item/Equipment/Armor/Weapon) exercised. A proven-red negative
//! control (one appended byte) turns the frame agreement false. Loud failure when
//! `_originals/` is absent (the `common` JAR helper panics).

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::byte_util::ByteUtilState;
use heroes_lore_wind_of_soltia_game_xlat::item::{self, ItemClass};

/// Every `itm/NN` (NN two decimal digits, 00..23), sorted, from the baseline JAR.
fn item_files() -> Vec<(i8, Vec<u8>)> {
    let mut out: Vec<(i8, Vec<u8>)> = jar()
        .matching(|n| {
            let bytes = n.as_bytes();
            bytes.len() == 6
                && n.starts_with("itm/")
                && bytes[4].is_ascii_digit()
                && bytes[5].is_ascii_digit()
        })
        .into_iter()
        .map(|(n, b)| (n[4..6].parse::<i8>().expect("itm/NN number"), b))
        .collect();
    out.sort_by_key(|(num, _)| *num);
    out
}

/// The transliteration's consumed length for one record: build the typed item
/// (the game's `create` switch) and drive the virtual `parse_record` at offset 1
/// (skipping the leading redundant type byte), returning `(item, consumed_end)`.
fn translit_parse(file_num: i8, record_index: usize, content: &[i8]) -> (item::Item, i32) {
    let mut bu = ByteUtilState::seeded(0); // rollEnchants=false → RNG untouched.
    let mut it = item::construct_for_type(file_num, record_index as i8);
    let end = item::parse_record(&mut it, &mut bu, false, content, 1);
    (it, end)
}

/// Independent field walk (separately coded from the transliteration): the expected
/// consumed length and the base + subclass fields decoded straight from `content`.
struct Independent {
    end: usize,
    name: Vec<u16>,
    desc: Vec<u16>,
    price: i32,
    value: Option<i16>,
    level_req: Option<i8>,
    attribute: Option<i8>,
    accuracy: Option<i8>,
    crit_bonus: Option<i8>,
}

fn independent_walk(file_num: i8, content: &[u8]) -> Independent {
    // Layout begins at offset 1 (content[0] is the redundant type byte).
    let mut off = 1usize;
    // name: [u8 len][len ascii]
    let name_len = content[off] as usize;
    off += 1;
    let name: Vec<u16> = content[off..off + name_len]
        .iter()
        .map(|&b| b as u16)
        .collect();
    off += name_len;
    // description: [u8 len][len ascii]
    let desc_len = content[off] as usize;
    off += 1;
    let desc: Vec<u16> = content[off..off + desc_len]
        .iter()
        .map(|&b| b as u16)
        .collect();
    off += desc_len;
    // price: 4 bytes little-endian
    let price = (content[off] as u32)
        | ((content[off + 1] as u32) << 8)
        | ((content[off + 2] as u32) << 16)
        | ((content[off + 3] as u32) << 24);
    off += 4;
    let after_price = off;

    let (mut value, mut level_req, mut attribute, mut accuracy, mut crit_bonus) =
        (None, None, None, None, None);
    let end = match file_num {
        // Weapon (0,1,2): equip stats (12) + accuracy + critBonus + attribute.
        0..=2 => {
            value = Some(content[after_price] as i16);
            level_req = Some(content[after_price + 1] as i8);
            accuracy = Some(content[after_price + 12] as i8);
            crit_bonus = Some(content[after_price + 13] as i8);
            attribute = Some(content[after_price + 14] as i8);
            after_price + 12 + 3
        }
        // Armor (3): equip stats (12) + attribute.
        3 => {
            value = Some(content[after_price] as i16);
            level_req = Some(content[after_price + 1] as i8);
            attribute = Some(content[after_price + 12] as i8);
            after_price + 12 + 1
        }
        // Equipment (4,5,6): equip stats (12).
        4..=6 => {
            value = Some(content[after_price] as i16);
            level_req = Some(content[after_price + 1] as i8);
            after_price + 12
        }
        // Base Item: name + desc + price only.
        _ => after_price,
    };

    Independent {
        end,
        name,
        desc,
        price: price as i32,
        value,
        level_req,
        attribute,
        accuracy,
        crit_bonus,
    }
}

#[test]
fn item_records_cross_check_agrees_over_the_real_corpus() {
    let files = item_files();
    assert!(
        files.len() >= 24,
        "non-vacuity: expected >= 24 itm/NN files, found {} — corpus is wrong",
        files.len()
    );

    let mut total_records: usize = 0;
    let mut saw_weapon = false;
    let mut saw_armor = false;
    let mut saw_equipment = false;
    let mut saw_item = false;
    // Liveness: at least one record whose parse did real work (non-empty name).
    let mut saw_named = false;

    for (file_num, bytes) in &files {
        // Independent framing (the second implementation): record boundaries.
        let parsed = heroes_lore_wind_of_soltia_formats::item::parse(bytes)
            .unwrap_or_else(|e| panic!("formats::item::parse failed for itm/{file_num:02}: {e:?}"));

        for (record_index, content_u8) in parsed.records.iter().enumerate() {
            let content = to_i8(content_u8);

            // content[0] is a leading byte that `Item.load` skips (parseRecord starts
            // at offset 1). Its meaning is not part of the parse contract, so it is
            // NOT asserted here (empirically it is not always the file number).

            let (it, end) = translit_parse(*file_num, record_index, &content);
            let ind = independent_walk(*file_num, content_u8);

            // 1. Frame agreement: transliteration consumes exactly the framed record,
            //    AND the independent walk lands on the same end — both == content.len().
            assert_eq!(
                end as usize,
                content.len(),
                "itm/{file_num:02} record {record_index}: transliteration consumed {end} != record length {}",
                content.len()
            );
            assert_eq!(
                ind.end,
                content.len(),
                "itm/{file_num:02} record {record_index}: independent walk end {} != record length {}",
                ind.end,
                content.len()
            );

            // 2. Field agreement (base fields, present on every item).
            assert_eq!(
                it.name, ind.name,
                "itm/{file_num:02} rec {record_index}: name id mismatch"
            );
            assert_eq!(
                it.description, ind.desc,
                "itm/{file_num:02} rec {record_index}: description id mismatch"
            );
            assert_eq!(
                it.price, ind.price,
                "itm/{file_num:02} rec {record_index}: price mismatch"
            );

            // 3. Subclass fields.
            match it.class {
                ItemClass::Item => saw_item = true,
                ItemClass::Equipment => saw_equipment = true,
                ItemClass::Armor => saw_armor = true,
                ItemClass::Weapon => saw_weapon = true,
            }
            if let Some(v) = ind.value {
                assert_eq!(
                    it.value, v,
                    "itm/{file_num:02} rec {record_index}: value mismatch"
                );
            }
            if let Some(lr) = ind.level_req {
                assert_eq!(
                    it.level_req, lr,
                    "itm/{file_num:02} rec {record_index}: levelReq mismatch"
                );
            }
            if let Some(a) = ind.attribute {
                assert_eq!(
                    it.attribute, a,
                    "itm/{file_num:02} rec {record_index}: attribute mismatch"
                );
            }
            if let Some(a) = ind.accuracy {
                assert_eq!(
                    it.accuracy, a,
                    "itm/{file_num:02} rec {record_index}: accuracy mismatch"
                );
            }
            if let Some(c) = ind.crit_bonus {
                assert_eq!(
                    it.crit_bonus, c,
                    "itm/{file_num:02} rec {record_index}: critBonus mismatch"
                );
            }

            if !it.name.is_empty() {
                saw_named = true;
            }
            total_records += 1;
        }
    }

    // Non-vacuity floors and hierarchy coverage (baseline: 24 files, 278 records).
    assert!(
        total_records >= 250,
        "non-vacuity: only {total_records} itm records parsed (floor 250, baseline 278)"
    );
    assert!(
        saw_named,
        "liveness: no record produced a non-empty name id"
    );
    assert!(saw_weapon, "hierarchy: no Weapon (itm/00..02) exercised");
    assert!(saw_armor, "hierarchy: no Armor (itm/03) exercised");
    assert!(
        saw_equipment,
        "hierarchy: no Equipment (itm/04..06) exercised"
    );
    assert!(saw_item, "hierarchy: no base Item exercised");

    println!(
        "item_oracle: {} itm/NN files, {} records cross-checked (Weapon/Armor/Equipment/Item all present)",
        files.len(),
        total_records
    );
}

#[test]
fn negative_control_one_appended_byte_breaks_frame_agreement() {
    // Take a real, agreeing record and append ONE byte. The transliteration still
    // consumes the record's true length (driven by its internal fields), so the
    // "consumed == length" frame agreement that held for the real record now FAILS
    // — proving the gate is red-sensitive, not vacuously green.
    let files = item_files();
    let (file_num, bytes) = files
        .iter()
        .find(|(n, _)| *n == 7) // itm/07: a single base-Item record (usable).
        .expect("itm/07 present");
    let parsed = heroes_lore_wind_of_soltia_formats::item::parse(bytes).expect("parse itm/07");
    let content = to_i8(&parsed.records[0]);

    // Sanity: the unperturbed record agrees.
    let (_, good_end) = translit_parse(*file_num, 0, &content);
    assert_eq!(
        good_end as usize,
        content.len(),
        "control record must agree first"
    );

    // Perturb: append one byte (a one-unit perturbation).
    let mut perturbed = content.clone();
    perturbed.push(0);
    let (_, bad_end) = translit_parse(*file_num, 0, &perturbed);
    assert_ne!(
        bad_end as usize,
        perturbed.len(),
        "negative control FAILED: frame agreement survived a one-byte perturbation"
    );
    println!(
        "item_oracle negative control: consumed {bad_end} != perturbed length {} (gate is red-sensitive)",
        perturbed.len()
    );
}
