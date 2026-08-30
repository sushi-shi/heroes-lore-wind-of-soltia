//! Transliterated from `java/src/main/java/defpackage/GameMap.java`
//! (original `ae.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! One loaded level: the tile and collision grids, the tile-occupancy array, the
//! NPC/object/enemy entity list ([`crate::entity_list`]), dropped pickups, the
//! delayed-enemy spawn queue, the trigger/event/dialogue tables, and the
//! boss-encounter setup. `GameState.map` (`n.a:Lae;`) owns the active
//! [`GameMapState`]; its per-instance fields live there while the class-level
//! mutable statics live on the always-present [`GameMapClassState`].
//!
//! **This slice is the FIELD LAYER + the constructor only.** The packed
//! `.map`/`.evt` parse ([`load`] and its `parse*` helpers) and the world/minimap
//! rendering ([`paint`]/[`draw_tiles`]/[`draw_entities`]) — which reach
//! `AssetCache`/`PngMerger`/`EnemyType`/`AudioManager`/the enemy hierarchy — are
//! DEFERRED to the render + world-logic lanes.
//!
//! `ownership.tsv` rows: `minimapScale` (`ae.c`) and `lastTilesetId` (`ae.d`) are
//! mutable statics owned by [`GameMapClassState`]; `minimapColors` (`ae.a`) and
//! `musicByTileset` (`ae.h`) are `static final` constants reproduced as `const`.
//!
//! Opcode shape (R8, `_reference/numeric_shapes.json`): `ae.<init>:(B)V => []`
//! (the ctor body is boolean tests + object construction — no arithmetic).

use crate::base_canvas::BaseCanvasState;
use crate::entity::EntityId;
use crate::entity_list::EntityListState;
use crate::game::Game;

/// `private static final int[] minimapColors` (`ae.a`) — minimap floor/wall colors,
/// two per tileset id. Read by the DEFERRED minimap lane (`paintMinimap`).
#[allow(dead_code)]
const MINIMAP_COLORS: [i32; 30] = [
    16768831, 4136767, 16768831, 4136767, 16768959, 8339263, 12582719, 2047807, 0, 0, 12582719,
    2047807, 4177919, 2047807, 12582719, 2047807, 12582719, 2047807, 12582719, 2047807, 0, 0,
    16768959, 8339263, 14680063, 2047871, 14680063, 2047871, 16768959, 8339263,
];

/// `private static final byte[] musicByTileset` (`ae.h`) — music track id per
/// tileset id (`-1` = keep current). Read by the DEFERRED [`load`] music setup.
#[allow(dead_code)]
const MUSIC_BY_TILESET: [i8; 20] = [
    28, 0, 27, 29, -1, 1, 26, 31, 31, 2, -1, 25, 24, 24, 30, 3, -1, -1, -1, -1,
];

/// `GameMap`'s mutable class-level statics — they persist across map loads and
/// while no map is active, so they live here (always present) rather than on the
/// Optional per-instance [`GameMapState`].
#[derive(Debug)]
pub struct GameMapClassState {
    /// `private static byte minimapScale = 2;` (`ae.c`) — minimap tile size in px
    /// (bumped to 3 on larger screens, in the ctor).
    pub minimap_scale: i8,
    /// `private static byte lastTilesetId = -1;` (`ae.d`) — tileset id of the
    /// previously loaded map (skips reloading tiles).
    pub last_tileset_id: i8,
}

impl Default for GameMapClassState {
    fn default() -> Self {
        // static field initializers: minimapScale = 2; lastTilesetId = -1;
        GameMapClassState {
            minimap_scale: 2,
            last_tileset_id: -1,
        }
    }
}

/// One loaded level's per-instance data (`GameMap` `ae` instance fields). The
/// `Default` reproduces the class-body field initializers (`cameraShiftX = 0`,
/// `cameraShiftY = 0`, `combatEnabled = false`; everything else 0/false/null/empty);
/// [`new_game_map`] reproduces the `GameMap(byte)` constructor.
#[derive(Debug, Default)]
pub struct GameMapState {
    /// `public byte mapType;` (`ae.a`) — map type/id (drives boss/music/camera).
    pub map_type: i8,
    /// `public byte tilesetId;`
    pub tileset_id: i8,
    /// `public boolean bossMap;` (`ae.a`) — true for the four boss map types.
    pub boss_map: bool,
    /// `public boolean lockedCamera;` (`ae.b`) — fixed boss-arena camera.
    pub locked_camera: bool,
    /// `private byte zoneBannerTimer;` (`ae.e`) — zone-name banner countdown.
    pub zone_banner_timer: i8,
    /// `public int widthTiles;` (`ae.a`)
    pub width_tiles: i32,
    /// `public int heightTiles;` (`ae.b`)
    pub height_tiles: i32,
    /// `public int widthPx;` (`ae.c`)
    pub width_px: i32,
    /// `public int heightPx;` (`ae.d`)
    pub height_px: i32,
    /// `public byte[][] tileGrid;` (`ae.b`) — null until [`load`] parses it.
    pub tile_grid: Option<Vec<Vec<i8>>>,
    /// `public byte[][] collisionGrid;` (`ae.c`)
    pub collision_grid: Option<Vec<Vec<i8>>>,
    /// `public Entity[][] occupancy;` (`ae.a`) — per-tile occupant handles.
    pub occupancy: Option<Vec<Vec<Option<EntityId>>>>,
    /// `public Npc[] npcs;` (`ae.a`) — the map's NPC handles (Npc DEFERRED).
    pub npcs: Option<Vec<Option<EntityId>>>,
    /// `public MapObject[] objects;` (`ae.a`) — the map's prop handles.
    pub objects: Option<Vec<Option<EntityId>>>,
    /// `private EntityList entities;` (`ae.a`) — the z-sorted draw list (endpoints
    /// into the shared [`crate::entity::EntityArena`]).
    pub entities: EntityListState,
    /// `private Vector pickups;` (`ae.a`) — dropped pickups (each a packed `byte[]`).
    pub pickups: Vec<Vec<i8>>,
    /// `private Vector spawnQueue;` (`ae.b`) — delayed enemy spawns (each an `int[]`).
    pub spawn_queue: Vec<Vec<i32>>,
    /// `private int spawnTick;` — countdown driving the spawn/fade processing.
    pub spawn_tick: i32,
    /// `public Object[] triggers;` (`ae.a`) — per-tile trigger tables (`byte[][]`).
    pub triggers: Option<Vec<Option<Vec<Vec<i8>>>>>,
    /// `public Object[] eventScripts;` (`ae.b`) — event tables (`byte[][]`).
    pub event_scripts: Option<Vec<Option<Vec<Vec<i8>>>>>,
    /// `public Object[] dialogueStrings;` (`ae.c`) — decoded dialogue (`char[]`).
    pub dialogue_strings: Option<Vec<Option<Vec<u16>>>>,
    /// `private boolean minimapBlink;` (`ae.d`) — toggles the minimap dot each frame.
    pub minimap_blink: bool,
    /// `private byte[] mapData;` — scratch holding raw `.map`/`.evt` bytes while parsing.
    pub map_data: Option<Vec<i8>>,
    /// `public char[] zoneName;` (`ae.a`) — zone name for the banner/minimap header.
    pub zone_name: Option<Vec<u16>>,
    /// `public int cameraShiftX = 0;` (`ae.e`) — one-frame camera nudge X.
    pub camera_shift_x: i32,
    /// `public int cameraShiftY = 0;` — one-frame camera nudge Y.
    pub camera_shift_y: i32,
    /// `public boolean combatEnabled = false;` (`ae.c`)
    pub combat_enabled: bool,
}

/// `public static final boolean isBossMap(int mapType)` — the four boss map types.
pub fn is_boss_map(map_type: i32) -> bool {
    // return mapType == 11 || mapType == 13 || mapType == 15 || mapType == 82;
    map_type == 11 || map_type == 13 || map_type == 15 || map_type == 82
}

/// `public GameMap(byte b)` (`ae.<init>:(B)V => []`). Builds the per-instance map
/// and, on a large screen, bumps the class-level `minimapScale` static.
pub fn new_game_map(
    class_state: &mut GameMapClassState,
    base_canvas: &BaseCanvasState,
    b: i8,
) -> GameMapState {
    let map = GameMapState {
        // this.mapType = b;
        map_type: b,
        // this.bossMap = isBossMap(b);
        boss_map: is_boss_map(b as i32),
        // this.lockedCamera = b == 13 || b == 15;
        locked_camera: b == 13 || b == 15,
        // this.entities = new EntityList();  this.pickups = new Vector();
        // this.spawnQueue = new Vector();
        entities: EntityListState::new(),
        // this.spawnTick = 16;
        spawn_tick: 16,
        ..GameMapState::default()
    };
    // if (BaseCanvas.width < 240 || BaseCanvas.height < 240) return;
    if base_canvas.width < 240 || base_canvas.height < 240 {
        return map;
    }
    // minimapScale = (byte) 3;   (mutates the static)
    class_state.minimap_scale = 3;
    map
}

// --- DEFERRED: the .map/.evt parse and the world/minimap rendering -------------

/// `public final void load()` (`ae.load`) — parses the packed `.map`/`.evt` assets
/// into the grids, entities, triggers, and boss setup. Reaches `AssetCache`,
/// `PngMerger`, `EnemyType`, `AudioManager`, and the enemy/npc hierarchy. DEFERRED.
pub fn load(_g: &mut Game) {
    unimplemented!("DEFERRED: GameMap.load — not ported in this slice")
}

/// `public final void paint(Graphics graphics)` — draws the world, zone banner, and
/// entities at the resolved camera offset. DEFERRED to the render lane.
pub fn paint(_g: &mut Game) {
    unimplemented!("DEFERRED: GameMap.paint — not ported in this slice")
}

/// `private final void drawTiles(Graphics, int, int, int, int)` — draws the visible
/// tile window. DEFERRED to the render lane.
pub fn draw_tiles(_g: &mut Game, _cam_x: i32, _cam_y: i32, _view_w: i32, _view_h: i32) {
    unimplemented!("DEFERRED: GameMap.drawTiles — not ported in this slice")
}

/// `private final void drawEntities(Graphics, int, int)` — paints the entity list
/// head→tail at the camera offset. DEFERRED to the render lane.
pub fn draw_entities(_g: &mut Game, _cam_x: i32, _cam_y: i32) {
    unimplemented!("DEFERRED: GameMap.drawEntities — not ported in this slice")
}
