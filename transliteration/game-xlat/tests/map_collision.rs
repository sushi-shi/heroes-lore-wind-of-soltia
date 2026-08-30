//! REGRESSION gate: THE COLLISION GRID + THE WALK-BLOCK (crash #1).
//!
//! The port's #1 documented runtime crash is a walk running an actor off the tile
//! grid: `Battler.move`'s `Debug.assertTrue` bounds checks (and the `occupancy`
//! index it re-registers into) fire the moment the hero steps past the map edge,
//! because the `/m/<classId>/<NN>.evt` collision parse was DEFERRED — the collision
//! grid was never populated and never read, so nothing ever reported a tile blocked.
//!
//! The crash-class doctrine's fix is to PORT the collapsed collision load, not to
//! bolt on a guard the bytecode lacks. So [`game_map::load`] now reads the `.evt` and
//! runs `parseCollision` (section 1) into `GameMapState::collision_grid`, and the
//! block-tests `GameMap.isWalkable`/`isWalkableSpan`/`canOccupy`/`canStep` are ported.
//! `canStep` is exactly the verdict `Battler.tryStepForward` consults to halt a walk
//! at a wall: `canStep == false` IS "the tile ahead is blocked".
//!
//! This gate proves the ported collision load is real and correct. (a) The grid is
//! populated with the right dimensions and a mix of blocked / walkable tiles,
//! cross-checked BYTE-FOR-BYTE against the independent `hlws-formats` `.evt` parser
//! (two implementations, one truth). (b) The block verdict is right and PANIC-SAFE at
//! the exact boundary crash #1 fires at: `canStep`/`isWalkable` toward an in-map wall
//! AND toward the all-blocked map border return "blocked" without indexing off the grid
//! (the `i >= 0 && …` bounds conjuncts short-circuit before the index), so once the walk
//! consults `canStep` the hero halts one tile short of the edge instead of crashing.
//! (c) A proven contrast: an OPEN direction is walkable and the hero physically ADVANCES
//! when walked that way, while walking the in-bounds grid never panics.
//!
//! NOTE ON OBSERVABLE HALT. Making the hero *visibly* stop at a wall is one further
//! edit — un-collapsing the shared `Battler.tryStepForward` stub (`battler.rs`, still
//! "never blocked") to return `canStep`. That is deliberately NOT done in this lane:
//! it also changes Enemy/knockback stepping (the same shared `tryStepForward`) and so
//! is coupled to the entity lane's oracles. This gate therefore asserts the block at
//! the ported-predicate level — `game_map::can_step` — which is the value that halt
//! will read, and the substance of the crash-#1 fix.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_formats::{evt as fmt_evt, map as fmt_map};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, entity::EntityId, font_manager, game_loop, game_map, game_midlet,
    game_screen, game_state, title_screen, Game,
};

const GAME_RNG_SEED: i64 = 305_419_896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
/// SOFT1 (any key) leaves the state-1 title.
const KEY_SOFT1: i32 = -6;
/// Warrior — the class-6 start (`classStartTable {0, 22, 4}` → `/m/00.map`, hero
/// spawned at tile (22,4)); its `.evt` is `/m/6/00.evt`.
const CLASS_WARRIOR: i8 = 6;
/// Keypad `4` → `handlePlayKey` case 52 → `walkHero(3)` (LEFT).
const KEY_NUM4_LEFT: i32 = 52;
/// Keypad `6` → `handlePlayKey` case 54 → `walkHero(4)` (RIGHT).
const KEY_NUM6_RIGHT: i32 = 54;
/// Facing byte for `canStep`: 3 = left, 4 = right (`Directions`).
const DIR_LEFT: i8 = 3;
const DIR_RIGHT: i8 = 4;

fn load_resources(g: &mut Game) {
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
}

/// Boot → title → any-key → settled main menu (screen 9) → New Game (class 6) DIRECTLY,
/// pumping loading → world frames until the world screen (2) renders. Mirrors
/// `hero_moves::drive_to_world`, replicated here from the public API.
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

    let traits = [false, false, false];
    game_state::new_game(&mut g, false, CLASS_WARRIOR, &traits);

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
    assert!(g.game_state.map.is_some(), "GameState.map materialised");
    g
}

fn hero_id(g: &Game) -> EntityId {
    g.game_state.hero.expect("hero materialised")
}

fn hero_pixel_x(g: &Game) -> i32 {
    g.entity_arena[hero_id(g)].pixel_x as i32
}

/// Part (a): the collision grid is parsed into `GameMapState::collision_grid`, is the
/// map's size, holds a real mix of blocked/walkable cells, and equals — byte for byte —
/// the independently-reversed `hlws-formats` `.evt` parse of the same blob.
#[test]
fn collision_grid_populated_matches_independent_evt_parse() {
    let g = drive_to_world();

    let map = g.game_state.map.as_ref().expect("GameState.map");
    let grid = map
        .collision_grid
        .as_ref()
        .expect("collision_grid populated by load() (was DEFERRED/None before this lane)");

    // Dimensions: heightTiles rows × widthTiles cols (the map is 30×30).
    let h = grid.len();
    assert!(h > 0, "collision grid has rows");
    let w = grid[0].len();
    assert!(grid.iter().all(|row| row.len() == w), "rectangular grid");
    assert_eq!(h as i32, map.height_tiles, "rows == heightTiles");
    assert_eq!(w as i32, map.width_tiles, "cols == widthTiles");

    // A real collision map has BOTH blocked (< 0) and walkable (>= 0) tiles — not a
    // vacuous all-zero grid (which would read as "walk anywhere" and never block).
    let blocked = grid.iter().flatten().filter(|&&c| c < 0).count();
    let walkable = grid.iter().flatten().filter(|&&c| c >= 0).count();
    assert!(blocked > 0, "some tiles are blocked (< 0): {blocked}");
    assert!(walkable > 0, "some tiles are walkable (>= 0): {walkable}");
    assert_eq!(blocked + walkable, w * h, "every cell classified");

    // Two implementations, one truth: cross-check the transliterated grid against the
    // independent hlws-formats `.map`/`.evt` parse of the same JAR blobs.
    let map_bytes = jar()
        .get("m/00.map")
        .expect("m/00.map present in JAR")
        .to_vec();
    let fmap = fmt_map::parse(&map_bytes).expect("hlws-formats parses /m/00.map");
    assert_eq!(
        (fmap.w as i32, fmap.h as i32),
        (map.width_tiles, map.height_tiles)
    );
    let evt_bytes = jar()
        .get("m/6/00.evt")
        .expect("m/6/00.evt present in JAR")
        .to_vec();
    let fevt = fmt_evt::parse(&evt_bytes, fmap.w, fmap.h).expect("hlws-formats parses /m/6/00.evt");
    assert_eq!(
        fevt.collision.len(),
        w * h,
        "formats collision section is w*h"
    );
    for (ty, row) in grid.iter().enumerate() {
        for (tx, &cell) in row.iter().enumerate() {
            // collision_grid is `byte` (i8); the formats collision section is the same
            // raw bytes as `u8`. Compare the raw byte.
            assert_eq!(
                cell as u8,
                fevt.collision[ty * w + tx],
                "collision byte mismatch at tile ({tx},{ty})"
            );
        }
    }

    // Spot the specific tiles the walk-block scenarios below rely on (from the grid):
    // the map border is the all-blocked ring; (22,4) is the hero's walkable start.
    assert!(grid[0][0] < 0, "(0,0) is the blocked map border");
    assert!(grid[4][22] >= 0, "(22,4) the hero start is walkable");
    assert!(
        grid[4][25] < 0,
        "(25,4) is a blocked wall to the hero's right"
    );
    assert!(grid[4][23] >= 0, "(23,4) is an open tile");
}

/// Part (b), the crash-#1 closer: the ported block-verdict (`canStep`) is correct AND
/// panic-safe at the map boundary the crash fires at. Toward an in-map wall and toward
/// the all-blocked map BORDER, `canStep`/`isWalkable` report "blocked" without ever
/// indexing off the grid — so once the walk consults it the hero halts one tile short
/// of the edge, and the off-the-edge `Battler.move` panic is unreachable.
#[test]
fn hero_is_blocked_at_walls_and_map_border_panic_safe() {
    // --- An in-map wall: put the hero at (24,4); (25,4) is blocked, (23,4) is open. ---
    let mut g = drive_to_world();
    let hero = hero_id(&g);
    game_state::set_hero_tile(&mut g, 24, 4);
    assert_eq!(
        (g.entity_arena[hero].tile_x, g.entity_arena[hero].tile_y),
        (24, 4),
        "hero placed at (24,4)"
    );
    assert!(
        !game_map::can_step(&g, hero, DIR_RIGHT),
        "canStep RIGHT is FALSE — the tile ahead (25,4) is a blocked wall"
    );
    assert!(
        game_map::can_step(&g, hero, DIR_LEFT),
        "canStep LEFT is TRUE — the tile ahead (23,4) is walkable (the control)"
    );
    // A known open, unoccupied tile is walkable; the wall tile is not.
    assert!(
        game_map::is_walkable(&g, 23, 4),
        "(23,4) walkable and unoccupied"
    );
    assert!(!game_map::is_walkable(&g, 25, 4), "(25,4) blocked wall");

    // --- The map BORDER (the exact crash-#1 edge): a FRESH world, hero at (1,10), one
    //     tile inside the all-blocked left border. Walking left would step onto (0,10)
    //     then off the map — where Battler.move's `pixelX > 0` assert / the occupancy
    //     index used to fire. The block verdict must catch it one tile short. ---
    let mut g = drive_to_world();
    let hero = hero_id(&g);
    game_state::set_hero_tile(&mut g, 1, 10);
    assert!(
        !game_map::can_step(&g, hero, DIR_LEFT),
        "canStep LEFT is FALSE at the border tile (1,10) — (0,10) is the blocked edge, \
         so a walk consulting canStep halts here and never runs off the map (crash #1)"
    );
    assert!(
        game_map::can_step(&g, hero, DIR_RIGHT),
        "canStep RIGHT is TRUE at (1,10) — (2,10) is open (the control)"
    );

    // isWalkable is PANIC-SAFE at and past the boundary: off-map coordinates return
    // false via the `i >= 0 && i2 >= 0 && i < w && i2 < h` short-circuit, never indexing
    // collisionGrid/occupancy. (These calls must not panic.)
    assert!(
        !game_map::is_walkable(&g, -1, 10),
        "off-map (-1,10) -> false, no panic"
    );
    assert!(!game_map::is_walkable(&g, 0, 10), "border (0,10) blocked");
    assert!(
        !game_map::is_walkable(&g, map_w(&g), 10),
        "off-map (widthTiles,10) -> false, no panic"
    );
    assert!(
        !game_map::is_walkable(&g, 1, -1),
        "off-map (1,-1) -> false, no panic"
    );
    assert!(
        !game_map::is_walkable(&g, 1, map_h(&g)),
        "off-map (1,heightTiles) -> false, no panic"
    );
}

/// Part (c), the proven contrast + the reachable-state no-panic check. Walked in an OPEN
/// direction the hero physically advances (the walkable control), and walking the
/// in-bounds grid — even INTO a would-be-blocked tile — completes without panicking
/// (the collision load did not regress the walk; the block verdict flags the tile).
#[test]
fn open_direction_advances_and_walking_into_a_blocked_tile_does_not_panic() {
    // Walkable control: from (24,4) walk LEFT through open tiles → pixelX advances left.
    let mut g = drive_to_world();
    let hero = hero_id(&g);
    game_state::set_hero_tile(&mut g, 24, 4);
    assert!(
        game_map::can_step(&g, hero, DIR_LEFT),
        "LEFT is open at (24,4)"
    );
    let x_before = hero_pixel_x(&g);
    game_screen::key_pressed(&mut g, KEY_NUM4_LEFT);
    for _ in 0..3 {
        game_loop::run_one_frame(&mut g);
    }
    let x_after = hero_pixel_x(&g);
    assert!(
        x_after < x_before,
        "walked an OPEN direction: pixelX advanced left {x_before} -> {x_after}"
    );

    // Reachable-state: from (24,4) the tile ahead RIGHT (25,4) is a blocked wall
    // (canStep FALSE). Driving the real walk into it must (a) NOT panic — this is the
    // crash-#1 path: without collision the hero would overstep and trip Battler.move's
    // `Debug.assertTrue` bounds — and (b) HALT the hero at the wall, since
    // `Battler.tryStepForward` now consults the parsed collision (state → 1, no step).
    let mut g2 = drive_to_world();
    let hero2 = hero_id(&g2);
    game_state::set_hero_tile(&mut g2, 24, 4);
    assert!(
        !game_map::can_step(&g2, hero2, DIR_RIGHT),
        "the tile the hero is about to walk into (25,4) is flagged blocked"
    );
    let x0 = hero_pixel_x(&g2);
    game_screen::key_pressed(&mut g2, KEY_NUM6_RIGHT);
    for _ in 0..3 {
        // The in-bounds walk into the blocked tile completes with no `ASSERT FAILED` /
        // index panic — crash-#1 is closed.
        game_loop::run_one_frame(&mut g2);
    }
    let x1 = hero_pixel_x(&g2);
    assert_eq!(
        x1, x0,
        "the hero HALTED at the blocked wall (25,4) — no advance, no panic {x0} -> {x1}"
    );
}

/// `GameState.map.widthTiles` as the off-map x probe.
fn map_w(g: &Game) -> i32 {
    g.game_state.map.as_ref().expect("map").width_tiles
}
/// `GameState.map.heightTiles` as the off-map y probe.
fn map_h(g: &Game) -> i32 {
    g.game_state.map.as_ref().expect("map").height_tiles
}
