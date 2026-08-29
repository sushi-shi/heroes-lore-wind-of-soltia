//! Transliterated from `java/src/main/java/defpackage/Weapon.java`
//! (original `l.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! `Weapon extends Armor` (`l extends t`) — a wieldable armor adding `accuracy`
//! and `critBonus` (both on the flattened [`crate::item::Item`]). Its record layout
//! reads name/description/price and the equipment stats, then the two weapon bytes,
//! then the shared `Armor.attribute`, so it OVERRIDES `parseRecord` fully (calling
//! the `final` base/`Equipment` field parsers directly) rather than chaining through
//! `Armor`. It has no `serialize` override — it inherits `Armor.serialize`.
//!
//! Opcode shape (R8, `_reference/numeric_shapes.json`):
//! `l.a:(Z[BI)I => iadd,iadd,iadd,iadd,iinc,iinc,iinc` (iadd×4 for the four
//! `offset + parseX(...)` steps, iinc×3 for the three trailing byte reads).

use crate::byte_util::ByteUtilState;
use crate::equipment;
use crate::item::{self, Item};

/// `Weapon.parseRecord` (`l.a:(Z[BI)I`) — full override; does not call `super`.
pub fn parse_record(
    item: &mut Item,
    byte_util: &mut ByteUtilState,
    roll_enchants: bool,
    data: &[i8],
    offset: i32,
) -> i32 {
    // int afterName = offset + parseName(data, offset);
    let after_name = offset.wrapping_add(item::parse_name(item, data, offset));
    // int afterDesc = afterName + parseDescription(data, afterName);
    let after_desc = after_name.wrapping_add(item::parse_description(item, data, after_name));
    // int afterPrice = afterDesc + parsePrice(data, afterDesc);
    let after_price = after_desc.wrapping_add(item::parse_price(item, data, after_desc));
    // int afterEquip = afterPrice + parseEquipStats(data, afterPrice, rollEnchants);
    let after_equip = after_price.wrapping_add(equipment::parse_equip_stats(
        item,
        byte_util,
        data,
        after_price,
        roll_enchants,
    ));
    // int p = afterEquip + 1; this.accuracy = data[afterEquip];
    let p = after_equip.wrapping_add(1);
    item.accuracy = data[after_equip as usize];
    // int p2 = p + 1; this.critBonus = data[p];
    let p2 = p.wrapping_add(1);
    item.crit_bonus = data[p as usize];
    // int end = p2 + 1; this.attribute = data[p2];
    let end = p2.wrapping_add(1);
    item.attribute = data[p2 as usize];
    // return end;
    end
}
