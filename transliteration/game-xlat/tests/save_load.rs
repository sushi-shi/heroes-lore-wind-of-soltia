//! GameState save/load round-trip oracle: the RMS save format end to end.
//!
//! From a New Game world state (boot → title → menu → New Game → walkable world),
//! `GameState.saveGame` writes the encrypted hero/bag/progress/position blob to the
//! class's RMS slot (plus the quick-item bar to `/o`), and `GameState.loadGame` /
//! `continueGame` read it back. This gate drives the real orchestration over the
//! ported [`rms_file`] (`RmsFile`) + [`save_cipher`] (`SaveCipher`) leaf classes and
//! asserts the persisted state round-trips.
//!
//! SCOPE — the hero **stat** blob (`Hero.save` / `Hero.load`, class `ao`) is not yet
//! ported (a sibling lane owns `hero.rs`), so `saveGame` writes that slice from an
//! empty placeholder and `loadGame` reads-but-does-not-apply it (see
//! `game_state.rs`). Everything else the save format carries — the **progress**
//! bitsets + clear count (`packProgress`/`unpackProgress`), the **bag** and
//! **quick-item** contents (`ItemBag.serialize`/`deserialize`), and the **map/
//! position** triple `{map.mapType, tileX, tileY}` — round-trips in full and is what
//! this test asserts. When `Hero.save`/`load` land, extend the assertions to the
//! hero stats.
//!
//! Teeth (GATES.md R3): the state is deliberately CLOBBERED between save and load, so
//! the restored values can only match the pre-save snapshot if the load truly read
//! the slot; a one-bit perturbation of the snapshot is proven to differ.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, entity::EntityId, font_manager, game_loop, game_midlet, game_state,
    title_screen, Game,
};

const GAME_RNG_SEED: i64 = 305_419_896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
/// SOFT1 (any key) leaves the state-1 title.
const KEY_SOFT1: i32 = -6;
/// Warrior — the first selectable start class (RMS slot `saveSlots[0]` == "/k").
const CLASS_WARRIOR: i8 = 6;

fn load_resources(g: &mut Game) {
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
}

/// Drives boot → title → any-key → settled main menu (screen 9), then New Game
/// DIRECTLY, pumping loading → world frames until the world screen (2) renders.
/// Mirrors `hero_moves::drive_to_world`, replicated here from the public API.
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

/// A distinctive story-progress fingerprint written into the switch/flag bitsets +
/// clear count, so a successful restore is unmistakably distinct from both the
/// clobber sentinels and a New-Game reset (`startNewMap` would zero the flags and set
/// only switch 0).
fn stamp_distinctive_progress(g: &mut Game) {
    for (i, b) in g.game_state.switches.iter_mut().enumerate() {
        *b = (i as i32).wrapping_mul(7).wrapping_add(3) as i8;
    }
    for (i, b) in g.game_state.flags.iter_mut().enumerate() {
        *b = (i as i32).wrapping_mul(5).wrapping_sub(9) as i8;
    }
    g.game_state.clear_count = 2;
}

fn hero_gold(g: &Game, id: EntityId) -> i32 {
    g.entity_arena[id].as_hero().expect("Hero node").bag.gold
}

fn set_hero_gold(g: &mut Game, id: EntityId, gold: i32) {
    g.entity_arena[id]
        .as_hero_mut()
        .expect("Hero node")
        .bag
        .gold = gold;
}

#[test]
fn save_then_load_round_trips_progress_bag_and_position() {
    let mut g = drive_to_world();
    let hero_id = g.game_state.hero.expect("hero materialised");
    assert_eq!(g.game_state.screen, 2, "world screen (2)");

    // --- Arrange: stamp a distinctive persisted state. ---
    stamp_distinctive_progress(&mut g);
    const GOLD: i32 = 0x0051_7A93; // 5_339_795 — a value no code path sets by chance.
    set_hero_gold(&mut g, hero_id, GOLD);

    // Snapshot everything the ported slices should restore.
    let saved_switches = g.game_state.switches.clone();
    let saved_flags = g.game_state.flags.clone();
    let saved_clear = g.game_state.clear_count;
    let saved_gold = hero_gold(&g, hero_id);
    let saved_map_type = g
        .game_state
        .map
        .as_ref()
        .expect("GameState.map set at the world screen")
        .map_type;
    let (saved_tx, saved_ty) = {
        let n = &g.entity_arena[hero_id];
        (n.tile_x, n.tile_y)
    };

    // --- Act: save, then CLOBBER the in-memory state, then load it back. ---
    game_state::save_game(&mut g).expect("saveGame writes the class RMS slot + /o");

    for b in g.game_state.switches.iter_mut() {
        *b = 0;
    }
    for b in g.game_state.flags.iter_mut() {
        *b = 0;
    }
    g.game_state.clear_count = 99;
    g.game_state.story_map_id = 0;
    g.game_state.arg0 = 0;
    g.game_state.arg1 = 0;
    set_hero_gold(&mut g, hero_id, -1);

    game_state::load_game(&mut g).expect("loadGame reads back the slot");

    // --- Assert: the ported slices round-tripped exactly. ---
    assert_eq!(
        g.game_state.switches, saved_switches,
        "the 128-bit switch bitset round-tripped through packProgress/SaveCipher/RMS"
    );
    assert_eq!(
        g.game_state.flags, saved_flags,
        "the 128-bit flag bitset round-tripped"
    );
    assert_eq!(
        g.game_state.clear_count, saved_clear,
        "clearCount round-tripped"
    );
    assert_eq!(
        hero_gold(&g, hero_id),
        saved_gold,
        "the bag (gold) round-tripped through ItemBag.serialize/deserialize"
    );
    // posBytes == {map.mapType, tileX, tileY} → loadGame restores {storyMapId, arg0, arg1}.
    assert_eq!(
        g.game_state.story_map_id, saved_map_type,
        "position slice restored map.mapType into storyMapId"
    );
    assert_eq!(
        g.game_state.arg0, saved_tx,
        "position slice restored the hero's tileX into arg0"
    );
    assert_eq!(
        g.game_state.arg1, saved_ty,
        "position slice restored the hero's tileY into arg1"
    );

    // --- Teeth: the restore is real (distinct from the clobber sentinels) and the
    //     assertions would catch a one-bit divergence. ---
    assert_ne!(
        g.game_state.clear_count, 99,
        "load left the clobbered clearCount — nothing was read"
    );
    assert_ne!(
        hero_gold(&g, hero_id),
        -1,
        "load left the clobbered gold — the bag slice was not restored"
    );
    let mut perturbed = saved_switches.clone();
    perturbed[0] = perturbed[0].wrapping_add(1);
    assert_ne!(
        g.game_state.switches, perturbed,
        "a one-bit perturbation of the snapshot must not read as a match — the test is blind"
    );
}

#[test]
fn continue_game_reloads_the_saved_slot() {
    // The public "Continue" entry point (processStateRequest case 21 arg0==1):
    // continueGame() loads the slot rather than falling back to startNewMap.
    let mut g = drive_to_world();
    let hero_id = g.game_state.hero.expect("hero materialised");

    stamp_distinctive_progress(&mut g);
    const GOLD: i32 = 0x0002_ABCD;
    set_hero_gold(&mut g, hero_id, GOLD);
    let saved_switches = g.game_state.switches.clone();
    let saved_flags = g.game_state.flags.clone();
    let saved_clear = g.game_state.clear_count;

    game_state::save_game(&mut g).expect("saveGame");

    // Clobber, then take the public Continue path.
    for b in g.game_state.switches.iter_mut() {
        *b = 0;
    }
    for b in g.game_state.flags.iter_mut() {
        *b = 0;
    }
    g.game_state.clear_count = 42;
    set_hero_gold(&mut g, hero_id, 0);

    game_state::continue_game(&mut g);

    assert_eq!(
        g.game_state.switches, saved_switches,
        "continueGame restored the switch bitset (loaded the slot, no startNewMap fallback)"
    );
    assert_eq!(
        g.game_state.flags, saved_flags,
        "continueGame restored flags"
    );
    assert_eq!(
        g.game_state.clear_count, saved_clear,
        "continueGame restored clearCount"
    );
    assert_eq!(
        hero_gold(&g, hero_id),
        GOLD,
        "continueGame restored the bag gold"
    );

    // Teeth: if continueGame had fallen back to startNewMap, clearCount would be 0
    // (startNewMap sets it), the flags would be all-zero, and only switch 0 would be
    // set — none of which match the distinctive snapshot above.
    assert_eq!(
        saved_clear, 2,
        "the snapshot's clearCount is the distinctive 2"
    );
    assert_ne!(
        g.game_state.clear_count, 0,
        "clearCount is not the startNewMap reset value — the load path ran, not the fallback"
    );
}
