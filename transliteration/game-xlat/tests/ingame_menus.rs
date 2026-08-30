//! MILESTONE gate: the IN-GAME MENUS OPEN.
//!
//! `in_game_frame.rs` proved New Game reaches the world (screen 2) and renders the
//! tiles; `hero_moves.rs` proved movement. THIS gate proves the two in-game menus
//! wired this lane actually OPEN in play and that `GameScreen.paint` dispatches to
//! the ported menus:
//!
//!   * **CharacterMenu (screen 5)** — reached from the world by the back/menu
//!     soft-key: `GameScreen.keyPressed(-7 → -8)` → `handlePlayKey` case `-8` (with
//!     the hero idle) → `GameState.requestState(13)`. The next `GameScreen.paint`
//!     drains that request (`processStateRequest` case 13 → `setScreen(5)` +
//!     `CharacterMenu.instance().open()`) and then dispatches `paint` case 5
//!     (`invalidateUp`/`invalidateDown`/`draw`) onto the ported `CharacterMenu`.
//!   * **ShopMenu (screen 6)** — reached from a merchant event op
//!     `requestState(11, 0)`. `EventScript` (the op source) is unported, so the test
//!     issues that exact request directly — the faithful call the event would make.
//!     The next `paint` drains it (`processStateRequest` case 11 arg0 0 →
//!     `setScreen(6)` + `ShopMenu.instance().loadStrings()`) and dispatches `paint`
//!     case 6 (`drawWorldBehindMenu` + `draw`) onto the ported `ShopMenu`.
//!
//! Both are proven with STATE assertions (the screen transitioned, the singleton was
//! created, the child/label-table were populated) AND a FRAME assertion (the paint
//! dispatch drew the menu over the world frame it was reached from). A proven-red
//! control shows the transition is gated on the request: painting the world with no
//! request queued leaves screen 2 and opens neither menu.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::menu::MenuChild;
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, font_manager, game_loop, game_midlet, game_screen, game_state,
    title_screen, Game,
};

const GAME_RNG_SEED: i64 = 305_419_896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
/// SOFT1 (any key) leaves the state-1 title.
const KEY_SOFT1: i32 = -6;
/// Warrior — the first selectable start class.
const CLASS_WARRIOR: i8 = 6;
/// The world back/menu soft-key: raw `-7` remaps to `-8` in `keyPressed`, and
/// `handlePlayKey` case `-8` (hero idle) issues `requestState(13)` → open the
/// character menu.
const KEY_SOFT2_BACK: i32 = -7;

fn load_resources(g: &mut Game) {
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
}

/// Drives boot → title → any-key → settled main menu (screen 9), then New Game
/// DIRECTLY, pumping loading → world frames until the world screen (2) renders.
/// Mirrors `in_game_frame::drive_to_world` / `hero_moves::drive_to_world`.
fn drive_to_world() -> Game {
    let mut g = Game::new();
    g.byte_util = byte_util::ByteUtilState::seeded(GAME_RNG_SEED);
    load_resources(&mut g);

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
    g.canvas.as_mut().expect("canvas").key_pressed(KEY_SOFT1);
    for _ in 0..MENU_SETTLE {
        game_loop::run_one_frame(&mut g);
    }
    assert_eq!(
        g.game_state.screen, 9,
        "settled on the main menu (screen 9)"
    );

    // GameState.newGame(false, classId=6, traits) — bypass the menu chain.
    let traits = [false, false, false];
    game_state::new_game(&mut g, false, CLASS_WARRIOR, &traits);

    // Pump loading → world frames until the world screen (2) is reached and rendered.
    let mut guard = 0u32;
    while g.game_state.screen != 2 {
        game_loop::run_one_frame(&mut g);
        guard += 1;
        assert!(
            guard < 100,
            "New Game drive never reached screen 2 (stuck at screen {})",
            g.game_state.screen
        );
    }
    g
}

fn frame_pixels(g: &Game) -> Vec<u32> {
    g.screen.as_ref().expect("framebuffer").pixels().to_vec()
}

/// A drawn frame is non-blank: at least one non-white pixel exists.
fn has_non_white(g: &Game) -> bool {
    let img = g.screen.as_ref().expect("framebuffer");
    (0..img.height())
        .any(|y| (0..img.width()).any(|x| img.get(x, y).is_some_and(|p| p != 0xffff_ffff)))
}

/// The world back/menu soft-key opens the CharacterMenu (screen 5) and `GameScreen.paint`
/// case 5 draws it over the world frame it was reached from.
#[test]
fn world_back_key_opens_the_character_menu() {
    let mut g = drive_to_world();

    // Baseline: the world is showing (screen 2), the hero is idle, no menu open.
    assert_eq!(g.game_state.screen, 2, "world screen (2)");
    assert_eq!(
        game_state::hero_state(&g),
        1,
        "the hero is idle (state 1) before any input"
    );
    assert!(
        !g.character_menu.singleton,
        "the character menu is not open yet"
    );
    let world_frame = frame_pixels(&g);

    // Press the back/menu soft-key: keyPressed(-7 → -8) → handlePlayKey case -8 →
    // requestState(13). The request is queued but not yet processed (still screen 2).
    game_screen::key_pressed(&mut g, KEY_SOFT2_BACK);
    assert_eq!(
        g.game_state.next_state, 13,
        "the back key queued requestState(13)"
    );
    assert_eq!(
        g.game_state.screen, 2,
        "still the world until the request is processed"
    );

    // One paint drains the request (processStateRequest case 13 → setScreen(5) +
    // CharacterMenu.instance().open()) and dispatches paint case 5.
    game_screen::paint(&mut g);

    // STATE: transitioned to the character menu, singleton created + opened on the
    // status tab, and open() loaded the shared /sgui/gm label table.
    assert_eq!(g.game_state.screen, 5, "screen → 5 (character menu)");
    assert!(
        g.character_menu.singleton,
        "CharacterMenu.instance() created the singleton"
    );
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::Status,
        "opened on the status tab (StatusPage child)"
    );
    assert!(
        g.character_menu.text.is_some(),
        "open() loaded the /sgui/gm label table"
    );

    // FRAME: paint case 5 drew the character menu over the world frame.
    let menu_frame = frame_pixels(&g);
    assert_ne!(
        menu_frame, world_frame,
        "paint case 5 left the frame identical to the world — the menu was not drawn"
    );
    assert!(has_non_white(&g), "the character-menu frame is blank");
}

/// A merchant event's `requestState(11, 0)` opens the ShopMenu (screen 6) and
/// `GameScreen.paint` case 6 draws it over the world frame.
#[test]
fn shop_request_opens_the_shop_menu() {
    let mut g = drive_to_world();

    assert_eq!(g.game_state.screen, 2, "world screen (2)");
    assert!(!g.shop_menu.singleton, "the shop is not open yet");
    let world_frame = frame_pixels(&g);

    // The shop is opened by a merchant event op (requestState(11, 0)); EventScript is
    // unported, so issue that exact request directly — the faithful call the event makes.
    game_state::request_state_a0(&mut g, 11, 0);
    assert_eq!(g.game_state.next_state, 11, "requestState(11, 0) queued");
    assert_eq!(g.game_state.arg0, 0, "arg0 == 0 selects the shop sub-arm");

    // One paint drains the request (processStateRequest case 11 arg0 0 → setScreen(6) +
    // ShopMenu.instance().loadStrings()) and dispatches paint case 6.
    game_screen::paint(&mut g);

    // STATE: transitioned to the shop, singleton created over the decoded stock, the
    // first category's ShopItemList pushed, and loadStrings loaded /sgui/shop.
    assert_eq!(g.game_state.screen, 6, "screen → 6 (shop)");
    assert!(
        g.shop_menu.singleton,
        "ShopMenu.instance() created the singleton"
    );
    assert_eq!(
        g.shop_menu.base.child,
        MenuChild::ShopItemList,
        "the constructor pushed the first category's ShopItemList"
    );
    assert_eq!(
        g.shop_menu.shop_stock.len(),
        6,
        "buildShopStock returned six category vectors"
    );
    assert!(
        g.shop_menu.text.is_some(),
        "loadStrings loaded the /sgui/shop label table"
    );

    // FRAME: paint case 6 drew the shop panel over the world frame.
    let shop_frame = frame_pixels(&g);
    assert_ne!(
        shop_frame, world_frame,
        "paint case 6 left the frame identical to the world — the shop was not drawn"
    );
    assert!(has_non_white(&g), "the shop frame is blank");
}

/// Proven-red control (GATES.md R3): the menu open is gated on the queued request.
/// Painting the world with NO request queued leaves screen 2 and opens neither menu —
/// so the transitions above cannot read as something the world paint does on its own.
#[test]
fn no_request_leaves_the_world_and_opens_no_menu() {
    let mut g = drive_to_world();
    assert_eq!(g.game_state.screen, 2, "world screen (2)");
    assert_eq!(g.game_state.next_state, 0, "no request is queued");

    game_screen::paint(&mut g);

    assert_eq!(
        g.game_state.screen, 2,
        "with no request the world stays screen 2"
    );
    assert!(
        !g.character_menu.singleton,
        "no request → the character menu did not open"
    );
    assert!(!g.shop_menu.singleton, "no request → the shop did not open");
}
