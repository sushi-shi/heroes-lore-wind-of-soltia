//! STATE oracle for the front-menu dialogs ported this lane:
//! `OptionsMenu`, `PopupMenu` and `ConfirmDialog` — driven through the `MainMenu`
//! popup machinery (`showPopup`/`onPopupResult`/`close`).
//!
//! `menu_chain.rs` proves the New-Game child chain; THIS test drives the sibling
//! front-menu transitions:
//!   * DOWN then FIRE on the settled main menu opens `OptionsMenu` (`MainMenu` case
//!     2), witnessed by the child discriminant + the menu-stack depth + the pushed
//!     `Menu` base fields;
//!   * Back inside `OptionsMenu` runs `parent.close()`, unlinking it (depth back to 1);
//!   * a `PopupMenu` round-trip through the real exit path: Back on the main menu
//!     opens the type-2 confirm popup (`showPopup`), and FIRE resolves it through
//!     `MainMenu.onPopupResult` (pendingAction 2 → the demo-splash arm);
//!   * a `ConfirmDialog` round-trip: pushed as `MainMenu`'s child (its game creator
//!     `SkillTab` is unported), rendered once, then OK resolves it through
//!     `onPopupResult` back to the base dismiss.
//!
//! These are STATE assertions, not pixel diffs: the options/popup/dialog art is
//! partial (DEFERRED — it crosses into unported `FontManager`/`AssetCache` statics).
//! Keys are driven purely by keyCode (52 LEFT, 53 FIRE, 54 RIGHT, 56 DOWN, -8 BACK),
//! each drained by the next frame's serialized `game_screen::key_pressed`.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::menu::MenuChild;
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, confirm_dialog, font_manager, game_loop, game_midlet, menu,
    title_screen, Game,
};

/// Matches `menu_chain.rs`: the route's game-RNG seed keeps the pre-menu drive
/// deterministic (title birds consume `ByteUtil.rng`).
const GAME_RNG_SEED: i64 = 305419896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
/// SOFT1 (any key) leaves the state-1 title.
const KEY_SOFT1: i32 = -6;
/// FIRE / select — `keyCode 53` (KEY_NUM5).
const KEY_FIRE: i32 = 53;
/// DOWN — `keyCode 56` (KEY_NUM8).
const KEY_DOWN: i32 = 56;
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
/// main menu, 2 = one child open, …) — the flat model's `menu_depth` witness, read
/// from each concrete menu's `Menu.child` discriminant.
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
        };
        depth += 1;
        child = next;
    }
    depth
}

/// Drives boot → title → any-key → settled main menu with NEW GAME selected
/// (cursorIndex 0) — the same path `menu_chain::drive_to_main_menu` takes.
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

/// DOWN then FIRE on the settled main menu opens `OptionsMenu` (`MainMenu` case 2):
/// the disabled-Load skip lands the cursor on Options (index 2), and FIRE pushes
/// `OptionsMenu(this, false)` initialised `super(parent, 4)`.
#[test]
fn down_then_fire_opens_options() {
    let mut g = drive_to_main_menu();
    assert_eq!(
        g.main_menu.base.cursor_index, 0,
        "cursor starts on NEW GAME"
    );
    assert_eq!(menu_depth(&g), 1, "just the main menu is open");

    // DOWN: 0 -> 1 (Load), then the no-save skip advances past it to 2 (Options).
    press_and_step(&mut g, KEY_DOWN);
    assert_eq!(
        g.main_menu.base.cursor_index, 2,
        "DOWN skips the disabled Load row to Options"
    );

    // FIRE: MainMenu case 2 -> child = new OptionsMenu(this, false).
    press_and_step(&mut g, KEY_FIRE);
    assert_eq!(
        g.main_menu.base.child,
        MenuChild::Options,
        "FIRE on Options pushes OptionsMenu"
    );
    assert_eq!(menu_depth(&g), 2, "OptionsMenu is now the open child");
    assert_eq!(g.options_menu.base.item_count, 4, "super(parent, 4)");
    assert_eq!(
        g.options_menu.base.cursor_index, 0,
        "cursor starts at row 0"
    );
    assert!(
        g.options_menu.game_loop,
        "gameLoop = GameLoop.instance (present)"
    );
    assert!(
        !g.options_menu.in_game,
        "opened from the title menu → inGame = false"
    );
    assert_eq!(
        g.options_menu.base.child,
        MenuChild::None,
        "OptionsMenu has no child of its own"
    );
}

/// Back inside `OptionsMenu` runs `parent.close()` — the flat model unlinks it from
/// `MainMenu` (depth back to 1), exercising `Menu.close` + `invalidateUp`.
#[test]
fn options_back_closes_via_parent_close() {
    let mut g = drive_to_main_menu();
    press_and_step(&mut g, KEY_DOWN); // -> Options (cursor 2)
    press_and_step(&mut g, KEY_FIRE); // -> OptionsMenu child
    assert_eq!(g.main_menu.base.child, MenuChild::Options);

    // BACK inside OptionsMenu: keyCode -8 (getGameAction 0) -> parent.close().
    press_and_step(&mut g, KEY_BACK);
    assert_eq!(
        g.main_menu.base.child,
        MenuChild::None,
        "OptionsMenu Back closed via parent.close()"
    );
    assert_eq!(menu_depth(&g), 1, "back to just the main menu");
    assert!(
        g.main_menu.base.needs_repaint,
        "close() -> invalidateUp marked MainMenu dirty"
    );
}

/// A `PopupMenu` open+resolve round-trip through the real exit path: Back on the
/// main menu opens the type-2 confirm popup via `showPopup`, and FIRE resolves it
/// through `MainMenu.onPopupResult` (the base dismiss + the pendingAction-2 demo
/// arm).
#[test]
fn popup_menu_open_and_resolve() {
    let mut g = drive_to_main_menu();
    assert_eq!(menu_depth(&g), 1);

    // BACK on the main menu (no child): showPopup((byte) 2, (byte) 2, {confirmPrompt}).
    press_and_step(&mut g, KEY_BACK);
    assert_eq!(
        g.main_menu.base.child,
        MenuChild::Popup,
        "Back opened the exit-confirm PopupMenu"
    );
    assert_eq!(g.popup_menu.popup_type, 2, "type 2 yes-no confirm");
    assert_eq!(
        g.popup_menu.base.item_count, 2,
        "showPopup tag 2 -> itemCount 2"
    );
    assert_eq!(
        g.main_menu.pending_action, 2,
        "pendingAction armed to exit (2)"
    );
    assert_eq!(menu_depth(&g), 2, "the popup is the open child");
    assert_eq!(
        g.main_menu.demo_expiry, 0,
        "demoExpiry not armed until the popup resolves"
    );

    // FIRE on the popup: onPopupResult(2, 0) -> super dismiss + pendingAction 2 arm.
    press_and_step(&mut g, KEY_FIRE);
    assert_eq!(
        g.main_menu.base.child,
        MenuChild::None,
        "onPopupResult dismissed the popup"
    );
    assert_eq!(menu_depth(&g), 1, "back to just the main menu");
    assert!(
        g.main_menu.demo_expiry > 0,
        "pendingAction 2 (!fullVersion) armed the demo splash — onPopupResult ran"
    );
}

/// A `ConfirmDialog` open+resolve round-trip. Its game creator (`SkillTab`) is
/// unported, so it is pushed directly as `MainMenu`'s child; a frame renders its
/// partial paint, then OK (FIRE) reports `onPopupResult(resultTag, 1)`, which — with
/// a non-exit tag — runs the base dismiss (child unlinked, depth back to 1).
#[test]
fn confirm_dialog_open_and_resolve() {
    let mut g = drive_to_main_menu();

    // new ConfirmDialog(MainMenu, line1, line2, tag=7); ((Menu) main).child = it.
    let line1: Vec<u16> = "Learn skill?".encode_utf16().collect();
    let line2: Vec<u16> = "It costs SP.".encode_utf16().collect();
    confirm_dialog::construct(&mut g, line1, line2, 7);
    g.main_menu.base.child = MenuChild::Confirm;
    assert_eq!(g.confirm_dialog.result_tag, 7, "resultTag stored");
    assert_eq!(g.confirm_dialog.base.item_count, 0, "super(parent, 0)");
    assert_eq!(menu_depth(&g), 2, "ConfirmDialog is the open child");

    // Render one frame (the partial paint must not crash) then resolve with OK.
    game_loop::run_one_frame(&mut g);
    // OK: keyCode 53 -> parent.onPopupResult(resultTag=7, 1). tag 7 is not 2/12, so the
    // base dismiss runs: child = null.
    press_and_step(&mut g, KEY_FIRE);
    assert_eq!(
        g.main_menu.base.child,
        MenuChild::None,
        "ConfirmDialog OK resolved back through the base onPopupResult"
    );
    assert_eq!(menu_depth(&g), 1, "back to just the main menu");
}

/// The popup machinery drives the parent-scan directly: `show_message` pushes a
/// type-1 message popup, and its OK resolves through `onPopupResult` — a
/// tag-1 (non-exit) result, so the base dismiss unlinks it. Confirms
/// `Menu.showMessage` + `parent_of` in isolation from the exit-path side effects.
#[test]
fn show_message_popup_round_trips() {
    let mut g = drive_to_main_menu();
    let lines = vec!["Saved.".encode_utf16().collect::<Vec<u16>>()];
    menu::show_message(&mut g, menu::MenuNode::Main, lines);
    assert_eq!(g.main_menu.base.child, MenuChild::Popup);
    assert_eq!(g.popup_menu.popup_type, 1, "showMessage -> type 1");
    // labelOk (ported) is the default positive label for a type-1 popup.
    assert!(
        g.popup_menu.positive_label.is_some(),
        "type-1 popup defaults positiveLabel to FontManager.labelOk"
    );
    assert_eq!(menu_depth(&g), 2);

    press_and_step(&mut g, KEY_FIRE);
    assert_eq!(
        g.main_menu.base.child,
        MenuChild::None,
        "type-1 OK dismissed the message popup"
    );
    assert_eq!(
        g.main_menu.demo_expiry, 0,
        "tag 1 is not an exit tag — no demo arm"
    );
    assert_eq!(menu_depth(&g), 1);
}
