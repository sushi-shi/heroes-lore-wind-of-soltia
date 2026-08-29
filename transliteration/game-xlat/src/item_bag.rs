//! Transliterated from `java/src/main/java/defpackage/ItemBag.java`
//! (original `g.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! A fixed-capacity item store (hero bag + quick-item bar). Java keeps an
//! `Item[] slots` and relies on **object-reference semantics**: `findItem`/`get`
//! hand out references that, when mutated, change the slot; `removeItem`/`slotOf`
//! compare with `==` (identity). A slot is therefore modelled as
//! `Option<Rc<RefCell<Item>>>` — the faithful transliteration of a shared, mutable
//! Java object reference — with identity via [`Rc::ptr_eq`].
//!
//! ## Preserved de-obfuscation fix — `mergeStacks`
//!
//! JADX rendered `mergeStacks` (`g.e`) with a broken loop (an infinite-loop
//! artifact); a prior de-obf pass restored the real **nested `for`** control flow
//! (outer `i` over `slots.length - 1`, inner `j` over `slots.length`). That real
//! structure is transliterated verbatim here and MUST be preserved.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `g.e:()V =>
//! isub,iadd,i2b,iadd,isub,i2b,iadd,i2b,iadd,i2b` (mergeStacks — R8 inlined
//! `addQuantity`/`removeQuantity`, each contributing an iadd/isub + i2b; the loop
//! `slots.length - 1`/`i + 1`/`have + other`/`99 - have`/`(byte) i` supply the
//! rest); `g.a:(Lad;I)Z => iadd,i2b,iinc,i2b` (add), `g.a:(BBB)V => isub,i2b,iinc`
//! (removeItems), `g.d:()V => isub,iadd,iinc,iinc` (compact), `g.b:()V => iadd,i2b`
//! (cycleQuickType), `g.c:()[B => iinc` (serialize — the `writeInt` shifts are the
//! JDK's `DataOutputStream`, not `g`), `g.a:([B)V => iinc,iinc` (deserialize). The
//! `DataInputStream`/`DataOutputStream` big-endian byte work is reproduced inline
//! (as `string_table.rs` reproduces `readInt`/`readUTF`).

use crate::game::Game;
use crate::item::{self, Item};
use j2me_jvm::ishr;
use std::cell::RefCell;
use std::rc::Rc;

/// A shared, mutable item reference — the transliteration of a Java `Item` object
/// reference stored in a slot (identity via [`Rc::ptr_eq`]).
pub type ItemRef = Rc<RefCell<Item>>;

/// `Debug.assertTrue(condition)` (`Debug.java:19` — a false assertion throws
/// `RuntimeException("ASSERT FAILED")`, here a panic).
fn assert_true(condition: bool) {
    if !condition {
        panic!("ASSERT FAILED"); // Debug.java:21
    }
}

/// Java `g` / `ItemBag`.
#[derive(Debug)]
pub struct ItemBag {
    /// `private Item[] slots;` — capacity fixed at construction. `None` == Java null.
    pub slots: Vec<Option<ItemRef>>,
    /// `private byte quickSlot;` — slot backing the quick bar (-1 = none).
    pub quick_slot: i8,
    /// `private byte quickTypeCursor;` — index into [`QUICK_TYPES`].
    pub quick_type_cursor: i8,
    /// `public int gold;`
    pub gold: i32,
}

/// `private static final byte[] QUICK_TYPES = {7, 8, 9, 10};`
const QUICK_TYPES: [i8; 4] = [7, 8, 9, 10];

/// `public ItemBag(byte capacity)` — `this.slots = new Item[capacity]; resetQuickSelection();`.
pub fn new(capacity: i8) -> ItemBag {
    let mut bag = ItemBag {
        slots: (0..capacity as usize).map(|_| None).collect(),
        quick_slot: 0,
        quick_type_cursor: 0,
        gold: 0,
    };
    reset_quick_selection(&mut bag);
    bag
}

/// `public final void resetQuickSelection()`.
pub fn reset_quick_selection(bag: &mut ItemBag) {
    // this.quickSlot = (byte) -1; this.quickTypeCursor = (byte) -1; cycleQuickType();
    bag.quick_slot = -1;
    bag.quick_type_cursor = -1;
    cycle_quick_type(bag);
}

/// `public final Item get(int index)` (`g.a:(I)Lad; => []`).
pub fn get(bag: &ItemBag, index: i32) -> Option<ItemRef> {
    // return this.slots[index];
    bag.slots[index as usize].clone()
}

/// `public final byte[] occupiedSlots()` (`g.a:()[B`).
pub fn occupied_slots(bag: &ItemBag) -> Vec<i8> {
    // int count = 0; for (i) if (slots[i] != null) count++;
    let mut count: i32 = 0;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if bag.slots[i as usize].is_some() {
            count = count.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    // byte[] out = new byte[count]; int w = 0; for (i) if (slots[i] != null) out[w++] = (byte) i;
    let mut out: Vec<i8> = vec![0i8; count as usize];
    let mut w: i32 = 0;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if bag.slots[i as usize].is_some() {
            out[w as usize] = i as i8;
            w = w.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    out
}

/// `public final byte[] equipmentSlots(boolean armorOnly, byte identifiedFilter)`.
pub fn equipment_slots(bag: &ItemBag, armor_only: bool, identified_filter: i8) -> Vec<i8> {
    // The inline predicate (duplicated in both passes of the Java): an Equipment
    // matching the armor/identified filters.
    let matches = |slot: &Option<ItemRef>| -> bool {
        match slot {
            Some(it) => {
                let b = it.borrow();
                item::is_equipment(&b)
                    && (!armor_only || item::is_armor(&b))
                    && (identified_filter != 1 || b.identified)
                    && (identified_filter != -1 || !b.identified)
            }
            None => false,
        }
    };
    let mut count: i32 = 0;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if matches(&bag.slots[i as usize]) {
            count = count.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    let mut out: Vec<i8> = vec![0i8; count as usize];
    let mut w: i32 = 0;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if matches(&bag.slots[i as usize]) {
            out[w as usize] = i as i8;
            w = w.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    out
}

/// `public final byte[] slotsOfType(byte type)` (`g.a:(B)[B`).
pub fn slots_of_type(bag: &ItemBag, r#type: i8) -> Vec<i8> {
    let mut count: i32 = 0;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if slot_type_is(bag, i, r#type) {
            count = count.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    let mut out: Vec<i8> = vec![0i8; count as usize];
    let mut w: i32 = 0;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if slot_type_is(bag, i, r#type) {
            out[w as usize] = i as i8;
            w = w.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    out
}

fn slot_type_is(bag: &ItemBag, i: i32, r#type: i8) -> bool {
    // slots[i] != null && slots[i].type == type
    match &bag.slots[i as usize] {
        Some(it) => it.borrow().r#type == r#type,
        None => false,
    }
}

/// `public final byte[] quickUsableSlots()` (`g.b:()[B`).
pub fn quick_usable_slots(bag: &ItemBag) -> Vec<i8> {
    let quick = |slot: &Option<ItemRef>| -> bool {
        // slots[i] != null && Item.QUICK_USABLE[slots[i].type]
        match slot {
            Some(it) => item::QUICK_USABLE[it.borrow().r#type as usize],
            None => false,
        }
    };
    let mut count: i32 = 0;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if quick(&bag.slots[i as usize]) {
            count = count.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    let mut out: Vec<i8> = vec![0i8; count as usize];
    let mut w: i32 = 0;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if quick(&bag.slots[i as usize]) {
            out[w as usize] = i as i8;
            w = w.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    out
}

/// `public final Item replaceAt(Item item, byte slot)` (`g.a:(Lad;B)Lad; => []`).
pub fn replace_at(bag: &mut ItemBag, item: ItemRef, slot: i8) -> Option<ItemRef> {
    // Item previous = this.slots[slot]; this.slots[slot] = item; return previous;
    bag.slots[slot as usize].replace(item)
}

/// `public final Item currentQuickItem()` (`g.a:()Lad; => []`).
pub fn current_quick_item(bag: &ItemBag) -> Option<ItemRef> {
    // if (this.quickSlot == -1) return null; return this.slots[this.quickSlot];
    if bag.quick_slot == -1 {
        return None;
    }
    bag.slots[bag.quick_slot as usize].clone()
}

/// `public final byte currentQuickType()` (`g.a:()B => []`).
pub fn current_quick_type(bag: &ItemBag) -> i8 {
    // return QUICK_TYPES[this.quickTypeCursor];
    QUICK_TYPES[bag.quick_type_cursor as usize]
}

/// `public final void cycleQuickType()` (`g.b:()V => iadd,i2b`).
pub fn cycle_quick_type(bag: &mut ItemBag) {
    // this.quickTypeCursor = (byte) (this.quickTypeCursor + 1);
    bag.quick_type_cursor = (bag.quick_type_cursor as i32).wrapping_add(1) as i8;
    // if (this.quickTypeCursor == 4) this.quickTypeCursor = (byte) 0;
    if bag.quick_type_cursor == 4 {
        bag.quick_type_cursor = 0;
    }
    // this.quickSlot = findSlot(QUICK_TYPES[this.quickTypeCursor], (byte) 0);
    bag.quick_slot = find_slot(bag, QUICK_TYPES[bag.quick_type_cursor as usize], 0);
}

/// `private final void revalidateQuickSlot()` (`g.f:()V => []`).
fn revalidate_quick_slot(bag: &mut ItemBag) {
    // if (this.quickSlot != -1) {
    if bag.quick_slot != -1 {
        // if (this.slots[this.quickSlot] == null) this.quickSlot = (byte) -1;
        if bag.slots[bag.quick_slot as usize].is_none() {
            bag.quick_slot = -1;
        } else if bag.slots[bag.quick_slot as usize]
            .as_ref()
            .unwrap()
            .borrow()
            .r#type
            != QUICK_TYPES[bag.quick_type_cursor as usize]
        {
            // else if (slots[quickSlot].type != QUICK_TYPES[cursor])
            //     this.quickSlot = findSlot(QUICK_TYPES[cursor], (byte) 0);
            bag.quick_slot = find_slot(bag, QUICK_TYPES[bag.quick_type_cursor as usize], 0);
        }
    }
}

/// `private final void ensureQuickSlot()` (`g.g:()V => []`).
fn ensure_quick_slot(bag: &mut ItemBag) {
    // if (this.quickSlot == -1) this.quickSlot = findSlot(QUICK_TYPES[cursor], (byte) 0);
    if bag.quick_slot == -1 {
        bag.quick_slot = find_slot(bag, QUICK_TYPES[bag.quick_type_cursor as usize], 0);
    }
}

/// `public final boolean add(Item item, int count)` (`g.a:(Lad;I)Z`).
pub fn add(bag: &mut ItemBag, item: ItemRef, count: i32) -> bool {
    // if (!canAdd(item.type, item.subId, count)) return false;
    let (item_type, item_sub_id) = {
        let b = item.borrow();
        (b.r#type, b.sub_id)
    };
    if !can_add(bag, item_type, item_sub_id, count) {
        return false;
    }
    // byte free = firstFreeSlot();
    let free = first_free_slot(bag);
    // if (!Item.STACKABLE[item.type]) { ... }
    if !item::STACKABLE[item_type as usize] {
        // if (free == -1) return false;
        if free == -1 {
            return false;
        }
        // Debug.assertTrue(item.quantity == 1);
        assert_true(item.borrow().quantity == 1);
        // this.slots[free] = item; return true;
        bag.slots[free as usize] = Some(item);
        return true;
    }
    // Item[] stacks = findAllItems(item.type, item.subId);
    let stacks = find_all_items(bag, item_type, item_sub_id);
    // for (int i = 0; i < stacks.length; i++) {
    let mut i: i32 = 0;
    while i < stacks.len() as i32 {
        // if (stacks[i].quantity + count <= 99) {
        if (stacks[i as usize].borrow().quantity as i32).wrapping_add(count) <= 99 {
            // stacks[i].addQuantity((byte) count);
            item::add_quantity(&mut stacks[i as usize].borrow_mut(), count as i8);
            // mergeStacks(); ensureQuickSlot(); return true;
            merge_stacks(bag);
            ensure_quick_slot(bag);
            return true;
        }
        i = i.wrapping_add(1);
    }
    // if (free == -1) return false;
    if free == -1 {
        return false;
    }
    // item.quantity = (byte) count;
    item.borrow_mut().quantity = count as i8;
    // this.slots[free] = item;
    bag.slots[free as usize] = Some(item);
    // mergeStacks(); ensureQuickSlot(); return true;
    merge_stacks(bag);
    ensure_quick_slot(bag);
    true
}

/// `public final void removeItems(byte type, byte subId, byte count)` (`g.a:(BBB)V`).
pub fn remove_items(bag: &mut ItemBag, r#type: i8, sub_id: i8, count: i8) {
    // for (int guard = this.slots.length; guard > 0 && count > 0; guard--) {
    let mut count: i8 = count;
    let mut guard: i32 = bag.slots.len() as i32;
    while guard > 0 && count > 0 {
        // Item found = findItem(type, subId);
        let found = find_item(bag, r#type, sub_id);
        // if (found != null) {
        if let Some(found) = found {
            // byte take = found.quantity < count ? found.quantity : count;
            let fq = found.borrow().quantity;
            let take: i8 = if fq < count { fq } else { count };
            // decrementItem(found, take);
            decrement_item(bag, &found, take);
            // count = (byte) (count - take);
            count = (count as i32).wrapping_sub(take as i32) as i8;
        }
        guard = guard.wrapping_sub(1);
    }
    // Debug.assertTrue(count == 0);
    assert_true(count == 0);
    // mergeStacks();
    merge_stacks(bag);
}

/// `public final void removeFromSlot(byte slot, byte count)` (`g.a:(BB)V => []`).
pub fn remove_from_slot(bag: &mut ItemBag, slot: i8, count: i8) {
    // decrementItem(this.slots[slot], count);
    let it = bag.slots[slot as usize]
        .clone()
        .expect("removeFromSlot on empty slot");
    decrement_item(bag, &it, count);
}

/// `public final void decrementItem(Item item, byte count)` (`g.a:(Lad;B)V => []`).
pub fn decrement_item(bag: &mut ItemBag, item: &ItemRef, count: i8) {
    // Debug.assertTrue(item.quantity >= count);
    assert_true(item.borrow().quantity >= count);
    // item.removeQuantity(count);
    item::remove_quantity(&mut item.borrow_mut(), count);
    // if (item.quantity < 1) removeItem(item);
    if item.borrow().quantity < 1 {
        remove_item(bag, item);
    }
    // revalidateQuickSlot(); mergeStacks();
    revalidate_quick_slot(bag);
    merge_stacks(bag);
}

/// `private void removeItem(Item item)` (`g.a:(Lad;)V => iinc`) — identity search
/// then compact.
fn remove_item(bag: &mut ItemBag, item: &ItemRef) {
    // for (int i = 0; i < this.slots.length; i++) if (this.slots[i] == item) { ... return; }
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        let is_it = match &bag.slots[i as usize] {
            Some(s) => Rc::ptr_eq(s, item),
            None => false,
        };
        if is_it {
            bag.slots[i as usize] = None;
            compact(bag);
            return;
        }
        i = i.wrapping_add(1);
    }
}

/// `public final void removeQuestItems()` (`g.c:()V => iinc`).
pub fn remove_quest_items(bag: &mut ItemBag) {
    // for (i) if (slots[i] != null && slots[i].isQuestItem()) { slots[i] = null; compact(); }
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        let is_quest = match &bag.slots[i as usize] {
            Some(s) => item::is_quest_item(&s.borrow()),
            None => false,
        };
        if is_quest {
            bag.slots[i as usize] = None;
            compact(bag);
        }
        i = i.wrapping_add(1);
    }
}

/// `public final void compact()` (`g.d:()V => isub,iadd,iinc,iinc`) — shift items
/// down into the first empty slot.
pub fn compact(bag: &mut ItemBag) {
    // for (int i = 0; i < this.slots.length; i++) {
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        // if (this.slots[i] == null) {
        if bag.slots[i as usize].is_none() {
            // int j = i; while (j < this.slots.length - 1) { slots[j] = slots[j + 1]; j++; }
            let mut j: i32 = i;
            while j < (bag.slots.len() as i32).wrapping_sub(1) {
                bag.slots[j as usize] = bag.slots[j.wrapping_add(1) as usize].clone();
                j = j.wrapping_add(1);
            }
            // this.slots[j] = null; return;
            bag.slots[j as usize] = None;
            return;
        }
        i = i.wrapping_add(1);
    }
}

/// `public final Item findItem(byte type, byte subId)` (`g.a:(BB)Lad; => []`).
pub fn find_item(bag: &ItemBag, r#type: i8, sub_id: i8) -> Option<ItemRef> {
    // byte slot = findSlot(type, subId); if (slot == -1) return null; return this.slots[slot];
    let slot = find_slot(bag, r#type, sub_id);
    if slot == -1 {
        return None;
    }
    bag.slots[slot as usize].clone()
}

/// `public final Item[] findAllItems(byte type, byte subId)` (`g.a:(BB)[Lad;`).
pub fn find_all_items(bag: &ItemBag, r#type: i8, sub_id: i8) -> Vec<ItemRef> {
    // Vector matches = new Vector(2); for (i) if (slots[i] != null && type&&sub) addElement;
    let mut matches: Vec<ItemRef> = Vec::with_capacity(2);
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if let Some(s) = &bag.slots[i as usize] {
            let b = s.borrow();
            if b.r#type == r#type && b.sub_id == sub_id {
                drop(b);
                matches.push(s.clone());
            }
        }
        i = i.wrapping_add(1);
    }
    // Item[] out = new Item[matches.size()]; for ... out[i] = matches.elementAt(i);
    matches
}

/// `public final int totalQuantity(byte type, byte subId)` (`g.a:(BB)I => iadd,iinc`).
pub fn total_quantity(bag: &ItemBag, r#type: i8, sub_id: i8) -> i32 {
    // int total = 0; for (i) if (slots[i] != null && type&&sub) total += slots[i].quantity;
    let mut total: i32 = 0;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if let Some(s) = &bag.slots[i as usize] {
            let b = s.borrow();
            if b.r#type == r#type && b.sub_id == sub_id {
                total = total.wrapping_add(b.quantity as i32);
            }
        }
        i = i.wrapping_add(1);
    }
    total
}

/// `public final byte slotOf(Item item)` (`g.a:(Lad;)B => iinc`) — identity search.
pub fn slot_of(bag: &ItemBag, item: &ItemRef) -> i8 {
    // for (i) if (this.slots[i] == item) return (byte) i; return (byte) -1;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        let is_it = match &bag.slots[i as usize] {
            Some(s) => Rc::ptr_eq(s, item),
            None => false,
        };
        if is_it {
            return i as i8;
        }
        i = i.wrapping_add(1);
    }
    -1
}

/// `public final byte findSlot(byte type, byte subId)` (`g.a:(BB)B => iadd,i2b`).
pub fn find_slot(bag: &ItemBag, r#type: i8, sub_id: i8) -> i8 {
    // for (i) if (slots[i] != null && type&&sub) return (byte) i; return (byte) -1;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if let Some(s) = &bag.slots[i as usize] {
            let b = s.borrow();
            if b.r#type == r#type && b.sub_id == sub_id {
                return i as i8;
            }
        }
        i = i.wrapping_add(1);
    }
    -1
}

/// `public final boolean canAdd(byte type, byte subId, int count)` (`g.a:(BBI)Z`).
pub fn can_add(bag: &mut ItemBag, r#type: i8, sub_id: i8, count: i32) -> bool {
    // mergeStacks(); byte free = firstFreeSlot();
    merge_stacks(bag);
    let free = first_free_slot(bag);
    // if (count > 99) return false;
    if count > 99 {
        return false;
    }
    // if (free != -1) return true;
    if free != -1 {
        return true;
    }
    // if (!Item.STACKABLE[type]) return false;
    if !item::STACKABLE[r#type as usize] {
        return false;
    }
    // for (Item stack : findAllItems(type, subId)) if (stack.quantity + count <= 99) return true;
    for stack in find_all_items(bag, r#type, sub_id) {
        if (stack.borrow().quantity as i32).wrapping_add(count) <= 99 {
            return true;
        }
    }
    // return false;
    false
}

/// `private final byte firstFreeSlot()` (`g.b:()B => iadd,i2b`).
fn first_free_slot(bag: &ItemBag) -> i8 {
    // for (i) if (this.slots[i] == null) return (byte) i; return (byte) -1;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if bag.slots[i as usize].is_none() {
            return i as i8;
        }
        i = i.wrapping_add(1);
    }
    -1
}

/// `public final boolean hasAtLeast(byte type, byte subId, byte count)` (`g.a:(BBB)Z => iinc`).
pub fn has_at_least(bag: &ItemBag, r#type: i8, sub_id: i8, count: i8) -> bool {
    // for (i) if (slots[i] != null && type&&sub && slots[i].quantity >= count) return true;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        if let Some(s) = &bag.slots[i as usize] {
            let b = s.borrow();
            if b.r#type == r#type && b.sub_id == sub_id && b.quantity >= count {
                return true;
            }
        }
        i = i.wrapping_add(1);
    }
    false
}

/// `public final void mergeStacks()` (`g.e:()V`) — the preserved nested-`for`
/// (see the module header). Coalesces adjacent stacks of the same stackable item.
pub fn merge_stacks(bag: &mut ItemBag) {
    // for (int i = 0; i < this.slots.length - 1; i++) {
    let mut i: i32 = 0;
    while i < (bag.slots.len() as i32).wrapping_sub(1) {
        // if (slots[i] != null && Item.STACKABLE[slots[i].type] && slots[i].quantity < 99) {
        let outer_ok = match &bag.slots[i as usize] {
            Some(s) => {
                let b = s.borrow();
                item::STACKABLE[b.r#type as usize] && (b.quantity as i32) < 99
            }
            None => false,
        };
        if outer_ok {
            // for (int j = i + 1; j < this.slots.length; j++) {
            let mut j: i32 = i.wrapping_add(1);
            while j < bag.slots.len() as i32 {
                // if (slots[j] != null && slots[j].type == slots[i].type && slots[j].subId == slots[i].subId) {
                let matches = {
                    let si = bag.slots[i as usize].as_ref().unwrap().borrow();
                    match &bag.slots[j as usize] {
                        Some(sj) => {
                            let bj = sj.borrow();
                            bj.r#type == si.r#type && bj.sub_id == si.sub_id
                        }
                        None => false,
                    }
                };
                if matches {
                    // byte have = slots[i].quantity; byte other = slots[j].quantity;
                    let have = bag.slots[i as usize].as_ref().unwrap().borrow().quantity;
                    let other = bag.slots[j as usize].as_ref().unwrap().borrow().quantity;
                    // if (have + other <= 99) {
                    if (have as i32).wrapping_add(other as i32) <= 99 {
                        // slots[i].addQuantity(other); slots[j] = null;
                        item::add_quantity(
                            &mut bag.slots[i as usize].as_ref().unwrap().borrow_mut(),
                            other,
                        );
                        bag.slots[j as usize] = None;
                        // if (this.quickSlot == j) this.quickSlot = (byte) i;
                        if bag.quick_slot as i32 == j {
                            bag.quick_slot = i as i8;
                        }
                    } else {
                        // byte moved = (byte) (99 - have);
                        let moved = (99i32).wrapping_sub(have as i32) as i8;
                        // slots[i].addQuantity(moved); slots[j].removeQuantity(moved);
                        item::add_quantity(
                            &mut bag.slots[i as usize].as_ref().unwrap().borrow_mut(),
                            moved,
                        );
                        item::remove_quantity(
                            &mut bag.slots[j as usize].as_ref().unwrap().borrow_mut(),
                            moved,
                        );
                    }
                }
                j = j.wrapping_add(1);
            }
        }
        i = i.wrapping_add(1);
    }
}

/// `public final byte[] serialize()` (`g.c:()[B => iinc`). Big-endian gold via the
/// JDK `DataOutputStream.writeInt` (reproduced inline), then a `[flag][10-byte item]`
/// stream. The `catch (IOException)` returning null is unreachable for the in-memory
/// `ByteArrayOutputStream`, so this always yields `Some`.
pub fn serialize(bag: &ItemBag) -> Option<Vec<i8>> {
    // out.writeInt(this.gold);
    let mut out: Vec<i8> = Vec::new();
    write_int_be(&mut out, bag.gold);
    // for (i) { if (slots[i]==null) out.writeByte(0); else { out.writeByte(1); out.write(slots[i].serialize()); } }
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        match &bag.slots[i as usize] {
            None => out.push(0),
            Some(s) => {
                out.push(1);
                out.extend_from_slice(&item::serialize(&s.borrow()));
            }
        }
        i = i.wrapping_add(1);
    }
    // return byteStream.toByteArray();
    Some(out)
}

/// `public final void deserialize(byte[] data)` (`g.a:([B)V => iinc,iinc`). Restores
/// gold and the compacted slot contents (each via [`item::deserialize`]), then
/// `resetQuickSelection()`. The `catch (IOException)` is unreachable here.
pub fn deserialize(bag: &mut ItemBag, g: &mut Game, data: &[i8]) {
    let mut pos: usize = 0;
    // this.gold = in.readInt();
    bag.gold = read_int_be(data, pos);
    pos += 4;
    // int w = 0; for (int i = 0; i < this.slots.length; i++) {
    let mut w: i32 = 0;
    let mut i: i32 = 0;
    while i < bag.slots.len() as i32 {
        // if (in.readByte() != 0) {
        let flag = data[pos];
        pos += 1;
        if flag != 0 {
            // byte[] itemBytes = new byte[10]; in.read(itemBytes);
            let mut item_bytes = vec![0i8; 10];
            item_bytes.copy_from_slice(&data[pos..pos + 10]);
            pos += 10;
            // this.slots[w++] = Item.deserialize(itemBytes);
            let it = item::deserialize(g, &item_bytes);
            bag.slots[w as usize] = Some(Rc::new(RefCell::new(it)));
            w = w.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    // resetQuickSelection();
    reset_quick_selection(bag);
}

/// `DataOutputStream.writeInt` — big-endian 4-byte write (JDK semantics, inlined).
fn write_int_be(out: &mut Vec<i8>, v: i32) {
    out.push((ishr(v, 24) & 255) as i8);
    out.push((ishr(v, 16) & 255) as i8);
    out.push((ishr(v, 8) & 255) as i8);
    out.push((v & 255) as i8);
}

/// `DataInputStream.readInt` — big-endian signed 32-bit (JDK semantics, inlined).
fn read_int_be(b: &[i8], i: usize) -> i32 {
    (((b[i] as i32) & 255) << 24)
        | (((b[i + 1] as i32) & 255) << 16)
        | (((b[i + 2] as i32) & 255) << 8)
        | ((b[i + 3] as i32) & 255)
}
