//! STATE oracle for the full new-game menu chain:
//! `MainMenu → ClassSelectMenu → ClassConfirmMenu → StartTraitMenu`.
//!
//! `class_select.rs` proves FIRE on NEW GAME pushes `ClassSelectMenu`. THIS test
//! drives FIRE all the way through the chain — selecting a class, confirming it,
//! toggling exactly two of three guardians into `StartTraitMenu`'s `confirming`
//! state, and firing "Yes" — and asserts each child is pushed with the constructed
//! `Menu` base fields plus, at the end, that `StartTraitMenu.startGame` launched
//! `GameState.newGame` (Hero created, menu disposed). These are STATE assertions,
//! not pixel diffs: the class-select/confirm/trait art is not oracle-captured.
//!
//! Keys are driven purely by keyCode (52 = LEFT/NUM4, 53 = FIRE/NUM5, 54 =
//! RIGHT/NUM6) — the menu `handleKey`s branch on those directly, and each queued key
//! is drained by the next frame's serialized `game_screen::key_pressed` →
//! `MainMenu.handleKey` → recursive `passKeyToChild` descent to the deepest child.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::menu::MenuChild;
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, font_manager, game_loop, game_midlet, title_screen, Game,
};

/// Matches `class_select.rs` / `menu_oracle.rs`: the route's game-RNG seed keeps the
/// pre-menu drive deterministic (title birds consume `ByteUtil.rng`).
const GAME_RNG_SEED: i64 = 305419896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
/// SOFT1 (any key) leaves the state-1 title.
const KEY_SOFT1: i32 = -6;
/// FIRE / select — `keyCode 53` (KEY_NUM5).
const KEY_FIRE: i32 = 53;
/// LEFT — `keyCode 52` (KEY_NUM4).
const KEY_LEFT: i32 = 52;
/// RIGHT — `keyCode 54` (KEY_NUM6).
const KEY_RIGHT: i32 = 54;

fn key_press(g: &mut Game, code: i32) {
    g.canvas.as_mut().expect("canvas").key_pressed(code);
}

/// Presses one key and runs the frame that drains it.
fn press_and_step(g: &mut Game, code: i32) {
    key_press(g, code);
    game_loop::run_one_frame(g);
}

/// Drives boot → title → any-key → settled main menu with NEW GAME selected
/// (cursorIndex 0) — the same path `class_select::drive_to_main_menu` takes.
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

/// Drives `MainMenu → ClassSelectMenu`: FIRE on NEW GAME (cursorIndex 0, no save)
/// pushes the class picker, initialised `super(parent, 3); cursorIndex = 2;`.
#[test]
fn fire_pushes_class_select() {
    let mut g = drive_to_main_menu();
    assert_eq!(g.main_menu.base.cursor_index, 0, "cursor on NEW GAME");
    assert!(!g.main_menu.has_save, "fresh install → no save");

    press_and_step(&mut g, KEY_FIRE);

    assert_eq!(g.main_menu.base.child, MenuChild::ClassSelect);
    assert_eq!(g.class_select_menu.base.item_count, 3);
    assert_eq!(g.class_select_menu.base.cursor_index, 2);
    assert_eq!(g.class_select_menu.base.child, MenuChild::None);
}

/// Drives `ClassSelectMenu → ClassConfirmMenu`: FIRE on the default cursor (2, a
/// non-locked slot) pushes `ClassConfirmMenu(this, 6 + (2 - 2) = 6)`, initialised
/// `super(parent, 2); cursorIndex = 1;`.
#[test]
fn fire_selects_class_and_pushes_confirm() {
    let mut g = drive_to_main_menu();
    press_and_step(&mut g, KEY_FIRE); // → ClassSelect (cursorIndex 2)
    press_and_step(&mut g, KEY_FIRE); // ClassSelect FIRE → ClassConfirm

    assert_eq!(
        g.class_select_menu.base.child,
        MenuChild::ClassConfirm,
        "ClassSelect FIRE pushes ClassConfirm"
    );
    assert_eq!(
        g.class_confirm_menu.class_id, 6,
        "classId = 6 + (2 - cursorIndex=2) = 6"
    );
    assert_eq!(g.class_confirm_menu.base.item_count, 2);
    assert_eq!(
        g.class_confirm_menu.base.cursor_index, 1,
        "ClassConfirm cursorIndex initialised to 1 (No)"
    );
    assert_eq!(g.class_confirm_menu.base.child, MenuChild::None);
}

/// Drives `ClassConfirmMenu → StartTraitMenu`: move to Yes (cursorIndex 0) then
/// FIRE pushes `StartTraitMenu(this, classId=6)`, initialised `super(parent, 3);
/// guardianSelected = new boolean[3]; confirming = false;`.
#[test]
fn confirm_yes_pushes_start_trait() {
    let mut g = drive_to_main_menu();
    press_and_step(&mut g, KEY_FIRE); // → ClassSelect
    press_and_step(&mut g, KEY_FIRE); // → ClassConfirm (cursor 1 = No)
    press_and_step(&mut g, KEY_LEFT); // ClassConfirm cursor 1 → 0 (Yes)
    assert_eq!(g.class_confirm_menu.base.cursor_index, 0, "moved to Yes");

    press_and_step(&mut g, KEY_FIRE); // ClassConfirm FIRE on Yes → StartTrait

    assert_eq!(
        g.class_confirm_menu.base.child,
        MenuChild::StartTrait,
        "ClassConfirm Yes pushes StartTrait"
    );
    assert_eq!(
        g.start_trait_menu.class_id, 6,
        "StartTrait carries the chosen classId"
    );
    assert_eq!(g.start_trait_menu.base.item_count, 3);
    assert_eq!(
        g.start_trait_menu.guardian_selected,
        vec![false; 3],
        "guardianSelected = new boolean[3] (all false)"
    );
    assert!(!g.start_trait_menu.confirming, "not confirming on entry");
    assert!(
        g.game_state.hero.is_none(),
        "no game launched yet on StartTrait entry"
    );
}

/// The full chain to `StartTraitMenu`'s confirming state and the New Game launch:
/// toggle exactly two of three guardians (→ `confirming`), select "Yes", FIRE →
/// `startGame()` → `GameState.newGame(false, classId, guardianSelected)` disposes the
/// menu, creates the Hero, and leaves the front-menu screen.
#[test]
fn full_chain_reaches_confirming_and_launches_new_game() {
    let mut g = drive_to_main_menu();
    press_and_step(&mut g, KEY_FIRE); // → ClassSelect
    press_and_step(&mut g, KEY_FIRE); // → ClassConfirm (No)
    press_and_step(&mut g, KEY_LEFT); // → Yes
    press_and_step(&mut g, KEY_FIRE); // → StartTrait (cursor 0)

    // Toggle guardian 0 (FIRE), advance to guardian 1 (RIGHT), toggle it (FIRE).
    press_and_step(&mut g, KEY_FIRE); // guardianSelected[0] = true (count 1)
    assert!(g.start_trait_menu.guardian_selected[0]);
    assert!(
        !g.start_trait_menu.confirming,
        "one guardian selected → not confirming yet"
    );

    press_and_step(&mut g, KEY_RIGHT); // cursorIndex 0 → 1
    assert_eq!(g.start_trait_menu.base.cursor_index, 1);

    press_and_step(&mut g, KEY_FIRE); // guardianSelected[1] = true (count 2)
    assert!(g.start_trait_menu.guardian_selected[1]);
    assert!(
        g.start_trait_menu.confirming,
        "exactly two guardians selected → confirming"
    );
    assert!(!g.start_trait_menu.confirm_yes, "confirmYes starts false");
    assert!(
        g.game_state.hero.is_none(),
        "no game launched before firing Yes"
    );

    // In the confirmation: toggle to "Yes" (RIGHT), then FIRE → startGame().
    press_and_step(&mut g, KEY_RIGHT); // confirmYes = true
    assert!(g.start_trait_menu.confirm_yes);

    press_and_step(&mut g, KEY_FIRE); // FIRE on Yes → startGame() → GameState.newGame(...)

    // startGame() calls GameState.newGame(false, classId=6, guardianSelected={true,true,false}):
    // MainMenu is disposed, a Hero is created, the class is recorded, and the game leaves
    // the front-menu screen (setScreen(0) + requestState(21) begins the New Game load).
    assert!(
        g.game_state.hero.is_some(),
        "startGame launched newGame — the Hero was created"
    );
    assert_eq!(g.game_state.class_id, 6, "newGame recorded the chosen class");
    assert_eq!(
        g.game_state.screen, 0,
        "newGame left the front menu (screen 9 → 0; the New Game load is pending)"
    );
}
