//! STATE oracle for the New-Game → class-select child transition.
//!
//! `menu_oracle.rs` proves the fresh-install main menu renders pixel-exact. THIS
//! test drives one key further — FIRE (`keyCode 53`) on NEW GAME (cursorIndex 0,
//! `!hasSave`) — and asserts `MainMenu.handleKey`'s `switch(cursorIndex)` case 0
//! pushes a `ClassSelectMenu` as the child (`MainMenu.java:150-158`). This is a
//! STATE assertion, not a pixel diff: the class-select art is not oracle-captured
//! yet, so the check is on the menu child-stack discriminant + the constructed
//! child's `Menu` base fields, plus a smoke check that a post-FIRE frame renders
//! (the generalized `menu::render` dispatches to `class_select_menu::paint`)
//! without panicking.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::menu::MenuChild;
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, font_manager, game_loop, game_midlet, title_screen, Game,
};

/// Matches `menu_oracle.rs`: the route's game-RNG seed keeps the pre-menu drive
/// deterministic (title birds consume `ByteUtil.rng`).
const GAME_RNG_SEED: i64 = 305419896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
/// SOFT1 (any key) leaves the state-1 title; at the menu it is unused here.
const KEY_SOFT1: i32 = -6;
/// FIRE / select — `keyCode 53` (KEY_NUM5). `MainMenu.handleKey` selects on
/// `keyCode == 53 || action == 8`.
const KEY_FIRE: i32 = 53;

fn key_press(g: &mut Game, code: i32) {
    g.canvas.as_mut().expect("canvas").key_pressed(code);
}

/// Drives boot → title → any-key → settled main menu with NEW GAME selected
/// (cursorIndex 0), the same path `menu_oracle::drive_menu(0)` takes.
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

/// The settled main menu is at NEW GAME with no save — the wired FIRE case.
#[test]
fn main_menu_is_at_new_game_with_no_save() {
    let g = drive_to_main_menu();
    assert_eq!(g.game_state.screen, 9, "on the main-menu screen");
    assert_eq!(
        g.main_menu.base.cursor_index, 0,
        "cursor rests on NEW GAME (index 0)"
    );
    assert!(!g.main_menu.has_save, "fresh install → no save");
    assert_eq!(
        g.main_menu.base.child,
        MenuChild::None,
        "no child pushed before FIRE"
    );
}

/// FIRE on NEW GAME pushes a `ClassSelectMenu` as the main-menu child, constructed
/// with `super(parent, 3); cursorIndex = 2;` (`MainMenu.java:150-158` →
/// `ClassSelectMenu.<init>`).
#[test]
fn fire_on_new_game_pushes_class_select_child() {
    let mut g = drive_to_main_menu();
    // Sanity: precondition is the wired case.
    assert_eq!(g.main_menu.base.cursor_index, 0);
    assert!(!g.main_menu.has_save);

    // Press FIRE (keyCode 53); the queued key is drained by the next frame's
    // serialized dispatch → game_screen::key_pressed → MainMenu.handleKey case 0.
    key_press(&mut g, KEY_FIRE);
    game_loop::run_one_frame(&mut g);

    // MainMenu.child = new ClassSelectMenu(this);
    assert_eq!(
        g.main_menu.base.child,
        MenuChild::ClassSelect,
        "New Game FIRE pushes the ClassSelect child"
    );
    // ClassSelectMenu(MainMenu parent): super(parent, (byte) 3); cursorIndex = (byte) 2;
    assert_eq!(
        g.class_select_menu.base.item_count, 3,
        "ClassSelectMenu itemCount = 3 (three class slots)"
    );
    assert_eq!(
        g.class_select_menu.base.cursor_index, 2,
        "ClassSelectMenu cursorIndex initialised to 2"
    );
    assert!(
        g.class_select_menu.base.parent,
        "ClassSelectMenu parent (the MainMenu) is present"
    );
    assert_eq!(
        g.class_select_menu.base.child,
        MenuChild::None,
        "the freshly-pushed ClassSelectMenu has no child of its own"
    );
}

/// End-to-end child-stack dispatch: after FIRE, a rendered frame walks
/// `menu::render` into `class_select_menu::paint` (the PARTIAL class-select paint)
/// without panicking, and the child stays pushed. Not a pixel check — the art is
/// not oracle-captured — only that the generalized dispatch reaches the child.
#[test]
fn post_fire_frame_dispatches_render_to_class_select() {
    let mut g = drive_to_main_menu();
    // Frame 1 drains the FIRE key (sets the child); frame 2 renders the class-select:
    // game_screen paint (screen 9) → MainMenu.draw → menu::render → render_node(Main)
    // → render_node(ClassSelect) → class_select_menu::paint.
    key_press(&mut g, KEY_FIRE);
    game_loop::run_one_frame(&mut g);
    assert_eq!(g.main_menu.base.child, MenuChild::ClassSelect);
    game_loop::run_one_frame(&mut g);

    assert_eq!(
        g.main_menu.base.child,
        MenuChild::ClassSelect,
        "child stays pushed across the render frame"
    );
    let fb = g.screen.as_ref().expect("framebuffer");
    let distinct: std::collections::HashSet<u32> =
        fb.pixels().iter().map(|&p| p & 0x00FF_FFFF).collect();
    assert!(
        distinct.len() >= 8,
        "class-select frame rendered content (>= 8 distinct colours), got {}",
        distinct.len()
    );
}
