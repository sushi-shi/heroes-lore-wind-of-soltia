//! MILESTONE gate: THE HERO MOVES.
//!
//! `in_game_frame.rs` proved New Game reaches the world (screen 2) and renders the
//! tiles with the hero standing still. THIS gate proves the next milestone —
//! **player movement** — end to end through the real input + simulation path:
//!
//!   world showing (screen 2) → a directional key (`GameScreen.keyPressed` case 2 →
//!   `handlePlayKey` → `GameState.walkHero`) puts the hero into the stepping state →
//!   each frame `GameScreen.paint` case 2 runs `GameState.update()` (→ `updateHero`
//!   → `Hero.update` → `Battler.move(8)`, the 8px sub-tile step that toggles the
//!   off-grid flags and re-derives the tile) AND eases the follow-camera
//!   (`GameState.scrollCamera(true, true)`) so the world scrolls to track the hero.
//!
//! A single `keyPressed` models a HELD key: `walkHero` latches `state = 2`
//! (stepping), and the hero keeps stepping every frame until a release (`stopHero`)
//! — so pumping N frames walks N sub-tile steps with no further input.
//!
//! Collision is DEFERRED (the `.evt` collision grid is not parsed): `tryStepForward`
//! is stubbed to never block, so the hero walks the open tile grid.
//!
//! The gate proves motion three ways — the hero's world pixel position ADVANCED in
//! the pressed direction, the camera (`camX`) MOVED to follow, and the rendered
//! frame CHANGED — backed by a proven-red control: with NO input the hero stays put.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, entity::EntityId, font_manager, game_loop, game_midlet, game_screen,
    game_state, title_screen, Game,
};
use j2me_me::Image;

const GAME_RNG_SEED: i64 = 305_419_896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
/// SOFT1 (any key) leaves the state-1 title.
const KEY_SOFT1: i32 = -6;
/// Warrior — the first selectable start class.
const CLASS_WARRIOR: i8 = 6;
/// Keypad `4` → `handlePlayKey` case 52 → `walkHero((byte) 3)` (LEFT).
///
/// The class-6 start places the hero at tile (22,4) on the 30×30 map, which centres
/// the camera exactly at its RIGHT clamp (`camX == GameScreen.width - map.widthPx`).
/// Walking right there keeps the viewport pinned at the map edge (the follow camera
/// clamps), so the rendered frame would not change. Walking LEFT moves away from the
/// edge, so the camera unclamps and the tile window actually scrolls — exercising all
/// three milestone signals (hero advances, camera follows, frame changes).
const KEY_NUM4_LEFT: i32 = 52;
/// How many world frames to pump while the direction key is held.
const WALK_FRAMES: u32 = 5;

fn load_resources(g: &mut Game) {
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
}

/// Drives boot → title → any-key → settled main menu (screen 9), then New Game
/// DIRECTLY, pumping loading → world frames until the world screen (2) renders.
/// Mirrors `in_game_frame::drive_to_world`, replicated here from the public API.
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

/// The hero's world pixel position (`(pixelX, pixelY)`), read from the arena.
fn hero_pixel(g: &Game, id: EntityId) -> (i16, i16) {
    let node = &g.entity_arena[id];
    (node.pixel_x, node.pixel_y)
}

/// The hero's tile coordinate (`(tileX, tileY)`).
fn hero_tile(g: &Game, id: EntityId) -> (i8, i8) {
    let node = &g.entity_arena[id];
    (node.tile_x, node.tile_y)
}

fn frame_pixels(g: &Game) -> Vec<u32> {
    g.screen.as_ref().expect("framebuffer").pixels().to_vec()
}

#[test]
fn held_direction_walks_the_hero_and_scrolls_the_camera() {
    let mut g = drive_to_world();
    let hero_id = g.game_state.hero.expect("hero materialised");

    // Baseline: the world is showing, the hero is idle at its start tile.
    assert_eq!(g.game_state.screen, 2, "world screen (2)");
    assert_eq!(
        game_state::hero_state(&g),
        1,
        "the hero is idle (state 1) before any input"
    );
    let (x0, y0) = hero_pixel(&g, hero_id);
    let (tx0, ty0) = hero_tile(&g, hero_id);
    let (cam_x0, cam_y0) = (g.game_state.cam_x, g.game_state.cam_y);
    let frame0 = frame_pixels(&g);

    // Press-and-hold LEFT: keyPressed case 2 → handlePlayKey(52) → walkHero(3),
    // which latches the stepping state (2) + facing left (3).
    game_screen::key_pressed(&mut g, KEY_NUM4_LEFT);
    assert_eq!(
        game_state::hero_state(&g),
        2,
        "walkHero put the hero into the stepping state (2)"
    );
    assert_eq!(
        game_state::hero_facing(&g),
        3,
        "walkHero faced the hero LEFT (3)"
    );

    // Hold for N frames — no further input; the hero keeps stepping each frame.
    for _ in 0..WALK_FRAMES {
        game_loop::run_one_frame(&mut g);
    }

    let (x1, y1) = hero_pixel(&g, hero_id);
    let (tx1, ty1) = hero_tile(&g, hero_id);
    let (cam_x1, cam_y1) = (g.game_state.cam_x, g.game_state.cam_y);
    let frame1 = frame_pixels(&g);

    eprintln!(
        "hero_moves: pixel ({x0},{y0}) -> ({x1},{y1}); tile ({tx0},{ty0}) -> ({tx1},{ty1}); \
         cam ({cam_x0},{cam_y0}) -> ({cam_x1},{cam_y1}); {} frames",
        WALK_FRAMES
    );

    // 1. The hero ADVANCED left (pixelX down by exactly 8 * N; ~one tile per 2 frames).
    assert_eq!(
        x1 as i32,
        x0 as i32 - 8 * WALK_FRAMES as i32,
        "pixelX advanced left by 8px per held frame"
    );
    assert!(x1 < x0, "the hero moved in the pressed (left) direction");
    assert_eq!(y1, y0, "walking left did not change pixelY");
    assert!(tx1 < tx0, "the hero crossed to a new tile column");
    assert_eq!(ty1, ty0, "the tile row is unchanged");

    // 2. The camera FOLLOWED — camX eased right (less negative) as the hero moved left,
    //    scrolling the world so the hero stays centred (and unclamping from the edge).
    assert!(
        cam_x1 > cam_x0,
        "camX moved to follow the hero (world scrolled): {cam_x0} -> {cam_x1}"
    );
    let _ = (cam_y0, cam_y1); // camY is the facing-locked axis for horizontal walk.

    // 3. The rendered FRAME CHANGED (the tile window shifted with the camera).
    assert_ne!(
        frame1, frame0,
        "the world frame is unchanged — the camera scroll drew no new pixels"
    );

    // The frame is still a real, non-blank world frame after moving.
    let painted = g.screen.as_ref().expect("framebuffer");
    let blank = Image::create_mutable(painted.width(), painted.height()).unwrap();
    assert_ne!(painted.pixels(), blank.pixels(), "the moved frame is blank");
}

/// Proven-red control (GATES.md R3): with NO input the hero does NOT move — the same
/// N frames pumped without a key leave the hero's position exactly where it started.
/// If movement leaked without input, this would fail.
#[test]
fn no_input_leaves_the_hero_stationary() {
    let mut g = drive_to_world();
    let hero_id = g.game_state.hero.expect("hero materialised");

    assert_eq!(
        game_state::hero_state(&g),
        1,
        "the hero starts idle (state 1)"
    );
    let (x0, y0) = hero_pixel(&g, hero_id);
    let (tx0, ty0) = hero_tile(&g, hero_id);

    // Pump the SAME number of frames, with no key pressed.
    for _ in 0..WALK_FRAMES {
        game_loop::run_one_frame(&mut g);
    }

    let (x1, y1) = hero_pixel(&g, hero_id);
    let (tx1, ty1) = hero_tile(&g, hero_id);
    assert_eq!(
        (x0, y0),
        (x1, y1),
        "the hero moved with no input: ({x0},{y0}) -> ({x1},{y1})"
    );
    assert_eq!(
        (tx0, ty0),
        (tx1, ty1),
        "the hero's tile changed with no input"
    );
    assert_eq!(
        game_state::hero_state(&g),
        1,
        "the hero stayed idle (state 1) with no input"
    );
}
