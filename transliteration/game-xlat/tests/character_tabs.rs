//! STATE oracle for the five CharacterMenu tabs ported this lane: `ItemsTab` (`ay`),
//! `EquipTab` (`bz`), `GuardianTab` (`bm`), `SkillTab` (`s`) and `SystemTab` (`d`).
//!
//! * `CharacterMenu.moveCursor` now pushes each tab's ported child in turn — the sweep
//!   walks all six tabs (Status → Items → Equip → Guardian → Skill → System → wrap to
//!   Status) and asserts the `MenuChild` discriminant + `itemCount` each carries.
//! * One action per tab, each with a negative control:
//!   - `ItemsTab` FIRE on a usable bag item pushes the use popup; FIRE on an empty slot
//!     pushes nothing.
//!   - `EquipTab` FIRE on a slot whose category has bag candidates opens an
//!     `ItemPickerList`; a no-category slot shows a message instead.
//!   - `GuardianTab` FIRE on an empty guardian slot is not consumed; a nav key is.
//!   - `SystemTab` FIRE on the Options row pushes the (ported) `OptionsMenu`; the
//!     (DEFERRED) Help row pushes no child.
//!
//! These are STATE assertions, not pixel diffs (the tab art is DEFERRED — it crosses
//! into unported `AssetCache` icon/text banks + `Menu`/`BaseCanvas` widgets). The shared
//! `/sgui/gm` label table the tabs read is loaded from the real JAR by
//! `CharacterMenu.open`.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::entity::EntityId;
use heroes_lore_wind_of_soltia_game_xlat::item_bag::{self, ItemRef};
use heroes_lore_wind_of_soltia_game_xlat::menu::MenuChild;
use heroes_lore_wind_of_soltia_game_xlat::{
    byte_util, character_menu, equip_tab, guardian_tab, hero, item, items_tab, system_tab, Game,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Deterministic RNG seed (the other state tests use the same fixed seed).
const GAME_RNG_SEED: i64 = 305_419_896;
/// The warrior class (class ids run 6..8).
const CLASS_WARRIOR: i8 = 6;

/// FIRE / select — `keyCode 53` (KEY_NUM5).
const KEY_FIRE: i32 = 53;
/// DOWN — `keyCode 56` (KEY_NUM8).
const KEY_DOWN: i32 = 56;

/// A New-Game warrior placed as `GameState.hero`, with the JAR loaded (so
/// `CharacterMenu.open` resolves `/sgui/gm` and `initClass`'s equipment records load),
/// a sized flag array (so `SkillTab`'s constructor scan can read `GameState.isFlag`),
/// a concrete centred origin, and the character menu opened on the status tab. Returns
/// the hero id.
fn opened_character_menu() -> (Game, EntityId) {
    let mut g = Game::new();
    g.byte_util = byte_util::ByteUtilState::seeded(GAME_RNG_SEED);
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    g.game_state.class_id = CLASS_WARRIOR;
    // A fresh new-game hero (initClass equips the starting gear; the bag starts empty).
    let id = hero::new_hero(&mut g.entity_arena, &g.clock, 0, 0, 8, 8, CLASS_WARRIOR);
    g.game_state.hero = Some(id);
    hero::init_class(&mut g, id, CLASS_WARRIOR);
    // SkillTab's constructor reads GameState.isFlag(1 + skill*3 + n) for every class skill
    // (15 for the warrior → bits up to 45 → flags[5]); a fresh install has these clear, so
    // size the flag array and leave it zero (no skill learned → SkillTab stays empty).
    g.game_state.flags = vec![0i8; 16];
    // A concrete centred origin (panelX/panelY, and SystemTab's save box, are checkable).
    g.base_canvas.half_w = 88;
    g.base_canvas.half_h = 104;
    g.base_canvas.width = 176;
    character_menu::instance(&mut g);
    character_menu::open(&mut g);
    (g, id)
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

/// `CharacterMenu.moveCursor` pushes each tab's ported child in turn; the six-tab sweep
/// wraps back to the status tab.
#[test]
fn move_cursor_sweeps_all_six_ported_tabs() {
    let (mut g, _hero) = opened_character_menu();

    // instance() opened on tab 0 (StatusPage).
    assert_eq!(g.character_menu.base.cursor_index, 0, "opened on tab 0");
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::Status,
        "tab 0 → StatusPage"
    );

    // RIGHT (moveCursor 4) advances one tab at a time, pushing each ported child.
    character_menu::move_cursor(&mut g, 4);
    assert_eq!(g.character_menu.base.cursor_index, 1);
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::Items,
        "tab 1 → ItemsTab"
    );
    assert_eq!(g.items_tab.base.item_count, 30, "ItemsTab → 30-slot bag");

    character_menu::move_cursor(&mut g, 4);
    assert_eq!(g.character_menu.base.cursor_index, 2);
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::Equip,
        "tab 2 → EquipTab"
    );
    assert_eq!(
        g.equip_tab.base.item_count, 5,
        "EquipTab → five equip slots"
    );

    character_menu::move_cursor(&mut g, 4);
    assert_eq!(g.character_menu.base.cursor_index, 3);
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::Guardian,
        "tab 3 → GuardianTab"
    );
    assert_eq!(
        g.guardian_tab.base.item_count, 5,
        "GuardianTab → five guardian slots"
    );

    character_menu::move_cursor(&mut g, 4);
    assert_eq!(g.character_menu.base.cursor_index, 4);
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::Skill,
        "tab 4 → SkillTab"
    );
    assert_eq!(
        g.skill_tab.base.item_count, 0,
        "SkillTab → no learnable skills for a fresh hero (classSkillText DEFERRED)"
    );

    character_menu::move_cursor(&mut g, 4);
    assert_eq!(g.character_menu.base.cursor_index, 5);
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::System,
        "tab 5 → SystemTab"
    );
    assert_eq!(g.system_tab.base.item_count, 4, "SystemTab → four buttons");
    assert_eq!(
        g.system_tab.save_state, 0,
        "SystemTab save state starts idle"
    );

    // RIGHT off the last tab wraps back to tab 0, rebuilding the StatusPage child.
    character_menu::move_cursor(&mut g, 4);
    assert_eq!(
        g.character_menu.base.cursor_index, 0,
        "tab 5 → wrap to tab 0"
    );
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::Status,
        "the wrap rebuilt StatusPage"
    );
}

/// `ItemsTab` FIRE on a usable bag item offers the use popup; FIRE on an empty slot is
/// consumed but pushes nothing.
#[test]
fn items_tab_fire_offers_use_popup() {
    let (mut g, hero_id) = opened_character_menu();
    // A usable potion (type 7) in the bag.
    let potion = Rc::new(RefCell::new(item::new_item(7, 0)));
    let slot = add_to_bag(&mut g, hero_id, potion, 1);

    // Switch to the items tab and point the cursor at the potion.
    character_menu::move_cursor(&mut g, 4);
    assert_eq!(g.character_menu.base.child, MenuChild::Items);
    g.items_tab.base.cursor_index = slot;

    // FIRE on a usable item → showPopup(5, 2, …) → a Popup child.
    assert!(
        items_tab::handle_key(&mut g, 0, KEY_FIRE),
        "FIRE on a bag item is consumed"
    );
    assert_eq!(
        g.items_tab.base.child,
        MenuChild::Popup,
        "FIRE on a usable item pushed the use popup"
    );

    // NEGATIVE CONTROL: FIRE on an empty slot pushes nothing (item == null → return true).
    g.items_tab.base.child = MenuChild::None;
    g.items_tab.base.cursor_index = 29; // an empty slot (only slot `slot` holds an item)
    assert!(
        items_tab::handle_key(&mut g, 0, KEY_FIRE),
        "FIRE is still consumed on an empty slot"
    );
    assert_eq!(
        g.items_tab.base.child,
        MenuChild::None,
        "FIRE on an empty slot pushes no popup"
    );
}

/// `EquipTab` FIRE on a slot whose category has bag candidates opens an
/// `ItemPickerList`; a slot that resolves to no category shows a message instead.
#[test]
fn equip_tab_fire_opens_item_picker() {
    let (mut g, hero_id) = opened_character_menu();
    // A type-0 weapon in the bag (the warrior's slot-0 category is 0).
    let weapon = Rc::new(RefCell::new(item::new_weapon(0, 0)));
    add_to_bag(&mut g, hero_id, weapon, 1);

    // Switch to the equipment tab; cursor 0 (weapon slot) → category 0 for the warrior.
    character_menu::move_cursor(&mut g, 4);
    character_menu::move_cursor(&mut g, 4);
    assert_eq!(g.character_menu.base.child, MenuChild::Equip);
    g.equip_tab.base.cursor_index = 0;

    // FIRE with a bag candidate → new ItemPickerList → an ItemPicker child.
    assert!(
        equip_tab::handle_key(&mut g, 0, KEY_FIRE),
        "FIRE on a slot with candidates is consumed"
    );
    assert_eq!(
        g.equip_tab.base.child,
        MenuChild::ItemPicker,
        "FIRE opened the item picker over the bag candidates"
    );

    // NEGATIVE CONTROL: slot 1 for a warrior (classId 6) resolves to no category
    // (case 1 sets a category only for classId 8) → showMessage, NOT an ItemPickerList.
    g.equip_tab.base.child = MenuChild::None;
    g.equip_tab.base.cursor_index = 1;
    assert!(
        equip_tab::handle_key(&mut g, 0, KEY_FIRE),
        "FIRE is consumed"
    );
    assert_eq!(
        g.equip_tab.base.child,
        MenuChild::Popup,
        "a no-category slot shows a message popup, not the picker"
    );
    assert_ne!(
        g.equip_tab.base.child,
        MenuChild::ItemPicker,
        "no picker was opened for the no-category slot"
    );
}

/// `GuardianTab` FIRE on an empty guardian slot is not consumed (no guardian to act on);
/// a nav key IS consumed (so "not consumed" is not an unconditional constant).
#[test]
fn guardian_tab_fire_on_empty_slot_is_inert() {
    let (mut g, _hero) = opened_character_menu();
    character_menu::move_cursor(&mut g, 4);
    character_menu::move_cursor(&mut g, 4);
    character_menu::move_cursor(&mut g, 4);
    assert_eq!(g.character_menu.base.child, MenuChild::Guardian);

    // FIRE with guardians[cursorIndex] == null → returns false, pushes nothing.
    assert!(
        !guardian_tab::handle_key(&mut g, 0, KEY_FIRE),
        "FIRE on an empty guardian slot is not consumed"
    );
    assert_eq!(
        g.guardian_tab.base.child,
        MenuChild::None,
        "no popup pushed on an empty guardian slot"
    );

    // NEGATIVE CONTROL: a DOWN nav key IS consumed (moveCursorVerticalNoWrap).
    assert!(
        guardian_tab::handle_key(&mut g, 0, KEY_DOWN),
        "DOWN nav is consumed on the guardian tab"
    );
}

/// `SystemTab` FIRE on the Options row pushes the ported `OptionsMenu`; the DEFERRED
/// Help row pushes no child (so the Options push is not unconditional).
#[test]
fn system_tab_fire_opens_options_and_defers_help() {
    let (mut g, _hero) = opened_character_menu();
    // moveCursor(3) retreats tab 0 → wrap to tab 5 (System) directly.
    character_menu::move_cursor(&mut g, 3);
    assert_eq!(g.character_menu.base.cursor_index, 5);
    assert_eq!(g.character_menu.base.child, MenuChild::System);

    // Options row (cursor 2) → new OptionsMenu(this, true) → an Options child.
    g.system_tab.base.cursor_index = 2;
    assert!(
        system_tab::handle_key(&mut g, 0, KEY_FIRE),
        "FIRE on the Options row is consumed"
    );
    assert_eq!(
        g.system_tab.base.child,
        MenuChild::Options,
        "FIRE on the Options row pushed OptionsMenu"
    );
    assert!(
        g.options_menu.in_game,
        "OptionsMenu was constructed in-game (new OptionsMenu(this, true))"
    );

    // NEGATIVE CONTROL: the Help row (cursor 1) is DEFERRED (HelpMenu unported) → the
    // key is consumed but no child is pushed (so the Options push above is not a constant).
    system_tab::construct(&mut g); // a fresh SystemTab (child cleared)
    g.system_tab.base.cursor_index = 1;
    assert!(
        system_tab::handle_key(&mut g, 0, KEY_FIRE),
        "FIRE on the Help row is consumed"
    );
    assert_eq!(
        g.system_tab.base.child,
        MenuChild::None,
        "the DEFERRED Help row pushes no child"
    );
}
