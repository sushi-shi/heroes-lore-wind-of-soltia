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

use crate::entity::EntityId;
use crate::game::Game;
use crate::game_loop;
use crate::game_map::GameMapState;
use crate::main_menu;

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

/// `public static final void processStateRequest()` (`n.processStateRequest`):
/// dispatches the queued `nextState`. ANTI-BOG: only `case 2` (set-screen + FPS) —
/// the transition `buildLoadMenu` requests for the main menu — is ported; the map /
/// menu / ending transitions (cases 1, 11, 12, 13, 14, 15, 16, 21) reach unported
/// screens and are DEFERRED. State `0` (no request) falls through to a no-op.
pub fn process_state_request(g: &mut Game) {
    // if (nextState == 0) {}   — empty statement (decompiler noise).
    // byte state = nextState; nextState = (byte) 0;
    let state = g.game_state.next_state;
    g.game_state.next_state = 0;
    // switch (state)  — an 8-case dispatch; only `case 2` (set-screen + FPS, the
    // main-menu transition) is ported, the rest DEFERRED (hence single_match).
    #[allow(clippy::single_match)]
    match state {
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
        // (DEFERRED: cases 1,11,12,13,14,15,16,21 — map warp / shop-refine-blacksmith /
        // character menu / game-over / ending; not reached by the boot→main-menu route.)
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
