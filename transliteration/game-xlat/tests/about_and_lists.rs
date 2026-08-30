//! STATE oracle for the three leaf menu classes ported this lane:
//! `AboutScreen` (`bl`), `ItemPickerList` (`m`) and `SellList` (`bb`).
//!
//! * From the settled main menu, three DOWNs land the cursor on About (index 4,
//!   past the disabled Load row) and FIRE runs `MainMenu` case 4 → pushes
//!   `AboutScreen`, witnessed by the child discriminant + the menu-stack depth + the
//!   pushed `Menu` base fields + the three small-font control-glyph un-hides; a frame
//!   renders its partial paint (must not crash); Back runs `parent.close()` and
//!   re-hides the control glyphs.
//! * `ItemPickerList`'s constructor sets `itemCount = slots.length`, and its
//!   vertical no-wrap navigation clamps at the ends.
//! * `SellList` constructed over a hero-bag's occupied slots has one row per
//!   occupied slot (`itemCount == occupiedSlots.length`), with a negative control on
//!   a different occupancy.
//!
//! These are STATE assertions, not pixel diffs: the list/about art is partial
//! (DEFERRED — it crosses into unported `FontManager` wrapped-text / `AssetCache`
//! art). Keys are driven purely by keyCode (53 FIRE, 56 DOWN, -8 BACK), each drained
//! by the next frame's serialized `game_screen::key_pressed`.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::item;
use heroes_lore_wind_of_soltia_game_xlat::item_bag::{self, ItemBag, ItemRef};
use heroes_lore_wind_of_soltia_game_xlat::menu::MenuChild;
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, font_manager, game_loop, game_midlet, item_picker_list, sell_list,
    title_screen, Game,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Matches `front_menus.rs`/`menu_chain.rs`: the route's game-RNG seed keeps the
/// pre-menu drive deterministic (title birds consume `ByteUtil.rng`).
const GAME_RNG_SEED: i64 = 305419896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
/// SOFT1 (any key) leaves the state-1 title.
const KEY_SOFT1: i32 = -6;
/// FIRE / select — `keyCode 53` (KEY_NUM5).
const KEY_FIRE: i32 = 53;
/// DOWN — `keyCode 56` (KEY_NUM8).
const KEY_DOWN: i32 = 56;
/// UP — `keyCode 50` (KEY_NUM2).
const KEY_UP: i32 = 50;
/// BACK / clear — `keyCode -8` (its `getGameAction` is 0).
const KEY_BACK: i32 = -8;

fn key_press(g: &mut Game, code: i32) {
    g.canvas.as_mut().expect("canvas").key_pressed(code);
}

/// Presses one key and runs the frame that drains it.
fn press_and_step(g: &mut Game, code: i32) {
    key_press(g, code);
    game_loop::run_one_frame(g);
}

/// The number of menus in the pushed child stack rooted at `MainMenu` (1 = just the
/// main menu, 2 = one child open, …) — the flat model's depth witness.
fn menu_depth(g: &Game) -> u32 {
    let mut depth = 1; // MainMenu is always the root.
    let mut child = g.main_menu.base.child;
    loop {
        let next = match child {
            MenuChild::None => break,
            MenuChild::ClassSelect => g.class_select_menu.base.child,
            MenuChild::ClassConfirm => g.class_confirm_menu.base.child,
            MenuChild::StartTrait => g.start_trait_menu.base.child,
            MenuChild::Popup => g.popup_menu.base.child,
            MenuChild::Confirm => g.confirm_dialog.base.child,
            MenuChild::Continue => g.continue_menu.base.child,
            MenuChild::Options => g.options_menu.base.child,
            MenuChild::About => g.about_screen.base.child,
            MenuChild::ItemPicker => g.item_picker_list.base.child,
            MenuChild::SellList => g.sell_list.picker.base.child,
            MenuChild::ShopItemList => g.shop_item_list.base.child,
            MenuChild::BuySell => g.buy_sell_dialog.base.child,
            MenuChild::Status => g.status_page.base.child,
            MenuChild::Items => g.items_tab.base.child,
            MenuChild::Equip => g.equip_tab.base.child,
            MenuChild::Guardian => g.guardian_tab.base.child,
            MenuChild::Skill => g.skill_tab.base.child,
            MenuChild::System => g.system_tab.base.child,
            MenuChild::StatAlloc => g.stat_alloc_menu.base.child,
            MenuChild::Enchant => g.enchant_menu.base.child,
            MenuChild::Combine => g.combine_menu.base.child,
            MenuChild::CostConfirm => g.cost_confirm_dialog.base.child,
        };
        depth += 1;
        child = next;
    }
    depth
}

/// Drives boot → title → any-key → settled main menu with NEW GAME selected
/// (cursorIndex 0) — the same path `front_menus::drive_to_main_menu` takes.
fn drive_to_main_menu() -> Game {
    let mut g = Game::new();
    g.byte_util = byte_util::ByteUtilState::seeded(GAME_RNG_SEED);
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    game_midlet::construct(&mut g);
    game_midlet::start_app(&mut g);
    title_screen::construct(&mut g);
    asset_cache::load_logo(&mut g);
    asset_cache::load_title_screen(&mut g);
    font_manager::init_fonts(&mut g);
    font_manager::load_title_labels(&mut g);
    title_screen::start_logo(&mut g);
    {
        let Game {
            display, canvas, ..
        } = &mut g;
        display.set_current(None, canvas.as_mut().expect("TitleScreen canvas"));
    }
    let mut guard = 0u32;
    loop {
        game_loop::run_one_frame(&mut g);
        guard += 1;
        if g.title_screen.state == 1 {
            break;
        }
        assert!(guard < 10_000, "state-10 never transitioned to the title");
    }
    for _ in 0..TITLE_FRAMES_BEFORE_KEY {
        game_loop::run_one_frame(&mut g);
    }
    key_press(&mut g, KEY_SOFT1);
    for _ in 0..MENU_SETTLE {
        game_loop::run_one_frame(&mut g);
    }
    g
}

/// A base (type-7) bag item with a chosen `subId` — mirrors the `item_bag.rs`
/// unit-test helper (occupies one bag slot).
fn bag_item(sub_id: i8) -> ItemRef {
    Rc::new(RefCell::new(item::new_item(7, sub_id)))
}

/// Reads a small font's `hideControls` flag (set by `AboutScreen`).
fn hide_controls_black(g: &Game) -> bool {
    g.font_manager
        .small_black
        .as_ref()
        .expect("smallBlack")
        .hide_controls
}

/// Three DOWNs on the settled main menu (0 → skip Load → 2 → 3 → 4 = About) then FIRE
/// runs `MainMenu` case 4 → `child = new AboutScreen(this, false)`.
#[test]
fn about_reached_from_main_menu_case_4() {
    let mut g = drive_to_main_menu();
    assert_eq!(
        g.main_menu.base.cursor_index, 0,
        "cursor starts on NEW GAME"
    );
    assert_eq!(menu_depth(&g), 1, "just the main menu is open");
    // The small fonts' control glyphs are hidden before About opens (initFonts).
    assert!(
        hide_controls_black(&g),
        "smallBlack.hideControls starts true (initFonts)"
    );

    // DOWN #1: 0 -> 1 (Load), then the no-save skip advances past it to 2 (Options).
    press_and_step(&mut g, KEY_DOWN);
    assert_eq!(
        g.main_menu.base.cursor_index, 2,
        "DOWN skips Load to Options"
    );
    // DOWN #2: 2 -> 3 (Help).
    press_and_step(&mut g, KEY_DOWN);
    assert_eq!(g.main_menu.base.cursor_index, 3, "DOWN to Help");
    // DOWN #3: 3 -> 4 (About).
    press_and_step(&mut g, KEY_DOWN);
    assert_eq!(g.main_menu.base.cursor_index, 4, "DOWN to About");

    // FIRE: MainMenu case 4 -> child = new AboutScreen(this, false).
    press_and_step(&mut g, KEY_FIRE);
    assert_eq!(
        g.main_menu.base.child,
        MenuChild::About,
        "FIRE on About pushes AboutScreen"
    );
    assert_eq!(menu_depth(&g), 2, "AboutScreen is now the open child");
    assert!(
        g.about_screen.base.parent,
        "super(parent, 1) -> parent present"
    );
    assert_eq!(
        g.about_screen.base.item_count, 1,
        "super(parent, 1); the lineOffsets recompute is DEFERRED so itemCount stays 1"
    );
    assert_eq!(
        g.about_screen.base.child,
        MenuChild::None,
        "AboutScreen has no child of its own"
    );
    // The constructor un-hid the small fonts' control glyphs.
    assert!(
        !hide_controls_black(&g),
        "AboutScreen ctor un-hid smallBlack.hideControls"
    );

    // Render one frame: the partial About paint must not crash.
    game_loop::run_one_frame(&mut g);

    // BACK inside AboutScreen: keyCode -8 -> parent.close() + re-hide control glyphs.
    press_and_step(&mut g, KEY_BACK);
    assert_eq!(
        g.main_menu.base.child,
        MenuChild::None,
        "AboutScreen Back closed via parent.close()"
    );
    assert_eq!(menu_depth(&g), 1, "back to just the main menu");
    assert!(
        hide_controls_black(&g),
        "AboutScreen Back re-hid smallBlack.hideControls"
    );
}

/// `ItemPickerList`'s constructor sets `itemCount = slots.length`, and vertical
/// no-wrap navigation clamps at the ends (DOWN advances, UP at the top stays).
#[test]
fn item_picker_list_ctor_and_no_wrap_nav() {
    let mut g = Game::new();
    // new ItemPickerList(parent, {0,1,2}, resultTag=5, title) → super(parent, 3).
    item_picker_list::construct(&mut g, vec![0, 1, 2], 5, "Slots".encode_utf16().collect());
    assert_eq!(
        g.item_picker_list.base.item_count, 3,
        "super(parent, slots.length) → itemCount 3"
    );
    assert_eq!(g.item_picker_list.result_tag, 5, "resultTag stored");
    assert_eq!(
        g.item_picker_list.base.cursor_index, 0,
        "cursor starts at 0"
    );

    // UP at the top: no-wrap clamps to 0.
    assert!(item_picker_list::handle_key(&mut g, 0, KEY_UP));
    assert_eq!(
        g.item_picker_list.base.cursor_index, 0,
        "UP at the top stays (no wrap)"
    );
    // DOWN twice: 0 -> 1 -> 2.
    assert!(item_picker_list::handle_key(&mut g, 0, KEY_DOWN));
    assert!(item_picker_list::handle_key(&mut g, 0, KEY_DOWN));
    assert_eq!(g.item_picker_list.base.cursor_index, 2, "two DOWNs → row 2");
    // DOWN at the bottom: no-wrap clamps to 2.
    assert!(item_picker_list::handle_key(&mut g, 0, KEY_DOWN));
    assert_eq!(
        g.item_picker_list.base.cursor_index, 2,
        "DOWN at the bottom stays (no wrap)"
    );
}

/// A `SellList` built over a bag's occupied slots has one row per occupied slot.
#[test]
fn sell_list_row_count_matches_bag_occupancy() {
    // A 10-slot bag with three occupied slots (0, 2, 5).
    let mut bag: ItemBag = item_bag::new(10);
    bag.slots[0] = Some(bag_item(0));
    bag.slots[2] = Some(bag_item(1));
    bag.slots[5] = Some(bag_item(2));
    let occupied = item_bag::occupied_slots(&bag);
    assert_eq!(
        occupied,
        vec![0i8, 2, 5],
        "occupiedSlots enumerates the three"
    );

    let mut g = Game::new();
    // new SellList(parent, occupiedSlots) → super(parent, occupiedSlots, 0, title).
    sell_list::construct(&mut g, occupied.clone());
    assert_eq!(
        g.sell_list.picker.base.item_count,
        occupied.len() as i8,
        "SellList itemCount == occupiedSlots.length (three rows)"
    );
    assert_eq!(
        g.sell_list.picker.slots, occupied,
        "the sell list lists exactly the occupied slots"
    );
    assert_eq!(
        g.sell_list.picker.result_tag, 0,
        "SellList passes resultTag 0"
    );
    assert!(g.sell_list.picker.base.parent, "parent present");
    assert!(
        g.sell_list.picker.title.is_empty(),
        "title is DEFERRED (ShopMenu.text.get(18) unported → empty placeholder)"
    );

    // NEGATIVE CONTROL: a single-item bag yields exactly one row (so the three-row
    // assertion above cannot read as a fixed constant).
    let mut bag1: ItemBag = item_bag::new(10);
    bag1.slots[7] = Some(bag_item(9));
    let occupied1 = item_bag::occupied_slots(&bag1);
    let mut g1 = Game::new();
    sell_list::construct(&mut g1, occupied1.clone());
    assert_eq!(
        g1.sell_list.picker.base.item_count, 1,
        "a one-item bag → a one-row SellList"
    );
}
