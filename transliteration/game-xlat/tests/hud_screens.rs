//! GATE: the in-game HUD + the additive overlay screens.
//!
//! `in_game_frame.rs` proved the world tiles render (`GameScreen.paint` case 2 →
//! `GameMap.paint`). THIS gate drives the freshly-ported HUD and overlay dispatch
//! that layer over that world:
//!
//!   - [`game_screen::draw_hud`] — from a New-Game world, enable the HUD
//!     (`activate` → `worldVisible` + `redrawAll`) and prove it blits the HP bar:
//!     the red fill spans exactly `(hp * barWidth) / maxHp` pixels (`java_div`), and
//!     ink is actually present (never 0). The image-bank-bound pieces (the HUD frame,
//!     stat-point alert, quick-item icon, the numeric labels, the Guardian skill
//!     icons, the target panel) are DEFERRED in the port — this drives the portable
//!     bar geometry.
//!   - `GameScreen.paint` `case 10` (game-over) and `case 15` (paused) — drive each
//!     screen and prove `paint` DISPATCHES (the frame changes from a blank surface),
//!     the screen id is preserved across the frame, and (for case 10) the `fxTimer`
//!     fade-out ticks.
//!
//! State + liveness throughout: a real bar of the computed width (not merely "some
//! pixels"), a proven-non-blank frame per screen, and the fade counter advancing.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, font_manager, game_loop, game_midlet, game_screen, game_state,
    title_screen, Game,
};
use j2me_me::Image;

/// Matches `in_game_frame.rs`: the route's game-RNG seed keeps the pre-menu drive
/// deterministic (title birds consume `ByteUtil.rng`).
const GAME_RNG_SEED: i64 = 305419896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
/// SOFT1 (any key) leaves the state-1 title.
const KEY_SOFT1: i32 = -6;
/// Warrior — the first selectable start class (`ClassSelectMenu` default).
const CLASS_WARRIOR: i8 = 6;

/// ARGB the HP bar's primary fill lands as: `graphics.setColor(16711680)` (0xFF0000)
/// with the runtime's opaque alpha.
const HP_RED: u32 = 0xffff_0000;

fn load_resources(g: &mut Game) {
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
}

fn key_press(g: &mut Game, code: i32) {
    g.canvas.as_mut().expect("canvas").key_pressed(code);
}

/// Drives boot → title → any-key → settled main menu (screen 9), mirroring
/// `in_game_frame::drive_to_main_menu`.
fn drive_to_main_menu() -> Game {
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
    key_press(&mut g, KEY_SOFT1);
    for _ in 0..MENU_SETTLE {
        game_loop::run_one_frame(&mut g);
    }
    assert_eq!(
        g.game_state.screen, 9,
        "settled on the main menu (screen 9)"
    );
    g
}

/// Drives New Game DIRECTLY and pumps loading → world frames until `screen == 2`,
/// mirroring `in_game_frame::drive_to_world`.
fn drive_to_world() -> Game {
    let mut g = drive_to_main_menu();
    let traits = [false, false, false];
    game_state::new_game(&mut g, false, CLASS_WARRIOR, &traits);
    assert_eq!(g.game_state.screen, 0, "newGame set screen 0");

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

/// Swaps in a fresh (all-white) 240x320 framebuffer so the next paint's pixels are
/// the ONLY thing on the surface — isolating what a screen actually drew.
fn blank_framebuffer(g: &mut Game) {
    g.screen = Some(Image::create_mutable(240, 320).expect("blank framebuffer"));
}

fn framebuffer_pixels(g: &Game) -> Vec<u32> {
    g.screen.as_ref().expect("framebuffer").pixels().to_vec()
}

/// Counts pixels equal to `argb` on scan-row `y`.
fn count_on_row(g: &Game, y: i32, argb: u32) -> i32 {
    let img = g.screen.as_ref().expect("framebuffer");
    let mut count = 0;
    for x in 0..img.width() {
        if img.get(x, y) == Some(argb) {
            count += 1;
        }
    }
    count
}

#[test]
fn draw_hud_blits_the_hp_bar_at_the_computed_width() {
    let mut g = drive_to_world();
    assert_eq!(g.game_state.screen, 2, "reached the world (screen 2)");

    // The hero the HUD reads (class-6 warrior: full HP).
    let hero_id = g.game_state.hero.expect("hero materialised");
    let (hp, max_hp) = {
        let hero = g.entity_arena[hero_id].as_hero().expect("Hero data");
        (hero.hp, hero.max_hp)
    };
    assert!(hp > 0 && max_hp > 0, "the hero has live HP ({hp}/{max_hp})");

    // barWidth = BaseCanvas.width - 67 (the GameScreen ctor geometry).
    let bar_width = g.base_canvas.width - 67;
    // hudY = (height - 31) - 5; the HP red fill's top row is hudY + 20.
    let hp_row = (g.base_canvas.height - 31 - 5) + 20;
    // hpFill = (hp * barWidth) / maxHp — the faithful java_div bar math.
    let expected_fill = (hp * bar_width) / max_hp;
    assert!(
        expected_fill > 0,
        "the HP bar has a non-zero width ({expected_fill}) to blit"
    );

    // Isolate the HUD: paint it over a blank surface so only its pixels remain.
    blank_framebuffer(&mut g);
    // Enable the world HUD, then draw it.
    game_screen::activate(&mut g);
    assert!(
        g.game_screen.world_visible && g.game_screen.redraw_all,
        "activate set worldVisible + redrawAll"
    );
    game_screen::draw_hud(&mut g);

    // Liveness: the HUD actually wrote pixels.
    let blank = Image::create_mutable(240, 320).unwrap();
    assert_ne!(
        framebuffer_pixels(&g),
        blank.pixels(),
        "drawHud left the framebuffer blank — nothing was drawn"
    );

    // State: the red HP fill spans EXACTLY (hp * barWidth) / maxHp pixels.
    let red_on_row = count_on_row(&g, hp_row, HP_RED);
    eprintln!("hud: HP fill {red_on_row} px on row {hp_row} (expected {expected_fill}), hp {hp}/{max_hp}, barWidth {bar_width}");
    assert!(red_on_row > 0, "no HP-bar ink on row {hp_row}");
    assert_eq!(
        red_on_row, expected_fill,
        "HP bar width {red_on_row} != (hp*barWidth)/maxHp = {expected_fill}"
    );

    // The dirty flag was cleared by the HP block (redrawAll path also drew it).
    assert!(!g.game_screen.hp_dirty, "the HP block cleared hpDirty");
}

#[test]
fn draw_hud_bar_is_absent_before_it_is_drawn() {
    // Negative control: the HP-fill scan is meaningful only because a blank frame
    // carries no HP ink — proving the bar the positive test sees is really drawn.
    let mut g = drive_to_world();
    let hp_row = (g.base_canvas.height - 31 - 5) + 20;
    blank_framebuffer(&mut g);
    assert_eq!(
        count_on_row(&g, hp_row, HP_RED),
        0,
        "a blank framebuffer already had HP-red pixels — the HP-bar assertion is vacuous"
    );
}

#[test]
fn paint_case_10_game_over_dispatches_and_ticks_the_fade() {
    let mut g = drive_to_world();
    // Land on the game-over screen with the fade timer armed so the fxTimer==0
    // fallback (setScreen(1) + the DEFERRED loadMainMenu) does not fire this frame.
    game_state::clear_request(&mut g);
    game_state::set_screen(&mut g, 10);
    g.game_screen.fx_timer = 16;

    blank_framebuffer(&mut g);
    let before = framebuffer_pixels(&g);
    game_screen::paint(&mut g);
    let after = framebuffer_pixels(&g);

    // Dispatch: the frame changed (drawGameOver filled the surface black).
    assert_ne!(before, after, "case-10 paint drew nothing (no dispatch)");
    assert!(
        count_on_row(&g, 160, 0xff00_0000) > 0,
        "drawGameOver did not fill black"
    );
    // The screen is preserved (fxTimer > 0 → no fallback to screen 1).
    assert_eq!(g.game_state.screen, 10, "still on game-over (screen 10)");
    // The fade advanced by one tick.
    assert_eq!(g.game_screen.fx_timer, 15, "fxTimer ticked 16 -> 15");
}

#[test]
fn paint_case_15_paused_dispatches() {
    let mut g = drive_to_world();
    game_state::clear_request(&mut g);
    game_state::set_screen(&mut g, 15);

    blank_framebuffer(&mut g);
    let before = framebuffer_pixels(&g);
    game_screen::paint(&mut g);
    let after = framebuffer_pixels(&g);

    // Dispatch: FontManager.clearScreen filled the surface black + the "Ok" soft key.
    assert_ne!(before, after, "case-15 paint drew nothing (no dispatch)");
    assert!(
        count_on_row(&g, 160, 0xff00_0000) > 0,
        "clearScreen did not fill black"
    );
    // The paused screen is preserved.
    assert_eq!(g.game_state.screen, 15, "still paused (screen 15)");
}
