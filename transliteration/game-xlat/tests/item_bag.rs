//! `ItemBag` (`g`) unit tests — the carried-inventory store's stack/merge/add/remove
//! ops, including the preserved `mergeStacks` behavior (the nested-`for` restored
//! from a JADX infinite-loop artifact, capping stacks at 99). Each stateful check
//! carries a negative control so a passing assertion cannot read as vacuous.

use heroes_lore_wind_of_soltia_game_xlat::item::{self, Item};
use heroes_lore_wind_of_soltia_game_xlat::item_bag::{self, ItemBag, ItemRef};
use std::cell::RefCell;
use std::rc::Rc;

/// A base (stackable, type-7) item with a chosen `subId` and `quantity`.
fn stackable(sub_id: i8, quantity: i8) -> ItemRef {
    let mut it: Item = item::new_item(7, sub_id); // type 7 → base Item, STACKABLE[7] = true
    it.quantity = quantity;
    Rc::new(RefCell::new(it))
}

/// A non-stackable weapon (type 0, STACKABLE[0] = false), quantity 1.
fn weapon(sub_id: i8) -> ItemRef {
    Rc::new(RefCell::new(item::new_weapon(0, sub_id)))
}

fn qty(bag: &ItemBag, slot: usize) -> Option<i8> {
    bag.slots[slot].as_ref().map(|it| it.borrow().quantity)
}

fn occupied(bag: &ItemBag) -> usize {
    bag.slots.iter().filter(|s| s.is_some()).count()
}

#[test]
fn merge_stacks_coalesces_same_item_and_caps_at_99() {
    // Two adjacent stacks of the SAME type/subId, 30 + 40 = 70 (<= 99) → fully
    // merged into slot 0; slot 1 nulled.
    let mut bag = item_bag::new(5);
    bag.slots[0] = Some(stackable(0, 30));
    bag.slots[1] = Some(stackable(0, 40));
    item_bag::merge_stacks(&mut bag);
    assert_eq!(qty(&bag, 0), Some(70), "full merge should sum into slot 0");
    assert_eq!(qty(&bag, 1), None, "merged-away stack should be nulled");
    assert_eq!(occupied(&bag), 1, "one stack remains after a full merge");

    // NEGATIVE CONTROL: identical quantities but DIFFERENT subId must NOT merge —
    // proving the coalesce is gated on subId (not a vacuous pass).
    let mut bag2 = item_bag::new(5);
    bag2.slots[0] = Some(stackable(0, 30));
    bag2.slots[1] = Some(stackable(1, 40)); // different subId
    item_bag::merge_stacks(&mut bag2);
    assert_eq!(
        qty(&bag2, 0),
        Some(30),
        "different subId must be left untouched"
    );
    assert_eq!(
        qty(&bag2, 1),
        Some(40),
        "different subId must be left untouched"
    );
    assert_eq!(occupied(&bag2), 2, "different-subId stacks stay separate");
}

#[test]
fn merge_stacks_partial_move_preserves_99_cap_and_conserves_total() {
    // 60 + 60 = 120 > 99 → preserved partial-move: moved = 99 - 60 = 39; slot 0
    // becomes 99, slot 1 becomes 21 (both remain). This is the exact behavior the
    // de-obf pass restored (JADX had rendered it as an infinite loop).
    let mut bag = item_bag::new(5);
    bag.slots[0] = Some(stackable(0, 60));
    bag.slots[1] = Some(stackable(0, 60));
    item_bag::merge_stacks(&mut bag);
    assert_eq!(qty(&bag, 0), Some(99), "capped stack should be exactly 99");
    assert_eq!(
        qty(&bag, 1),
        Some(21),
        "overflow remainder should be 60 - 39 = 21"
    );
    // Conservation: nothing created or destroyed (60 + 60 == 99 + 21).
    let total = qty(&bag, 0).unwrap() as i32 + qty(&bag, 1).unwrap() as i32;
    assert_eq!(
        total, 120,
        "quantity must be conserved across the partial move"
    );

    // NEGATIVE CONTROL: 120 != 99 (the naive "just add" answer) — the cap really bit.
    assert_ne!(
        qty(&bag, 0),
        Some(120i32 as i8),
        "control: slot 0 must be capped at 99, not the uncapped 120"
    );
}

#[test]
fn merge_stacks_nested_for_collapses_three_stacks() {
    // Three same-item stacks 10, 20, 30 across non-adjacent slots — the nested for
    // must fold all into slot 0 = 60. Exercises the real nested-`for` control flow.
    let mut bag = item_bag::new(6);
    bag.slots[0] = Some(stackable(2, 10));
    bag.slots[2] = Some(stackable(2, 20));
    bag.slots[4] = Some(stackable(2, 30));
    item_bag::merge_stacks(&mut bag);
    assert_eq!(qty(&bag, 0), Some(60), "all three stacks fold into slot 0");
    assert_eq!(occupied(&bag), 1, "only one stack remains");

    // NEGATIVE CONTROL: had the inner loop not run (broken control flow), slot 0
    // would still read 10. Assert it does not.
    assert_ne!(
        qty(&bag, 0),
        Some(10),
        "control: the nested merge actually ran"
    );
}

#[test]
fn add_stacks_onto_existing_and_respects_capacity() {
    let mut bag = item_bag::new(3);
    // Add 10 of a stackable → new slot; then 5 more → stacks onto it (15 total).
    assert!(item_bag::add(&mut bag, stackable(0, 1), 10));
    assert!(item_bag::add(&mut bag, stackable(0, 1), 5));
    assert_eq!(item_bag::total_quantity(&bag, 7, 0), 15, "stacked to 15");
    assert_eq!(occupied(&bag), 1, "stacking uses one slot");

    // Non-stackable weapons each take a slot.
    assert!(item_bag::add(&mut bag, weapon(0), 1));
    assert!(item_bag::add(&mut bag, weapon(1), 1));
    assert_eq!(occupied(&bag), 3, "two weapons + one stack fill the bag");

    // NEGATIVE CONTROL: the bag is now full (3/3) and the stack is not full, but a
    // DIFFERENT stackable subId cannot stack and has no free slot → add fails.
    assert!(
        !item_bag::add(&mut bag, stackable(5, 1), 1),
        "control: add into a full bag with no matching stack must fail"
    );
    assert_eq!(occupied(&bag), 3, "the failed add changed nothing");
}

#[test]
fn remove_items_and_decrement_compact_the_bag() {
    let mut bag = item_bag::new(4);
    assert!(item_bag::add(&mut bag, stackable(0, 1), 20));
    assert!(item_bag::add(&mut bag, weapon(0), 1));
    // Bag: slot0 = stack(7,0) x20, slot1 = weapon.
    assert_eq!(occupied(&bag), 2);

    // Remove 20 of the stack → the stack empties and is removed, weapon compacts down.
    item_bag::remove_items(&mut bag, 7, 0, 20);
    assert_eq!(occupied(&bag), 1, "emptied stack removed");
    assert_eq!(
        bag.slots[0].as_ref().unwrap().borrow().r#type,
        0,
        "the weapon compacted into slot 0"
    );

    // NEGATIVE CONTROL: removing 5 of a type not present must leave the bag intact
    // (guard loop exits; count>0 would trip the assert, so use a present type at 0).
    item_bag::remove_items(&mut bag, 0, 0, 0);
    assert_eq!(occupied(&bag), 1, "control: a no-op remove changed nothing");
}

#[test]
fn serialize_produces_the_exact_save_bytes() {
    // gold + one occupied base-item slot + one empty slot.
    let mut bag = item_bag::new(2);
    bag.gold = 0x0102_0304;
    let mut it = item::new_item(7, 3);
    it.quantity = 5;
    bag.slots[0] = Some(Rc::new(RefCell::new(it)));
    // slot 1 stays empty.

    let bytes = item_bag::serialize(&bag).expect("serialize never returns null here");
    // [gold BE][flag=1][10-byte item: type,sub,qty,0*7][flag=0]
    let expected: Vec<i8> = vec![
        0x01, 0x02, 0x03, 0x04, // gold (big-endian)
        1,    // slot 0 present
        7, 3, 5, 0, 0, 0, 0, 0, 0, 0, // Item.serialize(): type, subId, quantity, pad
        0, // slot 1 absent
    ];
    assert_eq!(bytes, expected, "byte-exact ItemBag save form");

    // NEGATIVE CONTROL: a one-byte change to gold must change the serialization.
    let mut bag2 = item_bag::new(2);
    bag2.gold = 0x0102_0305; // low byte differs by 1
    let mut it2 = item::new_item(7, 3);
    it2.quantity = 5;
    bag2.slots[0] = Some(Rc::new(RefCell::new(it2)));
    let bytes2 = item_bag::serialize(&bag2).unwrap();
    assert_ne!(bytes, bytes2, "control: gold delta must alter the bytes");

    println!(
        "item_bag: serialize produced {} bytes; merge/add/remove ops verified",
        bytes.len()
    );
}
