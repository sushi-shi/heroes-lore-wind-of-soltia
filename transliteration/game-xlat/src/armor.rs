//! Transliterated from `java/src/main/java/defpackage/Armor.java`
//! (original `t.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! `Armor extends Equipment implements Directions` (`t extends e`) — equipment
//! carrying a combat `attribute`. The `attribute` field lives on the flattened
//! [`crate::item::Item`]; these are the armor-specific methods. `super.parseRecord`
//! / `super.serialize` call the `Equipment` bodies directly.
//!
//! ## Deferred cross-class boundary
//!
//! `Armor.attributeNames` is a static `TextTable` (`z`) resolved through the lang
//! table (like `Item.typeNames`); `TextTable` is not ported, so the static and any
//! attribute-name resolution are DEFERRED (not needed by the record cross-check).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `t.a:(Z[BI)I => iinc` (parseRecord), `t.a:()[B => []` (serialize),
//! `t.<clinit>:()V => []` (the two static byte[] tables, no arithmetic).

use crate::byte_util::ByteUtilState;
use crate::equipment;
use crate::item::Item;

/// `public static final byte[] PROC_STATUS` — status effect per combat `attribute`.
pub const PROC_STATUS: [i8; 9] = [0, 1, -1, -1, -1, 4, 3, 2, -1];

/// `public static final byte[] PROC_CHANCE` — proc chance (percent) per `attribute`.
pub const PROC_CHANCE: [i8; 9] = [20, 16, 6, 13, 13, 10, 10, 10, 10];

/// `Armor.parseRecord` (`t.a:(Z[BI)I => iinc`). `super.parseRecord` is
/// `Equipment.parseRecord`; then read the trailing `attribute` byte.
pub fn parse_record(
    item: &mut Item,
    byte_util: &mut ByteUtilState,
    roll_enchants: bool,
    data: &[i8],
    offset: i32,
) -> i32 {
    // int afterEquip = super.parseRecord(rollEnchants, data, offset);
    let after_equip = equipment::parse_record(item, byte_util, roll_enchants, data, offset);
    // int end = afterEquip + 1;
    let end = after_equip.wrapping_add(1);
    // this.attribute = data[afterEquip];
    item.attribute = data[after_equip as usize];
    // return end;
    end
}

/// `Armor.serialize` (`t.a:()[B => []`, `final`). `super.serialize` is
/// `Equipment.serialize`; then write the `attribute` byte. `Weapon` inherits this.
pub fn serialize(item: &Item) -> Vec<i8> {
    // byte[] out = super.serialize();
    let mut out = equipment::serialize(item);
    // out[9] = this.attribute;
    out[9] = item.attribute;
    out
}
