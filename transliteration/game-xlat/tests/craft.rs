//! STATE oracle for the craft/refine menu cluster ported this lane: `RefineMenu`
//! (`ax`), `EnchantMenu` (`ap`), `CombineMenu` (`k`) and `CostConfirmDialog` (`bo`).
//!
//! * `RefineMenu.instance()`/`open()` build the refinery singleton over the real
//!   `/sgui/refi` label table, centre the panel at `halfW-77`/`halfH-85`, and show the
//!   enchant/combine choice popup; its `onPopupResult` pushes the chosen child.
//! * `EnchantMenu` over a hero bag: picking an identified, un-enchanted armor and a
//!   type-17 scroll and confirming (through a yes/no popup) STAMPS the scroll's element
//!   onto the armor's `attribute`, CHARGES 500 gold, and CONSUMES the scroll — the
//!   drive threads the real `handleKey`/`ItemPickerList`/`PopupMenu` key dispatch.
//! * `CombineMenu` + `CostConfirmDialog`: staging two ingredients, pressing the button
//!   row opens a `CostConfirmDialog` (cost 500, tag 20); confirming it (its `handleKey`)
//!   routes to a yes/no popup and then to `Item.craft` — a no-recipe combine CONSUMES
//!   the staged ingredients.
//!
//! These are STATE assertions, not pixel diffs: the craft-menu art is partial
//! (DEFERRED — `Menu.drawQuickSlotRow`/`drawGold`/`drawButton`, `Armor.attributeNames`
//! and `Hero.serializeItems`/`slotOf` cross into unported draw kit / Hero methods). The
//! `/sgui/refi` label table and `/itm/mixtbl` recipe blob are read from the real JAR.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::entity::EntityId;
use heroes_lore_wind_of_soltia_game_xlat::item;
use heroes_lore_wind_of_soltia_game_xlat::item_bag::{self, ItemRef};
use heroes_lore_wind_of_soltia_game_xlat::menu::{self, MenuChild, MenuNode};
use heroes_lore_wind_of_soltia_game_xlat::{
    byte_util, combine_menu, cost_confirm_dialog, enchant_menu, hero, item_picker_list, popup_menu,
    refine_menu, text_table, Game,
};
use std::cell::RefCell;
use std::rc::Rc;

/// FIRE / select — `keyCode 53` (KEY_NUM5).
const KEY_FIRE: i32 = 53;
/// The warrior class (class ids run 6..8).
const CLASS_WARRIOR: i8 = 6;
/// Deterministic RNG seed (the other state tests use the same fixed seed).
const GAME_RNG_SEED: i64 = 305_419_896;

/// A `Game` with the baseline JAR's resources loaded (so `RefineMenu.open` can read
/// `/sgui/refi.tdf` and `Item.craft` can read `/itm/mixtbl`).
fn game_with_resources() -> Game {
    let mut g = Game::new();
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    g
}

/// A New-Game warrior placed as `GameState.hero`, with a fixed RNG seed. Returns its id.
fn new_game_hero(g: &mut Game) -> EntityId {
    g.byte_util = byte_util::ByteUtilState::seeded(GAME_RNG_SEED);
    g.game_state.class_id = CLASS_WARRIOR;
    let id = hero::new_hero(&mut g.entity_arena, &g.clock, 0, 0, 8, 8, CLASS_WARRIOR);
    g.game_state.hero = Some(id);
    hero::init_class(g, id, CLASS_WARRIOR);
    id
}

/// Places `item` into the hero's bag with `count` units and returns its bag slot.
fn add_to_bag(g: &mut Game, hero_id: EntityId, item: ItemRef, count: i32) -> i8 {
    let bag = &mut g.entity_arena[hero_id]
        .as_hero_mut()
        .expect("Hero node")
        .bag;
    assert!(item_bag::add(bag, item.clone(), count), "bag add succeeded");
    item_bag::slot_of(bag, &item)
}

fn gold(g: &Game, hero_id: EntityId) -> i32 {
    g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .bag
        .gold
}

fn set_gold(g: &mut Game, hero_id: EntityId, value: i32) {
    g.entity_arena[hero_id]
        .as_hero_mut()
        .expect("Hero node")
        .bag
        .gold = value;
}

/// An identified, un-enchanted armor piece (type 3, `attribute == -1`), quantity 1.
fn identified_armor(sub_id: i8) -> ItemRef {
    let mut it = item::new_armor(3, sub_id);
    it.identified = true;
    it.attribute = -1;
    Rc::new(RefCell::new(it))
}

/// An enchant scroll (item type 17) whose element is `sub_id`.
fn enchant_scroll(sub_id: i8) -> ItemRef {
    Rc::new(RefCell::new(item::new_item(17, sub_id)))
}

/// `RefineMenu.instance()`/`open()` build the singleton + choice popup, and
/// `onPopupResult(8, 0)` pushes an `EnchantMenu`; picking an armor + a type-17 scroll
/// and confirming stamps the scroll element onto the armor, charges 500 gold, and
/// consumes the scroll.
#[test]
fn enchant_selection_and_confirm_mutates_armor_gold_and_scroll() {
    let mut g = game_with_resources();
    let hero_id = new_game_hero(&mut g);
    // A concrete centred origin so panelX/panelY are checkable.
    g.base_canvas.half_w = 88;
    g.base_canvas.half_h = 104;
    set_gold(&mut g, hero_id, 1000);

    let armor = identified_armor(2);
    let scroll = enchant_scroll(5); // element 5
    let armor_slot = add_to_bag(&mut g, hero_id, armor.clone(), 1);
    let scroll_slot = add_to_bag(&mut g, hero_id, scroll.clone(), 1);

    // --- RefineMenu.instance()/open(): the singleton + the enchant/combine popup. ---
    refine_menu::instance(&mut g);
    assert!(g.refine_menu.singleton, "instance() created the singleton");
    assert!(
        !g.refine_menu.base.parent,
        "the refinery is a root (super(null, …))"
    );
    assert_eq!(g.refine_menu.panel_x, 88 - 77, "panelX = halfW - 77");
    assert_eq!(g.refine_menu.panel_y, 104 - 85, "panelY = halfH - 85");

    refine_menu::open(&mut g);
    assert_eq!(
        g.refine_menu.base.child,
        MenuChild::Popup,
        "open() showed the enchant/combine choice popup"
    );
    let text = g
        .refine_menu
        .text
        .as_ref()
        .expect("open() loaded /sgui/refi");
    assert!(text.count > 0, "the /sgui/refi table has entries");

    // onPopupResult(tag 8, result 0) → the popup's "enchant" row pushes an EnchantMenu.
    menu::on_popup_result(&mut g, MenuNode::Refine, 8, 0);
    assert_eq!(
        g.refine_menu.base.child,
        MenuChild::Enchant,
        "choosing row 0 pushed an EnchantMenu"
    );
    assert_eq!(g.enchant_menu.base.item_count, 3, "super(parent, (byte) 3)");
    assert!(g.enchant_menu.armor.is_none(), "no armor picked yet");
    assert!(g.enchant_menu.material.is_none(), "no scroll picked yet");

    // --- Pick the armor (row 0): resolves bag.get(slot) + validates identified/un-enchanted. ---
    // (The armor picker is opened by the DEFERRED Hero.serializeItems path, so the pick
    //  callback is driven directly — the resolution it exercises is fully ported.)
    g.enchant_menu.base.child = MenuChild::ItemPicker;
    menu::on_popup_result(&mut g, MenuNode::Enchant, 0, armor_slot);
    assert!(
        g.enchant_menu
            .armor
            .as_ref()
            .is_some_and(|a| Rc::ptr_eq(a, &armor)),
        "armor pick stored the identified, un-enchanted armor"
    );

    // --- Pick the scroll (row 1) through the REAL handleKey → ItemPickerList dispatch. ---
    g.enchant_menu.base.cursor_index = 1;
    assert!(
        enchant_menu::handle_key(&mut g, 0, KEY_FIRE),
        "row-1 FIRE opens the scroll picker"
    );
    assert_eq!(
        g.enchant_menu.base.child,
        MenuChild::ItemPicker,
        "the type-17 scroll picker was pushed"
    );
    assert_eq!(
        g.item_picker_list.slots,
        vec![scroll_slot],
        "the picker lists exactly the one scroll slot"
    );
    // The picker's OK reports parent.onPopupResult(resultTag = 1, scroll_slot).
    g.item_picker_list.base.cursor_index = 0;
    assert!(
        item_picker_list::handle_key(&mut g, 0, KEY_FIRE),
        "picker FIRE reports the chosen scroll slot"
    );
    assert!(
        g.enchant_menu
            .material
            .as_ref()
            .is_some_and(|m| Rc::ptr_eq(m, &scroll)),
        "scroll pick stored the type-17 material"
    );

    // --- Confirm (row 2): opens the yes/no popup, then a yes applies the enchant. ---
    let gold_before = gold(&g, hero_id);
    assert_eq!(
        gold_before, 1000,
        "gold untouched until the enchant confirms"
    );
    assert_eq!(
        armor.borrow().attribute,
        -1,
        "armor un-enchanted before confirm"
    );

    g.enchant_menu.base.cursor_index = 2;
    assert!(
        enchant_menu::handle_key(&mut g, 0, KEY_FIRE),
        "row-2 FIRE opens the confirm popup"
    );
    assert_eq!(
        g.enchant_menu.base.child,
        MenuChild::Popup,
        "the enchant confirm popup was pushed"
    );
    // The popup's OK reports parent.onPopupResult(2, 0) → the enchant is applied.
    assert!(
        popup_menu::handle_key(&mut g, 0, KEY_FIRE),
        "popup FIRE confirms the enchant"
    );

    // STATE assertions: armor stamped, 500 gold charged, scroll consumed.
    assert_eq!(
        armor.borrow().attribute,
        5,
        "armor.attribute stamped with the scroll's element (subId 5)"
    );
    assert_eq!(gold(&g, hero_id), 500, "gold charged 500 (1000 → 500)");
    assert_eq!(
        item_bag::total_quantity(
            &g.entity_arena[hero_id].as_hero().expect("Hero node").bag,
            17,
            5
        ),
        0,
        "the enchant scroll was consumed"
    );
    assert!(
        g.enchant_menu.armor.is_none() && g.enchant_menu.material.is_none(),
        "the enchant cleared the staged armor/material"
    );

    // NEGATIVE CONTROL: a fresh EnchantMenu confirm with NO armor/material picked must
    // NOT charge gold — the low-input guard shows a message (popup type 1) instead of the
    // yes/no confirm (type 2), so nothing reaches the enchant apply.
    let mut g2 = game_with_resources();
    let hero2 = new_game_hero(&mut g2);
    set_gold(&mut g2, hero2, 1000);
    add_to_bag(&mut g2, hero2, identified_armor(2), 1);
    g2.refine_menu.text = Some(text_table::construct(&mut g2, "/sgui/refi"));
    enchant_menu::construct(&mut g2);
    g2.enchant_menu.base.cursor_index = 2; // armor + material both null
    assert!(
        enchant_menu::handle_key(&mut g2, 0, KEY_FIRE),
        "FIRE handled"
    );
    assert_eq!(
        g2.popup_menu.popup_type, 1,
        "no armor staged → a message (type 1), not the yes/no confirm (type 2)"
    );
    assert_eq!(
        gold(&g2, hero2),
        1000,
        "no gold charged without a full selection"
    );
}

/// `RefineMenu.onPopupResult(8, 1)` pushes a `CombineMenu`; staging two ingredients and
/// pressing the button row opens a `CostConfirmDialog` (cost 500, tag 20). Confirming it
/// routes through a yes/no popup to `Item.craft`; a no-recipe combine consumes the
/// staged ingredients.
#[test]
fn combine_cost_confirm_consumes_ingredients() {
    let mut g = game_with_resources();
    let hero_id = new_game_hero(&mut g);
    g.base_canvas.half_w = 88;
    g.base_canvas.half_h = 104;
    set_gold(&mut g, hero_id, 1000);

    // A quick-usable, stackable item (type 9) with two units — no `{(9,5),(9,5)}` recipe
    // exists in /itm/mixtbl, so the craft returns null and the ingredients are consumed.
    let stack = Rc::new(RefCell::new(item::new_item(9, 5)));
    let stack_slot = add_to_bag(&mut g, hero_id, stack.clone(), 2);
    assert_eq!(
        item_bag::total_quantity(
            &g.entity_arena[hero_id].as_hero().expect("Hero node").bag,
            9,
            5
        ),
        2,
        "two type-9 units staged in the bag"
    );

    // --- RefineMenu → CombineMenu (choice popup row 1). ---
    refine_menu::instance(&mut g);
    refine_menu::open(&mut g);
    menu::on_popup_result(&mut g, MenuNode::Refine, 8, 1);
    assert_eq!(
        g.refine_menu.base.child,
        MenuChild::Combine,
        "choosing row 1 pushed a CombineMenu"
    );
    assert_eq!(g.combine_menu.base.item_count, 4, "super(parent, (byte) 4)");
    assert!(
        g.combine_menu.craft_slots.iter().all(|s| s.is_none()),
        "craft slots start empty"
    );

    // --- Stage two ingredients (slots 0 and 1) via the ported pick resolution. ---
    for tag in 0..2i8 {
        g.combine_menu.base.child = MenuChild::ItemPicker;
        menu::on_popup_result(&mut g, MenuNode::Combine, tag, stack_slot);
        assert!(
            g.combine_menu.craft_slots[tag as usize]
                .as_ref()
                .is_some_and(|s| Rc::ptr_eq(s, &stack)),
            "craft slot {tag} staged the picked ingredient"
        );
    }

    // --- Button row (cursor 3): two filled + 500 gold → a CostConfirmDialog. ---
    g.combine_menu.base.cursor_index = 3;
    assert!(
        combine_menu::handle_key(&mut g, 0, KEY_FIRE),
        "button-row FIRE opens the cost confirm"
    );
    assert_eq!(
        g.combine_menu.base.child,
        MenuChild::CostConfirm,
        "a CostConfirmDialog was pushed"
    );
    assert_eq!(g.cost_confirm_dialog.cost, 500, "the combine fee is 500");
    assert_eq!(
        g.cost_confirm_dialog.result_tag, 20,
        "reports back tagged 20"
    );
    assert_eq!(
        g.cost_confirm_dialog.item_lines.len(),
        3,
        "the dialog carries three item-name lines"
    );

    // --- CostConfirmDialog OK → CombineMenu shows the final yes/no popup. ---
    assert!(
        cost_confirm_dialog::handle_key(&mut g, 0, KEY_FIRE),
        "cost-confirm FIRE reports OK to the parent"
    );
    assert_eq!(
        g.combine_menu.base.child,
        MenuChild::Popup,
        "the combine's final yes/no popup was pushed"
    );

    // --- Final popup yes → Item.craft (no recipe → null) consumes the ingredients. ---
    assert!(
        popup_menu::handle_key(&mut g, 0, KEY_FIRE),
        "popup FIRE confirms the craft"
    );
    assert_eq!(
        item_bag::total_quantity(
            &g.entity_arena[hero_id].as_hero().expect("Hero node").bag,
            9,
            5
        ),
        0,
        "the no-recipe combine consumed both staged ingredients"
    );
    assert!(
        g.combine_menu.craft_slots.iter().all(|s| s.is_none()),
        "the craft cleared the staged slots"
    );

    // NEGATIVE CONTROL: with only ONE ingredient staged the button row shows a message
    // (not a CostConfirmDialog), so the cost-confirm above cannot read as unconditional.
    let mut g2 = game_with_resources();
    let hero2 = new_game_hero(&mut g2);
    set_gold(&mut g2, hero2, 1000);
    g2.refine_menu.text = Some(text_table::construct(&mut g2, "/sgui/refi"));
    let one = Rc::new(RefCell::new(item::new_item(9, 5)));
    let one_slot = add_to_bag(&mut g2, hero2, one.clone(), 1);
    combine_menu::construct(&mut g2);
    g2.combine_menu.base.child = MenuChild::ItemPicker;
    menu::on_popup_result(&mut g2, MenuNode::Combine, 0, one_slot);
    g2.combine_menu.base.cursor_index = 3;
    assert!(
        combine_menu::handle_key(&mut g2, 0, KEY_FIRE),
        "FIRE handled"
    );
    assert_ne!(
        g2.combine_menu.base.child,
        MenuChild::CostConfirm,
        "one ingredient (< 2) does NOT open the cost confirm"
    );
}
