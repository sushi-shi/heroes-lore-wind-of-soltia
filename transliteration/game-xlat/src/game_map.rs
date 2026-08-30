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
//! **This slice ports the FIELD LAYER + the constructor + the TILE render path.**
//! [`load`] reads the packed `/m/<NN>.map` (tileset id + dimensions + the flat tile
//! grid) and decodes the `/m/t/t<NN>` tileset atlas via [`crate::png_merger`];
//! [`paint`]/[`draw_tiles`] resolve + clamp the camera and blit the visible 16px
//! tile window. Still DEFERRED: the `/m/<classId>/<NN>.evt` parse (`parse*` helpers —
//! collision/objects/npcs/enemies/faces/triggers + boss setup), the map's
//! audio/zone-name, and the entity/pickup/minimap rendering (`drawEntities`,
//! `drawPickups`, `paintMinimap`) — those reach `EnemyType`/`AudioManager`/the
//! enemy-NPC hierarchy and the DEFERRED sprite banks.
//!
//! `ownership.tsv` rows: `minimapScale` (`ae.c`) and `lastTilesetId` (`ae.d`) are
//! mutable statics owned by [`GameMapClassState`]; `minimapColors` (`ae.a`) and
//! `musicByTileset` (`ae.h`) are `static final` constants reproduced as `const`.
//!
//! Opcode shape (R8, `_reference/numeric_shapes.json`): `ae.<init>:(B)V => []`
//! (the ctor body is boolean tests + object construction — no arithmetic).

use crate::asset_cache;
use crate::base_canvas::BaseCanvasState;
use crate::entity::{EntityId, EntityKind};
use crate::entity_list::{self, EntityListState};
use crate::game::Game;
use crate::hero;
use crate::png_merger;
use j2me_jvm::java_div;

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

/// `public final void addEntity(Entity ckVar)` (`ae`) — links `id` into the map's
/// z-sorted draw list and depth-sorts it.
pub fn add_entity(g: &mut Game, id: EntityId) {
    // this.entities.addBack(ckVar); this.entities.reorderByDepth(ckVar);
    let Game {
        game_state,
        entity_arena,
        ..
    } = &mut *g;
    let map = game_state
        .map
        .as_mut()
        .expect("GameState.map null in addEntity");
    entity_list::add_back(&mut map.entities, entity_arena, id);
    entity_list::reorder_by_depth(&mut map.entities, entity_arena, id);
}

/// `public final void fadeStep()` (`ae`) — one spawn-tick advance used to seed the
/// map after a warp. The delayed-enemy `spawnQueue` is empty in this slice (the
/// `.evt` enemy parse is DEFERRED), so `processSpawnQueue` only advances `spawnTick`
/// (from 0 → -1 → reset 16) with no spawn body.
pub fn fade_step(g: &mut Game) {
    let map = g
        .game_state
        .map
        .as_mut()
        .expect("GameState.map null in fadeStep");
    // this.spawnTick = 0;
    map.spawn_tick = 0;
    // processSpawnQueue(false, (byte) 3):  spawnTick--; if (<0) spawnTick = 16; <loop over
    //   the empty spawnQueue — DEFERRED enemy-spawn body>.
    map.spawn_tick = map.spawn_tick.wrapping_sub(1);
    if map.spawn_tick < 0 {
        map.spawn_tick = 16;
        // (empty spawnQueue → no delayed spawns; the enemy-spawn body is DEFERRED.)
    }
}

/// `public final void load()` (`ae.load`) — reads the packed `/m/<NN>.map` (byte0
/// tilesetId, byte1 widthTiles, byte2 heightTiles, then the flat tile-index grid)
/// into the tile grid + occupancy array, and lazily decodes the `/m/t/t<NN>` tileset
/// atlas into [`asset_cache::AssetCacheState::map_tiles`] via [`png_merger`].
///
/// **The `.evt` half is DEFERRED.** The original then reads
/// `/m/<classId>/<NN>.evt` and runs `parseCollision`/`parseObjects`/`parseNpcs`/
/// `parseEnemies`/`parseFaces`/`parseTriggers` + `applyInitialPatches` + the boss
/// setup, plus the map's audio/zone-name. Those reach `EnemyType`/`AudioManager`/the
/// enemy-NPC hierarchy and the `mapNameText` table; they are stubbed here
/// (`collisionGrid`/entity tables stay null, `zoneBannerTimer = 0`), which the
/// tile-render path does not read. The leading `AssetCache.unload*` / `AudioManager`
/// / `System.out` / `clearFloaters` calls are DEFERRED no-ops on this path.
pub fn load(g: &mut Game) {
    // this.mapData = readResource("/m/" + (storyMapId < 10 ? "0" : "") + storyMapId + ".map");
    let story = g.game_state.story_map_id as i32;
    let pad = if story < 10 { "0" } else { "" };
    let path = format!("/m/{pad}{story}.map");
    let map_data = asset_cache::read_resource(g, &path).expect("readResource(.map) returned null");
    // this.tilesetId = mapData[0]; widthTiles = mapData[1]; heightTiles = mapData[2];
    //   (widthTiles/heightTiles are int fields; the byte sign-extends.)
    let tileset_id = map_data[0];
    let width_tiles = map_data[1] as i32;
    let height_tiles = map_data[2] as i32;
    // if (tilesetId not in {1,5,9,15}) combatEnabled = true;
    let combat_enabled = tileset_id != 1 && tileset_id != 5 && tileset_id != 9 && tileset_id != 15;
    // this.occupancy = new Entity[heightTiles][widthTiles];
    let occupancy: Vec<Vec<Option<EntityId>>> =
        vec![vec![None; width_tiles as usize]; height_tiles as usize];
    // widthPx = widthTiles * 16; heightPx = heightTiles * 16;
    let width_px = width_tiles.wrapping_mul(16);
    let height_px = height_tiles.wrapping_mul(16);
    // parseTiles(mapData, 3): heightTiles rows of widthTiles bytes, from offset 3.
    let mut tile_grid: Vec<Vec<i8>> = vec![vec![0i8; width_tiles as usize]; height_tiles as usize];
    let mut off: usize = 3;
    for row in tile_grid.iter_mut() {
        // System.arraycopy(bArr, i, tileGrid[i2], 0, widthTiles); i += widthTiles;
        row.copy_from_slice(&map_data[off..off + width_tiles as usize]);
        off += width_tiles as usize;
    }
    // this.mapData = null;
    // if (lastTilesetId != tilesetId) AssetCache.unloadMapTiles();
    if g.game_map_class.last_tileset_id != tileset_id {
        asset_cache::unload_map_tiles(g);
    }
    // (DEFERRED: the whole /m/<classId>/<NN>.evt parse + boss setup + music/zone name.)
    // if (AssetCache.mapTiles == null) mapTiles = new PngMerger("/m/t/t"+pad+tilesetId).allImages();
    if g.asset_cache.map_tiles.is_none() {
        let tid = tileset_id as i32;
        let tpad = if tid < 10 { "0" } else { "" };
        let tpath = format!("/m/t/t{tpad}{tid}");
        let mut merger = png_merger::construct(g, &tpath);
        let frames = png_merger::all_images(g, &mut merger);
        g.asset_cache.map_tiles = Some(frames);
    }
    // lastTilesetId = tilesetId;
    g.game_map_class.last_tileset_id = tileset_id;
    // (DEFERRED: zoneName = mapNameText.get(mapType); zoneBannerTimer; music; clearFloaters.)
    // Publish the parsed fields onto the live map instance.
    let map = g
        .game_state
        .map
        .as_mut()
        .expect("GameState.map null in load");
    map.tileset_id = tileset_id;
    map.width_tiles = width_tiles;
    map.height_tiles = height_tiles;
    map.combat_enabled = combat_enabled;
    map.occupancy = Some(occupancy);
    map.width_px = width_px;
    map.height_px = height_px;
    map.tile_grid = Some(tile_grid);
    map.zone_banner_timer = 0;
}

/// `public final void paint(Graphics graphics)` — resolves + clamps the camera and
/// draws the visible tile window. DEFERRED past the tiles: `drawPickups`,
/// `drawEntities` (which would paint the DEFERRED entity sprites), and the zone
/// banner are not drawn in this milestone slice.
pub fn paint(g: &mut Game) {
    // int i = cameraFollow ? camX : camTargetX; int i2 = cameraFollow ? camY : camTargetY;
    let mut i = if g.game_loop.camera_follow {
        g.game_state.cam_x
    } else {
        g.game_state.cam_target_x
    };
    let mut i2 = if g.game_loop.camera_follow {
        g.game_state.cam_y
    } else {
        g.game_state.cam_target_y
    };
    // int i3 = GameScreen.width; int i4 = GameScreen.worldHeight;
    let i3 = g.game_screen.width;
    let i4 = g.game_screen.world_height;
    let (width_px, height_px, locked_camera) = {
        let map = g
            .game_state
            .map
            .as_ref()
            .expect("GameState.map null in paint");
        (map.width_px, map.height_px, map.locked_camera)
    };
    // if (lockedCamera) { i = camTargetX; i2 = camTargetY + 30; }
    if locked_camera {
        i = g.game_state.cam_target_x;
        i2 = g.game_state.cam_target_y.wrapping_add(30);
    }
    // if (i > 0) i = 0; if (i < i3 - widthPx) i = i3 - widthPx;
    if i > 0 {
        i = 0;
    }
    if i < i3.wrapping_sub(width_px) {
        i = i3.wrapping_sub(width_px);
    }
    // if (i2 > 0) i2 = 0; if (i2 < i4 - heightPx) i2 = i4 - heightPx;
    if i2 > 0 {
        i2 = 0;
    }
    if i2 < i4.wrapping_sub(height_px) {
        i2 = i4.wrapping_sub(height_px);
    }
    // Small-map centering + black fill (only when the map is narrower/shorter than
    // the view — never on the tile maps here). Reproduced; graphics acquired only if
    // a branch fires.
    if i > 0 {
        // i = (i3 - widthPx) / 2; graphics.setColor(0); graphics.fillRect(0, 0, i3, i4);
        i = java_div(i3.wrapping_sub(width_px), 2).expect("(i3 - widthPx) / 2");
        black_fill(g, i3, i4);
    }
    if i2 > 0 {
        // i2 = (i4 - heightPx) / 2; graphics.setColor(0); graphics.fillRect(0, 0, i3, i4);
        i2 = java_div(i4.wrapping_sub(height_px), 2).expect("(i4 - heightPx) / 2");
        black_fill(g, i3, i4);
    }
    // graphics.setClip(0, 0, i3, i4);   — reproduced inside draw_tiles (see note there).
    // Apply the one-frame camera nudge (cameraShiftX/Y default 0 → no-op), clearing it.
    {
        let map = g
            .game_state
            .map
            .as_mut()
            .expect("GameState.map null in paint");
        if map.camera_shift_x != 0 {
            i = i.wrapping_add(map.camera_shift_x);
            map.camera_shift_x = 0;
        }
        if map.camera_shift_y != 0 {
            i2 = i2.wrapping_add(map.camera_shift_y);
            map.camera_shift_y = 0;
        }
    }
    // drawTiles(graphics, i, i2, i3, i4);
    draw_tiles(g, i, i2, i3, i4);
    // drawPickups(graphics, i, i2);  — DEFERRED (no pickups in this slice's map load).
    // drawEntities(graphics, i, i2);
    draw_entities(g, i, i2);
    // (DEFERRED: the zone banner.)
    // graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);  — no draw follows.
}

/// `private final void drawEntities(Graphics graphics, int i, int i2)`
/// (`ae.b:(…Graphics;II)V => []`) — walks the map's z-sorted entity list head→tail
/// (front to back by depth) and paints each at the camera offset (`i`,`i2`).
///
/// The concrete `Entity.paint` is a virtual call; this slice dispatches on the node's
/// [`EntityKind`]. On the first world frame the only linked entity is the hero (the
/// `.evt` object/npc/enemy parse that would add [`crate::map_object`]/Npc/Enemy nodes
/// is DEFERRED), so those subclass paints are DEFERRED and cannot appear here yet.
pub fn draw_entities(g: &mut Game, i: i32, i2: i32) {
    // Entity ckVar = this.entities.head;
    let mut cursor = g
        .game_state
        .map
        .as_ref()
        .expect("GameState.map null in drawEntities")
        .entities
        .head;
    // while (ckVar != null) { ckVar.paint(graphics, i, i2); ckVar = ckVar.next; }
    while let Some(id) = cursor {
        let next = g.entity_arena[id].next;
        match g.entity_arena[id].kind() {
            // ckVar.paint(graphics, i, i2)  — Hero.paint (`ao.a:(…Graphics;II)V`).
            EntityKind::Hero => hero::paint(g, id, i, i2),
            // (MapObject / Npc / Enemy paints DEFERRED — none are linked in this slice.)
            EntityKind::MapObject | EntityKind::Bare => {}
        }
        cursor = next;
    }
}

/// `graphics.setColor(0); graphics.fillRect(0, 0, w, h);` — the small-map letterbox
/// fill from [`paint`], factored so the framebuffer is borrowed only when it fires.
fn black_fill(g: &mut Game, w: i32, h: i32) {
    let target = g.screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);
    graphics.set_color(0);
    graphics.fill_rect(0, 0, w, h);
}

/// `private final void drawTiles(Graphics graphics, int i, int i2, int i3, int i4)`
/// — draws the visible 16px tile window from `AssetCache.mapTiles`, anchored `20`
/// (TOP|LEFT).
///
/// The Java caller ([`paint`]) sets `graphics.setClip(0, 0, i3, i4)` before this
/// call and relies on it persisting; each `j2me-me` `Graphics` acquisition starts
/// with a full-image clip, so the identical clip rect is re-established here — same
/// pixels, one structural move of the `setClip`. `allImages` fills every atlas frame
/// and every tile index is a valid frame, so the Java `image == null` collision-debug
/// branch (which would read the DEFERRED `.evt` `collisionGrid`) is unreachable here.
pub fn draw_tiles(g: &mut Game, i: i32, i2: i32, i3: i32, i4: i32) {
    // int i5 = (-i)/16; i6 = (-i2)/16; i7 = ((i3-i)-1)/16; i8 = ((i4-i2)-1)/16;
    let mut i5 = java_div(i.wrapping_neg(), 16).expect("(-i) / 16");
    let mut i6 = java_div(i2.wrapping_neg(), 16).expect("(-i2) / 16");
    let mut i7 = java_div(i3.wrapping_sub(i).wrapping_sub(1), 16).expect("((i3 - i) - 1) / 16");
    let mut i8 = java_div(i4.wrapping_sub(i2).wrapping_sub(1), 16).expect("((i4 - i2) - 1) / 16");
    let Game {
        screen,
        game_state,
        asset_cache,
        ..
    } = &mut *g;
    let map = game_state
        .map
        .as_ref()
        .expect("GameState.map null in drawTiles");
    let width_tiles = map.width_tiles;
    let height_tiles = map.height_tiles;
    // if (i5 < 0) i5 = 0; if (i6 < 0) i6 = 0;
    if i5 < 0 {
        i5 = 0;
    }
    if i6 < 0 {
        i6 = 0;
    }
    // if (i7 >= widthTiles) i7 = widthTiles - 1; if (i8 >= heightTiles) i8 = heightTiles - 1;
    if i7 >= width_tiles {
        i7 = width_tiles.wrapping_sub(1);
    }
    if i8 >= height_tiles {
        i8 = height_tiles.wrapping_sub(1);
    }
    let tile_grid = map.tile_grid.as_ref().expect("tileGrid null in drawTiles");
    let map_tiles = asset_cache
        .map_tiles
        .as_ref()
        .expect("AssetCache.mapTiles null in drawTiles");
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);
    // (paint's clip, re-established — see the doc note.)
    graphics.set_clip(0, 0, i3, i4);
    // for (int i9 = i6; i9 <= i8; i9++) { int i10 = i2 + i9*16; int i11 = i + i5*16;
    let mut i9 = i6;
    while i9 <= i8 {
        let i10 = i2.wrapping_add(i9.wrapping_mul(16));
        let mut i11 = i.wrapping_add(i5.wrapping_mul(16));
        // for (int i12 = i5; i12 <= i7; i12++) {
        let mut i12 = i5;
        while i12 <= i7 {
            // Image image = imageArr[this.tileGrid[i9][i12]];  (byte index sign-extends)
            let idx = tile_grid[i9 as usize][i12 as usize] as i32;
            // (image is never null here → always drawImage; collision-debug branch dead.)
            // graphics.drawImage(image, i11, i10, 20);
            graphics
                .draw_image(&map_tiles[idx as usize], i11, i10, 20)
                .expect("drawImage(mapTiles[tile])");
            // i11 += 16;
            i11 = i11.wrapping_add(16);
            i12 = i12.wrapping_add(1);
        }
        i9 = i9.wrapping_add(1);
    }
}
