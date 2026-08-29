//! Transliterated from `java/src/main/java/defpackage/Equipment.java`
//! (original `e.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! `Equipment extends Item` (`e extends ad`) — an equippable item. Fields
//! (`value`/`levelReq`/`needsIdentify`/`identified`/`refineLevel`/`enchant[4]`)
//! live on the flattened [`crate::item::Item`] struct; these are the
//! equipment-specific methods that operate on it. `super.parseRecord` /
//! `super.serialize` call the base `Item` bodies directly
//! ([`crate::item::parse_record_base`] / [`crate::item::serialize_base`]).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `e.a:(Z[BI)I => iadd` (parseRecord),
//! `e.a:([BIZ)I => iinc,iand,i2s,iinc,iinc,iadd,iinc,iadd,iadd,iadd,iadd,iadd,iadd,iadd,iadd`
//! (parseEquipStats — iinc×4, iand×1, i2s×1, iadd×9), `e.a:()[B => []` (serialize),
//! `e.a:(B[B[BB)V => i2b,iinc` (rollEnchant), `e.a:(BBBB)V => []` (setEnchant).

use crate::byte_util::{self, ByteUtilState};
use crate::item::{self, Item};

/// `Equipment.parseRecord` (`e.a:(Z[BI)I => iadd`). `super.parseRecord` is
/// `Item.parseRecord`.
pub fn parse_record(
    item: &mut Item,
    byte_util: &mut ByteUtilState,
    roll_enchants: bool,
    data: &[i8],
    offset: i32,
) -> i32 {
    // int afterBase = super.parseRecord(rollEnchants, data, offset);
    let after_base = item::parse_record_base(item, data, offset);
    // return afterBase + parseEquipStats(data, afterBase, rollEnchants);
    after_base.wrapping_add(parse_equip_stats(
        item,
        byte_util,
        data,
        after_base,
        roll_enchants,
    ))
}

/// `public final int parseEquipStats(byte[] data, int offset, boolean rollEnchants)`
/// (`e.a:([BIZ)I`). Decodes value/levelReq/needsIdentify + the enchant min/max
/// ranges; rolls the actual enchants when `rollEnchants` and any range is non-zero.
/// Always consumes 12 bytes.
pub fn parse_equip_stats(
    item: &mut Item,
    byte_util: &mut ByteUtilState,
    data: &[i8],
    offset: i32,
    roll_enchants: bool,
) -> i32 {
    // int p = offset + 1;
    let p = offset.wrapping_add(1);
    // this.value = (short) (data[offset] & 255);
    item.value = ((data[offset as usize] as i32) & 255) as i16;
    // int p2 = p + 1;
    let p2 = p.wrapping_add(1);
    // this.levelReq = data[p];
    item.level_req = data[p as usize];
    // int rangeBase = p2 + 1;
    let range_base = p2.wrapping_add(1);
    // this.needsIdentify = data[p2] != 0;
    item.needs_identify = data[p2 as usize] != 0;
    // boolean hasEnchant = false;
    let mut has_enchant = false;
    // for (int i = 1; i <= 8; i++) { if (data[rangeBase + i] != 0) { hasEnchant = true; break; } }
    let mut i: i32 = 1;
    while i <= 8 {
        if data[range_base.wrapping_add(i) as usize] != 0 {
            has_enchant = true;
            break;
        }
        i = i.wrapping_add(1);
    }
    // if (!hasEnchant) this.identified = true;
    if !has_enchant {
        item.identified = true;
    }
    // if (!rollEnchants || !hasEnchant) return 12;
    if !roll_enchants || !has_enchant {
        return 12;
    }
    // rollEnchant(data[rangeBase],
    //             new byte[]{data[rangeBase+1], +3, +5, +7},
    //             new byte[]{data[rangeBase+2], +4, +6, +8}, (byte) 0);
    let mins: [i8; 4] = [
        data[range_base.wrapping_add(1) as usize],
        data[range_base.wrapping_add(3) as usize],
        data[range_base.wrapping_add(5) as usize],
        data[range_base.wrapping_add(7) as usize],
    ];
    let maxs: [i8; 4] = [
        data[range_base.wrapping_add(2) as usize],
        data[range_base.wrapping_add(4) as usize],
        data[range_base.wrapping_add(6) as usize],
        data[range_base.wrapping_add(8) as usize],
    ];
    roll_enchant(item, byte_util, data[range_base as usize], &mins, &maxs, 0);
    // return 12;
    12
}

/// `Equipment.serialize` (`e.a:()[B => []`). `super.serialize` is `Item.serialize`.
pub fn serialize(item: &Item) -> Vec<i8> {
    // byte[] out = super.serialize();
    let mut out = item::serialize_base(item);
    // out[3] = this.identified ? (byte) 1 : (byte) 0;
    out[3] = if item.identified { 1 } else { 0 };
    // out[4] = this.refineLevel;
    out[4] = item.refine_level;
    // System.arraycopy(this.enchant, 0, out, 5, 4);
    out[5..9].copy_from_slice(&item.enchant[0..4]);
    out
}

/// `public final void rollEnchant(byte count, byte[] mins, byte[] maxs, byte refine)`
/// (`e.a:(B[B[BB)V => i2b,iinc`). Rolls `count` enchants into random empty,
/// non-zero-range slots, then records the refine level.
pub fn roll_enchant(
    item: &mut Item,
    byte_util: &mut ByteUtilState,
    count: i8,
    mins: &[i8],
    maxs: &[i8],
    refine: i8,
) {
    // for (int i = 0; i < count; i++) {
    let mut i: i32 = 0;
    while i < count as i32 {
        // while (true) { slot = ByteUtil.randRange(0, 3);
        //   if (enchant[slot] == 0 && (mins[slot] != 0 || maxs[slot] != 0)) break; }
        let slot: i32 = loop {
            let s = byte_util::rand_range(byte_util, 0, 3);
            if item.enchant[s as usize] == 0 && (mins[s as usize] != 0 || maxs[s as usize] != 0) {
                break s;
            }
        };
        // this.enchant[slot] = (byte) ByteUtil.randRange(mins[slot], maxs[slot]);
        item.enchant[slot as usize] = byte_util::rand_range(
            byte_util,
            mins[slot as usize] as i32,
            maxs[slot as usize] as i32,
        ) as i8;
        i = i.wrapping_add(1);
    }
    // this.refineLevel = refine;
    item.refine_level = refine;
}

/// `public final void setEnchant(byte e0, byte e1, byte e2, byte e3)`
/// (`e.a:(BBBB)V => []`).
pub fn set_enchant(item: &mut Item, e0: i8, e1: i8, e2: i8, e3: i8) {
    item.enchant[0] = e0;
    item.enchant[1] = e1;
    item.enchant[2] = e2;
    item.enchant[3] = e3;
}
