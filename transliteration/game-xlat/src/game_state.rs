//! Transliterated from `java/src/main/java/defpackage/GameState.java`
//! (original `n.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Global session state — a static-only class, never instantiated. This
//! increment ports only its `<clinit>` (the static-field initialization) so the
//! `<clinit>` machinery is in place; the session methods (`startNewMap`,
//! `processStateRequest`, save/load, camera/hero transitions, …) reach many
//! not-yet-ported classes (Hero, GameMap, AssetLoader, RmsFile, …) and are
//! DEFERRED.
//!
//! Opcode shape (R8, `_reference/numeric_shapes.json`): `n.<clinit>:()V => []`
//! (pure array/String construction — no arithmetic).

use crate::asset_cache;
use crate::asset_loader;
use crate::battler;
use crate::entity::{self, EntityId};
use crate::game::Game;
use crate::game_loop;
use crate::game_map::{self, GameMapState};
use crate::game_screen;
use crate::hero;
use crate::main_menu;
use j2me_jvm::{ishl, java_div, java_rem};

/// Java `n` / `GameState` state. Every field is `static` (see
/// `java/reconstruction/ownership.tsv`); struct order preserves the reviewed
/// declaration order. The reference-typed statics `map` and `hero` now carry real
/// state: the active [`GameMapState`] and the hero's [`EntityId`] handle into the
/// shared [`crate::entity::EntityArena`]; both are `None` (Java `null`) until New
/// Game — DEFERRED — constructs them.
#[derive(Debug)]
pub struct GameStateData {
    /// `public static GameMap map;` — the active map (`None` == null).
    pub map: Option<GameMapState>,
    /// `public static int camTargetX;`
    pub cam_target_x: i32,
    /// `public static int camTargetY;`
    pub cam_target_y: i32,
    /// `public static int camX;`
    pub cam_x: i32,
    /// `public static int camY;`
    pub cam_y: i32,
    /// `private static Hero hero;` — the player character's arena handle
    /// (`None` == null).
    pub hero: Option<EntityId>,
    /// `public static byte classId;` — selected class id (6..8).
    pub class_id: i8,
    /// `public static byte arg0;`
    pub arg0: i8,
    /// `public static byte arg1;`
    pub arg1: i8,
    /// `public static byte arg2;`
    pub arg2: i8,
    /// `public static byte storyMapId;`
    pub story_map_id: i8,
    /// `public static byte clearCount;`
    pub clear_count: i8,
    /// `private static byte[] clearBonusTable = {60, 30, 10};`
    pub clear_bonus_table: Vec<i8>,
    /// `private static byte[] saveKey = {5, 11, 8, 81, 3, 20};`
    pub save_key: Vec<i8>,
    /// `private static final byte[] classStartTable = {0,22,4, 60,5,36, 77,10,18};`
    pub class_start_table: Vec<i8>,
    /// `public static final String[] saveSlots = {"/k", "/s", "/w"};`
    pub save_slots: Vec<String>,
    /// `public static int screen = 0;`
    pub screen: i32,
    /// `public static byte nextState = 0;`
    pub next_state: i8,
    /// `private static byte pendingHeroState = 0;`
    pub pending_hero_state: i8,
    /// `private static byte pendingHeroFacing = 0;`
    pub pending_hero_facing: i8,
    /// `private static byte[] switches = new byte[128];`
    pub switches: Vec<i8>,
    /// `private static byte[] flags = new byte[128];`
    pub flags: Vec<i8>,
    /// `public static final boolean[][] classStartFlags = {…};` — per-class
    /// initial flag layout (three `boolean[15]` rows for classes 6..8).
    pub class_start_flags: Vec<Vec<bool>>,
}

/// The body of `n.<clinit>` (`n.<clinit>:()V => []`): the static-field
/// initializers in JVM order. Shared by [`GameStateData::default`] (eager, at
/// class-load) and [`class_init`] (the lazy trigger).
fn clinit_apply(s: &mut GameStateData) {
    // private static byte[] clearBonusTable = {60, 30, 10};
    s.clear_bonus_table = vec![60, 30, 10];
    // private static byte[] saveKey = {5, 11, 8, 81, 3, 20};
    s.save_key = vec![5, 11, 8, 81, 3, 20];
    // private static final byte[] classStartTable = {0,22,4, 60,5,36, 77,10,18};
    s.class_start_table = vec![0, 22, 4, 60, 5, 36, 77, 10, 18];
    // public static final String[] saveSlots = {"/k", "/s", "/w"};
    s.save_slots = vec!["/k".to_string(), "/s".to_string(), "/w".to_string()];
    // public static int screen = 0;
    s.screen = 0;
    // public static byte nextState = 0;
    s.next_state = 0;
    // private static byte pendingHeroState = 0;
    s.pending_hero_state = 0;
    // private static byte pendingHeroFacing = 0;
    s.pending_hero_facing = 0;
    // private static byte[] switches = new byte[128];
    s.switches = vec![0; 128];
    // private static byte[] flags = new byte[128];
    s.flags = vec![0; 128];
    // public static final boolean[][] classStartFlags = {{…},{…},{…}};
    s.class_start_flags = vec![
        vec![
            true, true, true, true, true, true, false, false, false, false, false, false, false,
            false, false,
        ],
        vec![
            true, false, true, false, false, false, false, true, true, true, true, false, false,
            true, true,
        ],
        vec![
            true, true, true, true, false, false, false, false, false, false, false, false, false,
            false, false,
        ],
    ];
}

impl Default for GameStateData {
    fn default() -> Self {
        // Uninitialized statics at their JVM defaults (0 / false / null); the
        // `<clinit>` initializers run via `clinit_apply`.
        let mut s = GameStateData {
            map: None,
            cam_target_x: 0,
            cam_target_y: 0,
            cam_x: 0,
            cam_y: 0,
            hero: None,
            class_id: 0,
            arg0: 0,
            arg1: 0,
            arg2: 0,
            story_map_id: 0,
            clear_count: 0,
            clear_bonus_table: Vec::new(),
            save_key: Vec::new(),
            class_start_table: Vec::new(),
            save_slots: Vec::new(),
            screen: 0,
            next_state: 0,
            pending_hero_state: 0,
            pending_hero_facing: 0,
            switches: Vec::new(),
            flags: Vec::new(),
            class_start_flags: Vec::new(),
        };
        clinit_apply(&mut s);
        s
    }
}

/// `n.<clinit>` at its JVM trigger point — the first active use of `GameState`.
/// The boot-entry path (`startApp` → `create` → `start`) never touches
/// `GameState`, so this is NOT reached during boot; it fires later, on the first
/// menu/world use (deferred). Idempotent (guarded by
/// [`Game::game_state_class_initialized`]).
pub fn class_init(g: &mut Game) {
    if g.game_state_class_initialized {
        return;
    }
    g.game_state_class_initialized = true;
    clinit_apply(&mut g.game_state);
}

/// `public static final void setScreen(int screenId)`
/// (`n.setScreen` — `screen = screenId`).
pub fn set_screen(g: &mut Game, screen_id: i32) {
    // screen = screenId;
    g.game_state.screen = screen_id;
}

/// `public static final synchronized void requestState(byte state, byte a0, byte a1)`
/// (`n.a:(BBB)V => []`): queues a two-argument state request (`arg2 = 0`). This is
/// the overload `buildLoadMenu` uses (`requestState((byte)2,(byte)9,(byte)3)`); the
/// other three overloads are DEFERRED (not reached on the boot→menu route).
/// `synchronized` is a no-op in the single-threaded transliteration.
pub fn request_state_a0_a1(g: &mut Game, state: i8, a0: i8, a1: i8) {
    // arg0 = a0; arg1 = a1; arg2 = (byte) 0; nextState = state;
    g.game_state.arg0 = a0;
    g.game_state.arg1 = a1;
    g.game_state.arg2 = 0;
    g.game_state.next_state = state;
}

/// `public static final synchronized void requestState(byte state, byte a0, byte a1, byte a2)`
/// (`n.a:(BBBB)V => []`): the three-argument overload (used by `requestMapWarp`).
pub fn request_state_a0_a1_a2(g: &mut Game, state: i8, a0: i8, a1: i8, a2: i8) {
    // arg0 = a0; arg1 = a1; arg2 = a2; nextState = state;
    g.game_state.arg0 = a0;
    g.game_state.arg1 = a1;
    g.game_state.arg2 = a2;
    g.game_state.next_state = state;
}

/// `public static final synchronized void requestState(byte state, byte a0)`
/// (`n.a:(BB)V => []`): the one-argument overload (`arg1 = arg2 = 0`). Used by New
/// Game (`requestState(21, resume ? 1 : 0)`) and the map warp (`requestState(15, arg0)`).
pub fn request_state_a0(g: &mut Game, state: i8, a0: i8) {
    // arg0 = a0; arg1 = (byte) 0; arg2 = (byte) 0; nextState = state;
    g.game_state.arg0 = a0;
    g.game_state.arg1 = 0;
    g.game_state.arg2 = 0;
    g.game_state.next_state = state;
}

/// `public static final void clearRequest()` (`n`): clears any queued state request.
pub fn clear_request(g: &mut Game) {
    // nextState = 0; arg0 = 0; arg1 = 0; arg2 = 0;
    g.game_state.next_state = 0;
    g.game_state.arg0 = 0;
    g.game_state.arg1 = 0;
    g.game_state.arg2 = 0;
}

/// `public static final void requestMapWarp(byte mapId, byte a0, byte a1, byte a2)`
/// (`n`): requests a map warp (state 1) and records the destination map id.
pub fn request_map_warp(g: &mut Game, map_id: i8, a0: i8, a1: i8, a2: i8) {
    // System.gc();  — no-op.
    // requestState((byte) 1, a0, a1, a2);
    request_state_a0_a1_a2(g, 1, a0, a1, a2);
    // AudioManager.stopBgm1(); AudioManager.stopBgm();  — DEFERRED (audio not on path).
    // storyMapId = mapId;
    g.game_state.story_map_id = map_id;
}

/// `public static final void processStateRequest()` (`n.processStateRequest`):
/// dispatches the queued `nextState`. This slice ports `case 1` (kick the map
/// loader), `case 2` (set-screen + FPS), `case 15` (warp the map to the world) and
/// `case 21` (New Game / start-new-map); the shop/character/game-over/ending
/// transitions (cases 11, 12, 13, 14, 16) reach unported screens and are DEFERRED.
/// State `0` (no request) falls through to a no-op.
pub fn process_state_request(g: &mut Game) {
    // if (nextState == 0) {}   — empty statement (decompiler noise).
    // byte state = nextState; nextState = (byte) 0;
    let state = g.game_state.next_state;
    g.game_state.next_state = 0;
    // switch (state)  — the transition dispatch.
    match state {
        // case 1: setScreen(1); GameLoop.instance.setLoadingFps(); AssetLoader.loadMap();
        1 => {
            set_screen(g, 1);
            game_loop::set_loading_fps(g);
            asset_loader::load_map(g);
        }
        // case 2: setScreen((int) arg0); <fps by arg1>
        2 => {
            // setScreen((int) arg0);
            set_screen(g, g.game_state.arg0 as i32);
            // if (arg1 == 0) setFps((int) arg2); else if 1 applyDifficultyFps();
            //   else if 2 setLoadingFps(); else if 3 setFastFps();
            let arg1 = g.game_state.arg1;
            if arg1 == 0 {
                let arg2 = g.game_state.arg2 as i32;
                game_loop::set_fps(g, arg2);
            } else if arg1 == 1 {
                game_loop::apply_difficulty_fps(g);
            } else if arg1 == 2 {
                game_loop::set_loading_fps(g);
            } else if arg1 == 3 {
                game_loop::set_fast_fps(g);
            }
        }
        // case 15: warpMap();
        15 => {
            warp_map(g);
        }
        // case 21: New Game / continue / new character, then kick the resource load.
        21 => {
            // if (arg0 == 1) continueGame(); else if (arg0 == 0) startNewMap();
            //   else if (arg0 == 2) { startNewCharacter(); ...; loadMainMenu(); stopBgm(); }
            if g.game_state.arg0 == 1 {
                // continueGame();  — DEFERRED (save/load path; not the New Game drive).
            } else if g.game_state.arg0 == 0 {
                start_new_map(g);
            } else if g.game_state.arg0 == 2 {
                // startNewCharacter()/closeMenu/loadMainMenu/stopBgm  — DEFERRED (NG+ path).
            }
            // setScreen(1); GameLoop.instance.setLoadingFps(); AssetLoader.loadResources();
            set_screen(g, 1);
            game_loop::set_loading_fps(g);
            asset_loader::load_resources(g);
        }
        // (DEFERRED: cases 11,12,13,14,16 — shop-refine-blacksmith / character menu /
        // game-over / ending; not reached by the New Game → world drive.)
        _ => {}
    }
}

/// `readSaveSlot(byte slotId)` — reads record store `saveSlots[slotId - 6]`
/// (`"/k"`/`"/s"`/`"/w"`) via `RmsFile`. DEFERRED: `RmsFile` is not modelled in
/// this increment (a sibling lane owns it), and a fresh install has **no** record
/// store, so every slot is absent — returns `None`.
fn read_save_slot(_g: &mut Game, _slot_id: i8) -> Option<Vec<i8>> {
    None
}

/// `public static final void buildLoadMenu()` (`n.buildLoadMenu`): builds the
/// main menu from the saved slots and requests the menu state.
///
/// ANTI-BOG: on a fresh install every `readSaveSlot(6..8)` is null, so `slotCount`
/// is 0 and the save-blob decrypt loop (`ByteUtil.readU16` / `SaveCipher.decrypt`
/// / `unpackProgress`) never executes — that whole branch is DEFERRED. The result
/// is the six-item, no-save main menu with the cursor on New Game.
pub fn build_load_menu(g: &mut Game) {
    // First active use of GameState on the menu path -> n.<clinit>.
    class_init(g);
    // int slotCount = 0; Object[] slotData = new Object[3];
    let mut slot_count: i32 = 0;
    let mut slot_data: Vec<Option<Vec<i8>>> = vec![None, None, None];
    // for (byte slot = 6; slot <= 8; slot++) { slotData[slot-6] = readSaveSlot(slot); if (!= null) slotCount++; }
    let mut slot: i8 = 6;
    loop {
        let slot_id = slot;
        // if (slotId > 8) break;
        if (slot_id as i32) > 8 {
            break;
        }
        // slotData[slotId - 6] = readSaveSlot(slotId);
        let idx = (slot_id as i32).wrapping_sub(6) as usize;
        slot_data[idx] = read_save_slot(g, slot_id);
        // if (slotData[slotId - 6] != null) slotCount++;
        if slot_data[idx].is_some() {
            slot_count = slot_count.wrapping_add(1);
        }
        // slot = (byte) (slotId + 1);
        slot = (slot_id as i32).wrapping_add(1) as i8;
    }
    // byte[] menuData = new byte[slotCount * 4];
    let menu_data: Vec<i8> = vec![0i8; (slot_count.wrapping_mul(4)) as usize];
    // The second loop decrypts each non-null slot into menuData; on a fresh install
    // every slot is null, so it does nothing. DEFERRED (save-blob decrypt path).
    // MainMenu.create(slotCount > 0, menuData);
    main_menu::create(g, slot_count > 0, menu_data);
    // ((Menu) MainMenu.instance()).cursorIndex = slotCount > 0 ? (byte) 1 : (byte) 0;
    g.main_menu.base.cursor_index = if slot_count > 0 { 1 } else { 0 };
    // requestState((byte) 2, (byte) 9, (byte) 3);
    request_state_a0_a1(g, 2, 9, 3);
}

/// `private static final void clearSwitches()` (`n`): zeroes the story switch bitset.
pub fn clear_switches(g: &mut Game) {
    // for (int bit = 0; bit < 128; bit++) switches[bit] = 0;
    for bit in 0..128usize {
        g.game_state.switches[bit] = 0;
    }
}

/// `private static final void clearFlags()` (`n`): zeroes the story flag bitset.
pub fn clear_flags(g: &mut Game) {
    // for (int bit = 0; bit < 128; bit++) flags[bit] = 0;
    for bit in 0..128usize {
        g.game_state.flags[bit] = 0;
    }
}

/// `public static final void setSwitch(int bit)` (`n`): sets story switch `bit`.
pub fn set_switch(g: &mut Game, bit: i32) {
    // switches[bit / 8] = (byte) (switches[bit / 8] | (1 << (bit % 8)));
    let idx = java_div(bit, 8).expect("bit / 8") as usize;
    let mask = ishl(1, java_rem(bit, 8).expect("bit % 8"));
    g.game_state.switches[idx] = ((g.game_state.switches[idx] as i32) | mask) as i8;
}

/// `public static final void startNewMap()` (`n`): starts a fresh map for the
/// current class — resets progress and reads the class start triple.
///
/// The quick-item RMS read (`new RmsFile("/o", 1)` + `SaveCipher.decrypt`) is
/// wrapped in `try { } catch (Exception) { }`; a fresh install has no `/o` record,
/// so it throws and is swallowed — modelled as a DEFERRED no-op.
pub fn start_new_map(g: &mut Game) {
    // clearCount = (byte) 0;
    g.game_state.clear_count = 0;
    // hero.initClass(classId);
    let id = g
        .game_state
        .hero
        .expect("GameState.hero null in startNewMap");
    let class_id = g.game_state.class_id;
    hero::init_class(g, id, class_id);
    // try { RmsFile("/o", 1) ... hero.quickItems.deserialize(...) } catch { }  — DEFERRED.
    // clearSwitches(); clearFlags(); setSwitch(0);
    clear_switches(g);
    clear_flags(g);
    set_switch(g, 0);
    // storyMapId = classStartTable[(classId - 6) * 3];
    let base = (class_id as i32).wrapping_sub(6).wrapping_mul(3);
    g.game_state.story_map_id = g.game_state.class_start_table[base as usize];
    // arg0 = classStartTable[((classId - 6) * 3) + 1];
    g.game_state.arg0 = g.game_state.class_start_table[base.wrapping_add(1) as usize];
    // arg1 = classStartTable[((classId - 6) * 3) + 2];
    g.game_state.arg1 = g.game_state.class_start_table[base.wrapping_add(2) as usize];
}

/// `public static final void setHeroTile(int tileX, int tileY)` (`n`): teleports the
/// hero to a tile and (re)registers its occupancy. `Hero.setPixelPos` overrides the
/// base to also `syncTile()`, so `tileX`/`tileY` and the off-grid flags are rederived.
pub fn set_hero_tile(g: &mut Game, tile_x: i32, tile_y: i32) {
    let id = g
        .game_state
        .hero
        .expect("GameState.hero null in setHeroTile");
    // hero.setPixelPos((short) (tileX * 16), (short) (tileY * 16));  (Hero override → syncTile)
    {
        let node = &mut g.entity_arena[id];
        entity::set_pixel_pos(
            node,
            tile_x.wrapping_mul(16) as i16,
            tile_y.wrapping_mul(16) as i16,
        );
        entity::sync_tile(node);
    }
    // hero.setOccupancy();
    battler::set_occupancy(g, id);
}

/// `public static final void centerCamera()` (`n`): recomputes the camera target so
/// the hero is centered on screen.
pub fn center_camera(g: &mut Game) {
    let id = g
        .game_state
        .hero
        .expect("GameState.hero null in centerCamera");
    let pixel_x = g.entity_arena[id].pixel_x as i32;
    let pixel_y = g.entity_arena[id].pixel_y as i32;
    // camTargetX = GameScreen.centerX - ((Entity) hero).pixelX;
    g.game_state.cam_target_x = g.game_screen.center_x.wrapping_sub(pixel_x);
    // camTargetY = GameScreen.centerY - ((Entity) hero).pixelY;
    g.game_state.cam_target_y = g.game_screen.center_y.wrapping_sub(pixel_y);
}

/// `public static final void warpMap()` (`n`): places the hero on the freshly-loaded
/// map, centers + snaps the camera, and requests the world screen (state 2).
///
/// `hero.addGuardianToMap()` (adds the active guardian entity) is DEFERRED — the
/// guardian summon is not driven in this slice, so `activeGuardian` is null.
pub fn warp_map(g: &mut Game) {
    let id = g.game_state.hero.expect("GameState.hero null in warpMap");
    // map.addEntity(hero);
    game_map::add_entity(g, id);
    // hero.addGuardianToMap();  — DEFERRED (guardian summon; activeGuardian null).
    // hero.init();
    hero::init(g, id);
    // hero.setFacing((byte) (arg0 + 1));
    let facing = (g.game_state.arg0 as i32).wrapping_add(1) as i8;
    {
        let battler_data = &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler;
        battler::set_facing(battler_data, facing);
    }
    // centerCamera();
    center_camera(g);
    // camX = camTargetX; camY = camTargetY;
    g.game_state.cam_x = g.game_state.cam_target_x;
    g.game_state.cam_y = g.game_state.cam_target_y;
    // clearRequest();
    clear_request(g);
    // storyMapId = (byte) -1;
    g.game_state.story_map_id = -1;
    // hero.setState((byte) 1);
    {
        let battler_data = &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler;
        battler::set_state(battler_data, 1);
    }
    // hero.resetCombo();
    hero::reset_combo(g, id);
    // GameLoop.gameScreen.markRedraw();
    game_screen::mark_redraw(g);
    // requestState((byte) 2, (byte) 2, (byte) 1);
    request_state_a0_a1(g, 2, 2, 1);
}

/// `public static final void newGame(boolean resume, byte newClassId, boolean[] traits)`
/// (`n.newGame`): creates a brand-new hero of class `newClassId` and requests class
/// setup (state 21). `traits` (the guardian/trait selection mask from
/// `StartTraitMenu`) drives the three `setState` calls when not resuming.
pub fn new_game(g: &mut Game, resume: bool, new_class_id: i8, traits: &[bool]) {
    // (GameState already class-inited by this point; class_init is idempotent.)
    class_init(g);
    // MainMenu.dispose();
    main_menu::dispose(g);
    // AssetCache.unloadMainMenuAssets();
    asset_cache::unload_main_menu_assets(g);
    // classId = newClassId;
    g.game_state.class_id = new_class_id;
    // hero = new Hero((short) 0, (short) 0, (byte) 8, (byte) 8, newClassId);
    let id = {
        let Game {
            entity_arena,
            clock,
            ..
        } = &mut *g;
        hero::new_hero(entity_arena, clock, 0, 0, 8, 8, new_class_id)
    };
    // GameState.hero = <the new Hero's arena handle>.
    g.game_state.hero = Some(id);
    // if (!resume) { if (traits[0]) hero.setState(0); if (traits[1]) hero.setState(1);
    //   if (traits[2]) hero.setState(2); }
    if !resume {
        if traits[0] {
            let b = &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler;
            battler::set_state(b, 0);
        }
        if traits[1] {
            let b = &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler;
            battler::set_state(b, 1);
        }
        if traits[2] {
            let b = &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler;
            battler::set_state(b, 2);
        }
    }
    // setScreen(0);
    set_screen(g, 0);
    // requestState((byte) 21, resume ? (byte) 1 : (byte) 0);
    request_state_a0(g, 21, if resume { 1 } else { 0 });
}
