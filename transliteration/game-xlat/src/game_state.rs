//! Transliterated from `java/src/main/java/defpackage/GameState.java`
//! (original `n.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Global session state — a static-only class, never instantiated.
//!
//! Ported: the `<clinit>`, `setScreen`, all four `requestState` overloads,
//! `clearRequest`, `requestMapWarp`, `processStateRequest` (cases 1/2/12/15/16/21
//! route to ported destinations; see below), `startNewMap`, `continueGame`,
//! `startNewCharacter`, `newGame`, `buildLoadMenu`, the camera + hero-transition
//! helpers, the switch/flag bitset accessors (`clearSwitches`/`clearFlags`/
//! `setSwitch`/`isSwitch`/`isFlag`), the world sim step, and the **RMS save
//! format** (`saveGame`/`loadGame` over [`crate::rms_file`] + [`crate::save_cipher`],
//! plus `packProgress`/`unpackProgress`/`progressBonus`).
//!
//! Two deferrals remain in the save/session methods, each reaching a class a
//! sibling lane still owns:
//! - **`Hero.save` / `Hero.load`** (`ao`) are unported (only Hero's field layer +
//!   New Game setup have landed), so the hero stat blob is DEFERRED: `saveGame`
//!   writes an **empty** hero slice (a documented placeholder — the four
//!   length-prefixed slices' structure is preserved intact) and `loadGame` reads
//!   the slice (advancing the RMS cursor) but does not apply it. The
//!   bag/quick-item/progress/position slices round-trip in full (their
//!   serialisers are ported).
//! - **`processStateRequest`** cases 11/13/14 now open+close the ported ShopMenu
//!   (screen 6) and CharacterMenu (screen 5); only their Refine/Blacksmith
//!   (screens 7/8) sub-arms, case 12's inner Refine/Blacksmith close-switch, the
//!   case-14/case-21 quit `AssetLoader.loadMainMenu`, and `startNewCharacter`'s
//!   `GameLoop.saveOptions` remain DEFERRED (each marked at its site).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `n.<clinit>:()V => []`;
//! `progressBonus n.a:(B)B`, `isSwitch n.a:(I)Z` / `isFlag n.b:(I)Z`,
//! `packProgress n.a:()[B => []`, `unpackProgress n.a:([B)V => []`,
//! `saveGame n.o:()V`, `loadGame n.r:()V`, `startNewCharacter n.c:()V`,
//! `continueGame` a no-arithmetic `()V` — each transliterated verbatim below.

use crate::asset_cache;
use crate::asset_loader;
use crate::audio_manager;
use crate::battler;
use crate::character_menu;
use crate::directions;
use crate::entity::{self, EntityId};
use crate::game::Game;
use crate::game_loop;
use crate::game_map::{self, GameMapState};
use crate::game_screen;
use crate::hero;
use crate::item_bag;
use crate::main_menu;
use crate::rms_file;
use crate::save_cipher;
use crate::shop_menu;
use j2me_jvm::{ishl, ishr, java_div, java_rem, JavaError};

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

/// `public static final synchronized void requestState(byte state)`
/// (`n.a:(B)V => []`): the no-argument overload (`arg0 = arg1 = arg2 = 0`). Used by
/// `handlePlayKey` (the back/soft-key → character-menu request, case 13).
pub fn request_state(g: &mut Game, state: i8) {
    // arg0 = (byte) 0; arg1 = (byte) 0; arg2 = (byte) 0; nextState = state;
    g.game_state.arg0 = 0;
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
/// dispatches the queued `nextState`. Ported: `case 1` (kick the map loader),
/// `case 2` (set-screen + FPS), `case 11` (open the shop — `arg0 == 0` → screen 6 +
/// [`shop_menu::load_strings`]; the Refine/Blacksmith sub-arms stay DEFERRED),
/// `case 12` (return to the world; its inner Refine/Blacksmith close-switch is
/// DEFERRED), `case 13` (open the character menu — screen 5 +
/// [`character_menu::open`], with the full-version quit escape via
/// [`character_menu::open_system_quit`]), `case 14` (close the character menu via
/// [`character_menu::close_menu`]; the quit-to-main-menu `AssetLoader.loadMainMenu`
/// stays DEFERRED), `case 15` (warp the map to the world), `case 16` (game-over
/// fade + sfx) and `case 21` (New Game / continue / new character). State `0` (no
/// request) falls through to a no-op.
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
        // case 11: switch (arg0) { 0: open shop; 1: Refine; 2: Blacksmith }
        11 => {
            // switch (arg0)
            match g.game_state.arg0 {
                // case 0: setScreen(6); ShopMenu.instance().loadStrings();
                0 => {
                    // setScreen(6);
                    set_screen(g, 6);
                    // ShopMenu.instance().loadStrings();
                    shop_menu::instance(g);
                    shop_menu::load_strings(g);
                }
                // case 1: setScreen(7); RefineMenu.instance().open();
                1 => {
                    // DEFERRED: RefineMenu (bd) not yet ported (target screen 7 unported).
                }
                // case 2: setScreen(8); BlacksmithMenu.instance().open();
                2 => {
                    // DEFERRED: BlacksmithMenu (bc) not yet ported (target screen 8 unported).
                }
                _ => {}
            }
        }
        // case 12: setScreen(2); switch (arg0) { RefineMenu/BlacksmithMenu close }
        12 => {
            // setScreen(2);
            set_screen(g, 2);
            // switch (arg0) { case 1: RefineMenu.instance().closeRefine();
            //   case 2: BlacksmithMenu.instance().closeBlacksmith(); }
            //   — DEFERRED: RefineMenu (bd) / BlacksmithMenu (bc) not yet ported.
        }
        // case 13: setScreen(5); CharacterMenu.instance().open(); <full-version quit escape>
        13 => {
            // setScreen(5);
            set_screen(g, 5);
            // CharacterMenu.instance().open();
            character_menu::instance(g);
            character_menu::open(g);
            // if ((Debug.fullVersion && arg0 == 1) || (AppConfig.fullVersion && hero.level >= 8)) {
            //   CharacterMenu.instance().openSystemQuit(); break; }
            //   — the `||`/`&&` short-circuit is preserved: hero.level is read only when
            //   Debug's disjunct is false AND AppConfig.fullVersion holds.
            let debug_full = g.debug.full_version;
            let arg0 = g.game_state.arg0;
            let app_full = g.app_config.full_version;
            let system_quit = (debug_full && arg0 == 1)
                || (app_full && {
                    let id = g
                        .game_state
                        .hero
                        .expect("GameState.hero null in processStateRequest case 13");
                    let level = g.entity_arena[id].as_hero().expect("Hero node").level;
                    (level as i32) >= 8
                });
            if system_quit {
                // CharacterMenu.instance().openSystemQuit();
                character_menu::instance(g);
                character_menu::open_system_quit(g);
            }
        }
        // case 14: CharacterMenu close (arg0 != 1: apply + return to world; else quit to menu)
        14 => {
            // if (arg0 != 1) CharacterMenu.instance().closeMenu(true);
            if g.game_state.arg0 != 1 {
                character_menu::instance(g);
                character_menu::close_menu(g, true);
            } else {
                // else { CharacterMenu.instance().closeMenu(false); setScreen(1); AssetLoader.loadMainMenu(); }
                character_menu::instance(g);
                character_menu::close_menu(g, false);
                // setScreen(1);
                set_screen(g, 1);
                // AssetLoader.loadMainMenu();  — DEFERRED (AssetLoader.loadMainMenu unported).
            }
        }
        // case 15: warpMap();
        15 => {
            warp_map(g);
        }
        // case 16: setScreen(10); AudioManager.loadClip(12); playSfx(12,false); fxTimer=16;
        16 => {
            // setScreen(10);
            set_screen(g, 10);
            // AudioManager.loadClip((byte) 12);
            audio_manager::load_clip(g, 12);
            // AudioManager.playSfx((byte) 12, false);
            audio_manager::play_sfx(g, 12, false);
            // GameScreen.fxTimer = 16;
            g.game_screen.fx_timer = 16;
        }
        // case 21: New Game / continue / new character, then kick the resource load.
        21 => {
            // if (arg0 == 1) continueGame(); else if (arg0 == 0) startNewMap();
            //   else if (arg0 == 2) { startNewCharacter(); closeMenu(false); setScreen(1);
            //     loadMainMenu(); stopBgm(); }
            if g.game_state.arg0 == 1 {
                // continueGame();
                continue_game(g);
            } else if g.game_state.arg0 == 0 {
                start_new_map(g);
            } else if g.game_state.arg0 == 2 {
                // startNewCharacter();
                start_new_character(g);
                // CharacterMenu.instance().closeMenu(false);
                character_menu::instance(g);
                character_menu::close_menu(g, false);
                // setScreen(1);
                set_screen(g, 1);
                // AssetLoader.loadMainMenu();  — DEFERRED (AssetLoader.loadMainMenu unported).
                // AudioManager.stopBgm();
                audio_manager::stop_bgm(g);
            }
            // setScreen(1); GameLoop.instance.setLoadingFps(); AssetLoader.loadResources();
            set_screen(g, 1);
            game_loop::set_loading_fps(g);
            asset_loader::load_resources(g);
        }
        // (No further cases: state 0 is the no-request idle; the shop/character-menu
        // open+close transitions (11/13/14) are now ported above, with only their
        // Refine/Blacksmith (screens 7/8) sub-arms and the quit-path `loadMainMenu`
        // DEFERRED at their sites.)
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

/// `public static final boolean isSwitch(int bit)` (`n.a:(I)Z => [idiv,irem,ishr,iand]`):
/// returns whether story switch `bit` is set. The `byte` element sign-extends to
/// `int` (baload) before the `>>` (ishr); the trailing `& 1` isolates bit 0.
pub fn is_switch(g: &Game, bit: i32) -> bool {
    // return ((switches[bit / 8] >> (bit % 8)) & 1) == 1;
    let idx = java_div(bit, 8).expect("bit / 8") as usize;
    let sh = java_rem(bit, 8).expect("bit % 8");
    (ishr(g.game_state.switches[idx] as i32, sh) & 1) == 1
}

/// `public static final boolean isFlag(int bit)` (`n.b:(I)Z => [idiv,irem,ishr,iand]`):
/// returns whether story flag `bit` is set (same shape as [`is_switch`], over `flags`).
pub fn is_flag(g: &Game, bit: i32) -> bool {
    // return ((flags[bit / 8] >> (bit % 8)) & 1) == 1;
    let idx = java_div(bit, 8).expect("bit / 8") as usize;
    let sh = java_rem(bit, 8).expect("bit % 8");
    (ishr(g.game_state.flags[idx] as i32, sh) & 1) == 1
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

/// `public static final void continueGame()` (`n`, a no-arithmetic `()V`): resumes a
/// saved game, falling back to a fresh map when the load fails.
///
/// The original's `Throwable th = null; … th.printStackTrace();` in the catch is a
/// JADX artifact (a phantom null local; the shipped code logged the caught `e2`).
/// Per `docs/TRANSLITERATION.md` `printStackTrace` is a no-op, which sidesteps the
/// phantom — a failed [`load_game`] simply routes to [`start_new_map`].
pub fn continue_game(g: &mut Game) {
    // clearSwitches(); clearFlags();
    clear_switches(g);
    clear_flags(g);
    // Throwable th = null;  — JADX phantom local (see doc note).
    // setSwitch(0);
    set_switch(g, 0);
    // try { loadGame(); } catch (Exception e2) { th.printStackTrace(); startNewMap(); }
    if load_game(g).is_err() {
        start_new_map(g);
    }
}

/// `public static final void startNewCharacter()` (`n.c:()V => [iadd,i2b, (isub,imul)
/// ×3, iadd×2]`): finalizes a newly created character — applies the New Game+ class
/// bonus, marks the profile, resets progress, and reads the class start triple.
///
/// DEFERRED: `GameLoop.instance.saveOptions()` (`bs`) is unported; the original wraps
/// it in `try { } catch (Exception) { }`, so skipping it is the swallowed-failure
/// case and observationally identical here.
pub fn start_new_character(g: &mut Game) {
    let id = g
        .game_state
        .hero
        .expect("GameState.hero null in startNewCharacter");
    // Hero player = hero;
    // player.classId = (byte) (player.classId + progressBonus(classId));
    let class_id = g.game_state.class_id;
    let bonus = progress_bonus(g, class_id);
    {
        let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
        hero.class_id = (hero.class_id as i32).wrapping_add(bonus as i32) as i8;
        // if (hero.classId > 100) hero.classId = (byte) 100;
        if hero.class_id > 100 {
            hero.class_id = 100;
        }
    }
    // clearCount = (byte) 1;
    g.game_state.clear_count = 1;
    // GameLoop.instance.hasCreatedCharacter = true;
    g.game_loop.has_created_character = true;
    // try { GameLoop.instance.saveOptions(); } catch (Exception unused) {}
    //   — DEFERRED: GameLoop.saveOptions (bs) unported; the catch swallows failure.
    // clearSwitches(); clearFlags(); setSwitch(0);
    clear_switches(g);
    clear_flags(g);
    set_switch(g, 0);
    // hero.bag.removeQuestItems();
    {
        let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
        item_bag::remove_quest_items(&mut hero.bag);
    }
    // hero.hp = hero.maxHp; hero.mp = hero.maxMp;
    {
        let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
        hero.hp = hero.max_hp;
        hero.mp = hero.max_mp;
    }
    // storyMapId = classStartTable[(classId - 6) * 3];
    let base = (class_id as i32).wrapping_sub(6).wrapping_mul(3);
    g.game_state.story_map_id = g.game_state.class_start_table[base as usize];
    // arg0 = classStartTable[((classId - 6) * 3) + 1];
    g.game_state.arg0 = g.game_state.class_start_table[base.wrapping_add(1) as usize];
    // arg1 = classStartTable[((classId - 6) * 3) + 2];
    g.game_state.arg1 = g.game_state.class_start_table[base.wrapping_add(2) as usize];
}

/// `private static final byte[] packProgress()` (`n.a:()[B => []`): serializes the
/// switch bitset, flag bitset, and `clearCount` into one 257-byte array — 128 switch
/// bytes, then 128 flag bytes, then the single `clearCount` byte — via a
/// `ByteArrayOutputStream`/`DataOutputStream`. The original's `catch (IOException)`
/// returns `null`, but an in-memory `ByteArrayOutputStream` never throws (cf.
/// [`crate::item_bag::serialize`]), so the result is always `Some`. `[]` shape — no
/// arithmetic (plain stream writes).
fn pack_progress(g: &Game) -> Option<Vec<i8>> {
    // dataOut.write(switches); dataOut.write(flags); dataOut.writeByte(clearCount);
    let mut out: Vec<i8> = Vec::new();
    out.extend_from_slice(&g.game_state.switches);
    out.extend_from_slice(&g.game_state.flags);
    out.push(g.game_state.clear_count);
    // return byteStream.toByteArray();
    Some(out)
}

/// `private static final void unpackProgress(byte[] data)` (`n.a:([B)V => []`):
/// restores the switch/flag bitsets and `clearCount` from a [`pack_progress`] blob
/// via a `ByteArrayInputStream`/`DataInputStream`. `DataInputStream.read(byte[])`
/// fills the whole target when enough bytes remain (the decrypted blob always has
/// ≥ 257), so the two 128-byte reads and the trailing `readByte` recover exactly.
/// `[]` shape — no arithmetic. The `catch (IOException)` is unreachable in memory.
fn unpack_progress(g: &mut Game, data: &[i8]) {
    // dataIn.read(switches); dataIn.read(flags); clearCount = dataIn.readByte();
    let mut pos: usize = 0;
    let n_sw = g.game_state.switches.len();
    g.game_state
        .switches
        .copy_from_slice(&data[pos..pos + n_sw]);
    pos += n_sw;
    let n_fl = g.game_state.flags.len();
    g.game_state.flags.copy_from_slice(&data[pos..pos + n_fl]);
    pos += n_fl;
    g.game_state.clear_count = data[pos];
}

/// `public static final byte progressBonus(byte forClassId)`
/// (`n.a:(B)B => [imul, iadd×3, i2b, iinc, iadd, i2b, iinc, (imul, idiv, i2b)×3]`):
/// the New Game+ stat bonus for `forClassId`, scaled by story progress (how many of
/// 20 milestone flags + 6 milestone switches are set) and the per-clear
/// [`GameStateData::clear_bonus_table`] factor. Returns `0` once the story has been
/// cleared three times.
pub fn progress_bonus(g: &Game, for_class_id: i8) -> i8 {
    // if (clearCount >= 3) return (byte) 0;
    if g.game_state.clear_count >= 3 {
        return 0;
    }
    // byte count = 0;
    let mut count: i8 = 0;
    // for (int flagIndex = 0; flagIndex < 20; flagIndex++) if (isFlag(1 + (flagIndex*3) + 1)) count++;
    let mut flag_index: i32 = 0;
    while flag_index < 20 {
        let bit = 1i32
            .wrapping_add(flag_index.wrapping_mul(3))
            .wrapping_add(1);
        if is_flag(g, bit) {
            count = (count as i32).wrapping_add(1) as i8;
        }
        flag_index = flag_index.wrapping_add(1);
    }
    // for (int switchIndex = 100; switchIndex <= 105; switchIndex++) if (isSwitch(switchIndex)) count++;
    let mut switch_index: i32 = 100;
    while switch_index <= 105 {
        if is_switch(g, switch_index) {
            count = (count as i32).wrapping_add(1) as i8;
        }
        switch_index = switch_index.wrapping_add(1);
    }
    // switch (forClassId) { 6: (count*cbt)/19; 7: /21; 8: /16; default: 0 }
    let cbt = g.game_state.clear_bonus_table[g.game_state.clear_count as usize] as i32;
    match for_class_id {
        6 => java_div((count as i32).wrapping_mul(cbt), 19).expect("(count * cbt) / 19") as i8,
        7 => java_div((count as i32).wrapping_mul(cbt), 21).expect("(count * cbt) / 21") as i8,
        8 => java_div((count as i32).wrapping_mul(cbt), 16).expect("(count * cbt) / 16") as i8,
        _ => 0,
    }
}

/// `public static final void saveGame() throws Exception` (`n.o:()V`): writes the
/// encrypted hero / bag / progress / position save blob to the class's RMS slot, plus
/// the encrypted quick-item bar to `/o`.
///
/// The blob is four `[u16 big-endian length][SaveCipher-encrypted payload]` slices,
/// assembled with the original's exact offset arithmetic (`(len & 65280) >> 8` high
/// byte, `len & 255` low byte). DEFERRED: `player.save()` (Hero.save, `ao`) is
/// unported (sibling lane), so the hero slice is written from an **empty**
/// placeholder — the four-slice structure is preserved intact; swap the placeholder
/// for `hero::save(...)` when it lands. The bag / progress / position / quick-item
/// slices are complete (their serialisers are ported).
pub fn save_game(g: &mut Game) -> Result<(), JavaError> {
    let id = g.game_state.hero.expect("GameState.hero null in saveGame");
    let save_key = g.game_state.save_key.clone();
    // Hero player = hero; byte[] heroBytes = player.save();
    //   — DEFERRED: Hero.save (ao) unported; empty placeholder preserves the slice layout.
    let hero_bytes: Vec<i8> = Vec::new();
    // byte[] bagBytes = player.bag.serialize();
    let bag_bytes = {
        let hero = g.entity_arena[id].as_hero().expect("Hero node");
        item_bag::serialize(&hero.bag)
    }
    .ok_or(JavaError::NullPointer)?;
    // byte[] progressBytes = packProgress();
    let progress_bytes = pack_progress(g).ok_or(JavaError::NullPointer)?;
    // byte[] posBytes = {map.mapType, ((Entity) player).tileX, ((Entity) player).tileY};
    let pos_bytes: Vec<i8> = {
        let map_type = g
            .game_state
            .map
            .as_ref()
            .expect("GameState.map null in saveGame")
            .map_type;
        let node = &g.entity_arena[id];
        vec![map_type, node.tile_x, node.tile_y]
    };
    // byte[] encHero/encBag/encProgress/encPos = SaveCipher.encrypt(<slice>, saveKey);
    let enc_hero = save_cipher::encrypt(&hero_bytes, &save_key);
    let enc_bag = save_cipher::encrypt(&bag_bytes, &save_key);
    let enc_progress = save_cipher::encrypt(&progress_bytes, &save_key);
    let enc_pos = save_cipher::encrypt(&pos_bytes, &save_key);
    // byte[] blob = new byte[encHero.length + encBag.length + encProgress.length + encPos.length + 8];
    let blob_len = (enc_hero.len() as i32)
        .wrapping_add(enc_bag.len() as i32)
        .wrapping_add(enc_progress.len() as i32)
        .wrapping_add(enc_pos.len() as i32)
        .wrapping_add(8);
    let mut blob = vec![0i8; blob_len as usize];
    // blob[0] = (byte)((encHero.length & 65280) >> 8); blob[1] = (byte)(encHero.length & 255);
    blob[0] = ishr(enc_hero.len() as i32 & 65280, 8) as i8;
    blob[1] = (enc_hero.len() as i32 & 255) as i8;
    // System.arraycopy(encHero, 0, blob, 2, encHero.length);
    blob[2..2 + enc_hero.len()].copy_from_slice(&enc_hero);
    // int off1 = 2 + encHero.length; int off1b = off1 + 1;
    let off1 = 2i32.wrapping_add(enc_hero.len() as i32);
    let off1b = off1.wrapping_add(1);
    // blob[off1] = (byte)((encBag.length & 65280) >> 8);
    blob[off1 as usize] = ishr(enc_bag.len() as i32 & 65280, 8) as i8;
    // int off2 = off1b + 1;
    let off2 = off1b.wrapping_add(1);
    // blob[off1b] = (byte)(encBag.length & 255);
    blob[off1b as usize] = (enc_bag.len() as i32 & 255) as i8;
    // System.arraycopy(encBag, 0, blob, off2, encBag.length);
    blob[off2 as usize..off2 as usize + enc_bag.len()].copy_from_slice(&enc_bag);
    // int off3 = off2 + encBag.length; int off3b = off3 + 1;
    let off3 = off2.wrapping_add(enc_bag.len() as i32);
    let off3b = off3.wrapping_add(1);
    // blob[off3] = (byte)((encProgress.length & 65280) >> 8);
    blob[off3 as usize] = ishr(enc_progress.len() as i32 & 65280, 8) as i8;
    // int off4 = off3b + 1;
    let off4 = off3b.wrapping_add(1);
    // blob[off3b] = (byte)(encProgress.length & 255);
    blob[off3b as usize] = (enc_progress.len() as i32 & 255) as i8;
    // System.arraycopy(encProgress, 0, blob, off4, encProgress.length);
    blob[off4 as usize..off4 as usize + enc_progress.len()].copy_from_slice(&enc_progress);
    // int off5 = off4 + encProgress.length; int off5b = off5 + 1;
    let off5 = off4.wrapping_add(enc_progress.len() as i32);
    let off5b = off5.wrapping_add(1);
    // blob[off5] = (byte)((encPos.length & 65280) >> 8);
    blob[off5 as usize] = ishr(enc_pos.len() as i32 & 65280, 8) as i8;
    // blob[off5b] = (byte)(encPos.length & 255);
    blob[off5b as usize] = (enc_pos.len() as i32 & 255) as i8;
    // System.arraycopy(encPos, 0, blob, off5b + 1, encPos.length);
    let pos_dst = off5b.wrapping_add(1);
    blob[pos_dst as usize..pos_dst as usize + enc_pos.len()].copy_from_slice(&enc_pos);
    // RmsFile rms = new RmsFile(saveSlots[classId - 6], 0);
    let class_id = g.game_state.class_id;
    let slot = g.game_state.save_slots[(class_id as i32).wrapping_sub(6) as usize].clone();
    let mut rms = rms_file::new_rms_file(&mut g.rms, &slot, 0)?;
    // rms.write(blob, 0, blob.length); rms.close();
    rms_file::write(&mut rms, &blob, 0, blob.len() as i32)?;
    rms_file::close(&mut rms, &mut g.rms);
    // byte[] encQuick = SaveCipher.encrypt(player.quickItems.serialize(), saveKey);
    let quick_ser = {
        let hero = g.entity_arena[id].as_hero().expect("Hero node");
        item_bag::serialize(&hero.quick_items)
    }
    .ok_or(JavaError::NullPointer)?;
    let enc_quick = save_cipher::encrypt(&quick_ser, &save_key);
    // RmsFile quickRms = new RmsFile("/o", 0);
    let mut quick_rms = rms_file::new_rms_file(&mut g.rms, "/o", 0)?;
    // byte[] quickHeader = {(byte)((encQuick.length & 65280) >> 8), (byte)(encQuick.length & 255)};
    let quick_header: Vec<i8> = vec![
        ishr(enc_quick.len() as i32 & 65280, 8) as i8,
        (enc_quick.len() as i32 & 255) as i8,
    ];
    // quickRms.write(quickHeader, 0, quickHeader.length);
    rms_file::write(&mut quick_rms, &quick_header, 0, quick_header.len() as i32)?;
    // quickRms.write(encQuick, 0, encQuick.length);
    rms_file::write(&mut quick_rms, &enc_quick, 0, enc_quick.len() as i32)?;
    // quickRms.close();
    rms_file::close(&mut quick_rms, &mut g.rms);
    Ok(())
}

/// `private static final void loadGame() throws Exception` (`n.r:()V => [isub,
/// (iand,ishl,iand,ior)×5]`): reads and decrypts the save blob, restoring bag,
/// progress, and position (plus the `/o` quick-item bar). Each slice length is the
/// big-endian `((header[0] & 255) << 8) | (header[1] & 255)`.
///
/// DEFERRED: `hero.load(SaveCipher.decrypt(heroBytes, saveKey))` (Hero.load, `ao`) is
/// unported, so the hero slice is still READ (to keep the RMS cursor aligned) and
/// decrypted, but not applied. Exposed `pub` for the save/load oracle; the original's
/// visibility is `private`.
pub fn load_game(g: &mut Game) -> Result<(), JavaError> {
    let id = g.game_state.hero.expect("GameState.hero null in loadGame");
    let save_key = g.game_state.save_key.clone();
    let class_id = g.game_state.class_id;
    // byte[] header = new byte[2];
    let mut header = vec![0i8; 2];
    // `header.length` (constant 2) — hoisted so the read call does not borrow `header`
    // both mutably (the buffer) and immutably (its length) at once.
    let hlen = header.len() as i32;
    // RmsFile rms = new RmsFile(saveSlots[classId - 6], 1);
    let slot = g.game_state.save_slots[(class_id as i32).wrapping_sub(6) as usize].clone();
    let mut rms = rms_file::new_rms_file(&mut g.rms, &slot, 1)?;
    // rms.read(header, 0, header.length);
    rms_file::read(&mut rms, &g.rms, &mut header, 0, hlen)?;
    // byte[] heroBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
    let hero_len = ishl(header[0] as i32 & 255, 8) | (header[1] as i32 & 255);
    // rms.read(heroBytes, 0, heroBytes.length);
    let mut hero_bytes = vec![0i8; hero_len as usize];
    rms_file::read(&mut rms, &g.rms, &mut hero_bytes, 0, hero_len)?;
    // hero.load(SaveCipher.decrypt(heroBytes, saveKey));
    //   — DEFERRED: Hero.load (ao) unported; slice decrypted for parity but not applied.
    let _ = save_cipher::decrypt(&hero_bytes, &save_key);
    // rms.read(header, 0, header.length);
    rms_file::read(&mut rms, &g.rms, &mut header, 0, hlen)?;
    // byte[] bagBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
    let bag_len = ishl(header[0] as i32 & 255, 8) | (header[1] as i32 & 255);
    // rms.read(bagBytes, 0, bagBytes.length);
    let mut bag_bytes = vec![0i8; bag_len as usize];
    rms_file::read(&mut rms, &g.rms, &mut bag_bytes, 0, bag_len)?;
    // hero.bag.deserialize(SaveCipher.decrypt(bagBytes, saveKey));
    let dec_bag = save_cipher::decrypt(&bag_bytes, &save_key).ok_or(JavaError::NullPointer)?;
    {
        // ItemBag.deserialize needs &mut Game (item creation), so the bag is moved out
        // of the hero, filled, and moved back — the store never aliases the arena.
        let mut bag = {
            let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
            std::mem::replace(&mut hero.bag, item_bag::new(30))
        };
        item_bag::deserialize(&mut bag, g, &dec_bag);
        g.entity_arena[id].as_hero_mut().expect("Hero node").bag = bag;
    }
    // rms.read(header, 0, header.length);
    rms_file::read(&mut rms, &g.rms, &mut header, 0, hlen)?;
    // byte[] progressBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
    let progress_len = ishl(header[0] as i32 & 255, 8) | (header[1] as i32 & 255);
    // rms.read(progressBytes, 0, progressBytes.length);
    let mut progress_bytes = vec![0i8; progress_len as usize];
    rms_file::read(&mut rms, &g.rms, &mut progress_bytes, 0, progress_len)?;
    // unpackProgress(SaveCipher.decrypt(progressBytes, saveKey));
    let dec_progress =
        save_cipher::decrypt(&progress_bytes, &save_key).ok_or(JavaError::NullPointer)?;
    unpack_progress(g, &dec_progress);
    // rms.read(header, 0, header.length);
    rms_file::read(&mut rms, &g.rms, &mut header, 0, hlen)?;
    // byte[] posBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
    let pos_len = ishl(header[0] as i32 & 255, 8) | (header[1] as i32 & 255);
    // rms.read(posBytes, 0, posBytes.length);
    let mut pos_bytes = vec![0i8; pos_len as usize];
    rms_file::read(&mut rms, &g.rms, &mut pos_bytes, 0, pos_len)?;
    // byte[] pos = SaveCipher.decrypt(posBytes, saveKey);
    let pos = save_cipher::decrypt(&pos_bytes, &save_key).ok_or(JavaError::NullPointer)?;
    // storyMapId = pos[0]; arg0 = pos[1]; arg1 = pos[2];
    g.game_state.story_map_id = pos[0];
    g.game_state.arg0 = pos[1];
    g.game_state.arg1 = pos[2];
    // rms.close();
    rms_file::close(&mut rms, &mut g.rms);
    // RmsFile quickRms = new RmsFile("/o", 1);
    let mut quick_rms = rms_file::new_rms_file(&mut g.rms, "/o", 1)?;
    // quickRms.read(header, 0, header.length);
    rms_file::read(&mut quick_rms, &g.rms, &mut header, 0, hlen)?;
    // byte[] quickBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
    let quick_len = ishl(header[0] as i32 & 255, 8) | (header[1] as i32 & 255);
    // quickRms.read(quickBytes, 0, quickBytes.length);
    let mut quick_bytes = vec![0i8; quick_len as usize];
    rms_file::read(&mut quick_rms, &g.rms, &mut quick_bytes, 0, quick_len)?;
    // hero.quickItems.deserialize(SaveCipher.decrypt(quickBytes, saveKey));
    let dec_quick = save_cipher::decrypt(&quick_bytes, &save_key).ok_or(JavaError::NullPointer)?;
    {
        let mut quick = {
            let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
            std::mem::replace(&mut hero.quick_items, item_bag::new(15))
        };
        item_bag::deserialize(&mut quick, g, &dec_quick);
        g.entity_arena[id]
            .as_hero_mut()
            .expect("Hero node")
            .quick_items = quick;
    }
    // quickRms.close();
    rms_file::close(&mut quick_rms, &mut g.rms);
    Ok(())
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

// --- Hero state accessors + the world simulation step (GameState.java:357-567) ---

/// `public static final byte heroState()` (`n`) — `((Battler) hero).state`.
pub fn hero_state(g: &Game) -> i8 {
    let id = g.game_state.hero.expect("GameState.hero null in heroState");
    g.entity_arena[id].as_battler().expect("Hero battler").state
}

/// `public static final byte heroFacing()` (`n`) — `((Battler) hero).facing`.
pub fn hero_facing(g: &Game) -> i8 {
    let id = g
        .game_state
        .hero
        .expect("GameState.hero null in heroFacing");
    g.entity_arena[id]
        .as_battler()
        .expect("Hero battler")
        .facing
}

/// `public static final void setHeroState(byte state)` (`n`) — `hero.setState(state)`.
pub fn set_hero_state(g: &mut Game, state: i8) {
    let id = g
        .game_state
        .hero
        .expect("GameState.hero null in setHeroState");
    let b = g.entity_arena[id].as_battler_mut().expect("Hero battler");
    battler::set_state(b, state);
}

/// `public static final void setHeroFacing(byte facing)` (`n`) — `hero.setFacing(facing)`.
pub fn set_hero_facing(g: &mut Game, facing: i8) {
    let id = g
        .game_state
        .hero
        .expect("GameState.hero null in setHeroFacing");
    let b = g.entity_arena[id].as_battler_mut().expect("Hero battler");
    battler::set_facing(b, facing);
}

/// `public static final boolean isHeroOnGrid()` (`n`) — true when the hero is aligned
/// to the tile grid (`!(offGridX || offGridY)`).
pub fn is_hero_on_grid(g: &Game) -> bool {
    let id = g
        .game_state
        .hero
        .expect("GameState.hero null in isHeroOnGrid");
    let node = &g.entity_arena[id];
    // return (offGridX || offGridY) ? false : true;
    !(node.off_grid_x || node.off_grid_y)
}

/// `public static final void walkHero(byte direction)` (`n`, GameState.java:357):
/// starts the hero walking in `direction`, or queues it if already mid-step.
pub fn walk_hero(g: &mut Game, direction: i8) {
    // if (heroState() == 1) { pendingHeroState = 0; pendingHeroFacing = 0;
    //   setHeroState(2); setHeroFacing(direction); return; }
    if hero_state(g) == 1 {
        g.game_state.pending_hero_state = 0;
        g.game_state.pending_hero_facing = 0;
        set_hero_state(g, 2);
        set_hero_facing(g, direction);
        return;
    }
    // if (heroState() == 2) { pendingHeroState = 2; pendingHeroFacing = direction; }
    if hero_state(g) == 2 {
        g.game_state.pending_hero_state = 2;
        g.game_state.pending_hero_facing = direction;
    }
}

/// `public static final void stopHero()` (`n`, GameState.java:372): queues the hero to
/// stop (return to idle) keeping the current facing.
pub fn stop_hero(g: &mut Game) {
    // pendingHeroState = (byte) 1; pendingHeroFacing = heroFacing();
    g.game_state.pending_hero_state = 1;
    g.game_state.pending_hero_facing = hero_facing(g);
}

/// `public static final void update()` (`n`, GameState.java:391): one simulation
/// step — apply any pending hero action, refresh the hero, update the world.
pub fn update(g: &mut Game) {
    // applyPendingHeroAction();
    apply_pending_hero_action(g);
    // updateHero();
    update_hero(g);
    // map.updateWorld();
    game_map::update_world(g);
}

/// `private static final void applyPendingHeroAction()` (`n`, GameState.java:402):
/// when the hero is grid-aligned, applies the queued state/facing and clears it.
pub fn apply_pending_hero_action(g: &mut Game) {
    // if (isHeroOnGrid() && pendingHeroState != 0) {
    if is_hero_on_grid(g) && g.game_state.pending_hero_state != 0 {
        // setHeroState(pendingHeroState); setHeroFacing(pendingHeroFacing);
        let ps = g.game_state.pending_hero_state;
        let pf = g.game_state.pending_hero_facing;
        set_hero_state(g, ps);
        set_hero_facing(g, pf);
        // pendingHeroState = 0; pendingHeroFacing = 0;
        g.game_state.pending_hero_state = 0;
        g.game_state.pending_hero_facing = 0;
    }
}

/// `public static final void updateHero()` (`n`, GameState.java:547): refreshes the
/// hero and re-sorts it in the map's draw list. The active-guardian refresh is
/// DEFERRED (`activeGuardian` is null — no guardian summon in this slice).
pub fn update_hero(g: &mut Game) {
    let id = g
        .game_state
        .hero
        .expect("GameState.hero null in updateHero");
    // hero.update();
    hero::update(g, id);
    // map.unlinkEntity(hero);
    game_map::unlink_entity(g, id);
    // Guardian guardian = hero.getActiveGuardian(); if (guardian != null) { guardian.update();
    //   map.unlinkEntity(guardian); }   — DEFERRED (activeGuardian null).
}

/// `public static final void scrollCamera(boolean lead, boolean followHero)`
/// (`n.a:(ZZ)V => [imul×2, idiv×4, isub×10, iadd×8]`, GameState.java:336): eases the
/// camera toward its target. With `followHero` false it eases both axes; with it true
/// it leads by the facing direction and eases only the axis the facing does not lock.
pub fn scroll_camera(g: &mut Game, lead: bool, follow_hero: bool) {
    // if (!followHero) { camX += (((camTargetX - camX) + 1) / 2) - 1;
    //                    camY += (((camTargetY - camY) + 1) / 2) - 1; return; }
    if !follow_hero {
        let dx = java_div(
            g.game_state
                .cam_target_x
                .wrapping_sub(g.game_state.cam_x)
                .wrapping_add(1),
            2,
        )
        .expect("(((camTargetX - camX) + 1) / 2)")
        .wrapping_sub(1);
        g.game_state.cam_x = g.game_state.cam_x.wrapping_add(dx);
        let dy = java_div(
            g.game_state
                .cam_target_y
                .wrapping_sub(g.game_state.cam_y)
                .wrapping_add(1),
            2,
        )
        .expect("(((camTargetY - camY) + 1) / 2)")
        .wrapping_sub(1);
        g.game_state.cam_y = g.game_state.cam_y.wrapping_add(dy);
        return;
    }
    // byte facing = heroFacing();
    let facing = hero_facing(g) as usize;
    // if (lead) { camTargetY -= 15 * dirDy[facing]; camTargetX -= 15 * dirDx[facing]; }
    if lead {
        let ldy = 15i32.wrapping_mul(directions::DIR_DY[facing] as i32);
        g.game_state.cam_target_y = g.game_state.cam_target_y.wrapping_sub(ldy);
        let ldx = 15i32.wrapping_mul(directions::DIR_DX[facing] as i32);
        g.game_state.cam_target_x = g.game_state.cam_target_x.wrapping_sub(ldx);
    }
    // if (!facingIsHorizontal[facing] && camY != camTargetY) camY += (((camTargetY - camY) + 1) / 2) - 1;
    if !directions::FACING_IS_HORIZONTAL[facing] && g.game_state.cam_y != g.game_state.cam_target_y
    {
        let dy = java_div(
            g.game_state
                .cam_target_y
                .wrapping_sub(g.game_state.cam_y)
                .wrapping_add(1),
            2,
        )
        .expect("(((camTargetY - camY) + 1) / 2)")
        .wrapping_sub(1);
        g.game_state.cam_y = g.game_state.cam_y.wrapping_add(dy);
    }
    // if (!facingIsHorizontal[facing] || camX == camTargetX) return;
    if !directions::FACING_IS_HORIZONTAL[facing] || g.game_state.cam_x == g.game_state.cam_target_x
    {
        return;
    }
    // camX += (((camTargetX - camX) + 1) / 2) - 1;
    let dx = java_div(
        g.game_state
            .cam_target_x
            .wrapping_sub(g.game_state.cam_x)
            .wrapping_add(1),
        2,
    )
    .expect("(((camTargetX - camX) + 1) / 2)")
    .wrapping_sub(1);
    g.game_state.cam_x = g.game_state.cam_x.wrapping_add(dx);
}
