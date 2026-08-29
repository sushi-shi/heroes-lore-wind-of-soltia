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

use crate::game::Game;

/// Java `n` / `GameState` state. Every field is `static` (see
/// `java/reconstruction/ownership.tsv`); struct order preserves the reviewed
/// declaration order. Reference-typed statics (`map`, `hero`) are presence flags
/// (`false` while null); their referent classes are not ported yet.
#[derive(Debug)]
pub struct GameStateData {
    /// `public static GameMap map;` — active map; GameMap not ported (null → false).
    pub map: bool,
    /// `public static int camTargetX;`
    pub cam_target_x: i32,
    /// `public static int camTargetY;`
    pub cam_target_y: i32,
    /// `public static int camX;`
    pub cam_x: i32,
    /// `public static int camY;`
    pub cam_y: i32,
    /// `private static Hero hero;` — the player character; Hero not ported
    /// (null → false).
    pub hero: bool,
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
            map: false,
            cam_target_x: 0,
            cam_target_y: 0,
            cam_x: 0,
            cam_y: 0,
            hero: false,
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
