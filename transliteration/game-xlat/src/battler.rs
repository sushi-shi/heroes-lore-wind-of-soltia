//! Transliterated from `java/src/main/java/defpackage/Battler.java`
//! (original `o.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Abstract base for every mobile combatant that walks the tile grid: the player
//! ([`crate::hero`]), hostile actors (Enemy/Boss) and town folk (Npc). On top of
//! [`crate::entity::EntityNode`] (pixel/tile position + list links) it adds the
//! movement/animation finite-state machine ([`BattlerData::state`],
//! [`BattlerData::facing`], [`BattlerData::move_dir`], [`BattlerData::anim_frame`]),
//! plus two per-actor overlay lists (floating combat text and active status
//! effects). Tiles are 16px and actors step 8px at a time (a tile takes two
//! frames); `offGridX`/`offGridY` on the base track the half-tile phase.
//!
//! **This slice ports the FIELD LAYER + `setState`/`setFacing`/`setOccupancy`.**
//! [`BattlerData`] + the FSM `state` constants + [`set_occupancy`] (writing the
//! actor's footprint into the map occupancy grid, needed by the map warp) land
//! here; the per-tick FSM ([`update`]/[`move_`]/[`step_if_moving`]/[`approach`] and
//! the AI helpers) is DEFERRED to a later world-logic lane. `Battler` has no
//! `static` fields (no `ownership.tsv` rows).
//!
//! Opcode shapes (R8): `o.<init>:(SSBB)V => []`, `o.a:()V (init) => []`.

use crate::debug;
use crate::entity::{self, EntityId};
use crate::game::Game;

// --- FSM state constants (`BattlerData::state`) ------------------------------
// From `Battler.state` (`o.h`): "1 idle/walk, 2 stepping, 3 attacking, 4
// knockback, 5 dying, 6 dead".
/// `state == 1` — idle / walking in place.
pub const STATE_IDLE: i8 = 1;
/// `state == 2` — mid sub-tile step.
pub const STATE_STEPPING: i8 = 2;
/// `state == 3` — attacking.
pub const STATE_ATTACKING: i8 = 3;
/// `state == 4` — knockback / recoil.
pub const STATE_KNOCKBACK: i8 = 4;
/// `state == 5` — dying.
pub const STATE_DYING: i8 = 5;
/// `state == 6` — dead.
pub const STATE_DEAD: i8 = 6;

/// DEFERRED placeholder for a `Floater`/`Overlay` (`y`/`bb`) heap object — those
/// classes are not ported. The list is empty in this slice (never pushed).
#[derive(Debug)]
pub struct OverlayRef;

/// DEFERRED placeholder for a `StatusIcon` (`bp`) heap object — not ported. The
/// list is empty in this slice.
#[derive(Debug)]
pub struct StatusIconRef;

/// The `Battler` (`o`) base fields — the "super" of every combatant subclass.
/// Embedded in each subclass's data (e.g. [`crate::hero::HeroData::battler`]).
#[derive(Debug)]
pub struct BattlerData {
    /// `public Vector floaters;` (`o.a`) — floating-text overlays on this actor
    /// (`new Vector(2)`; DEFERRED element type — empty in this slice).
    pub floaters: Vec<OverlayRef>,
    /// `public Vector statuses;` (`o.b`) — active status-effect icons
    /// (`new Vector(3)`; DEFERRED element type — empty in this slice).
    pub statuses: Vec<StatusIconRef>,
    /// `public byte state;` (`o.h`) — FSM state (see the `STATE_*` constants).
    pub state: i8,
    /// `public byte facing;` (`o.i`) — 1 up, 2 down, 3 left, 4 right (`Directions`).
    pub facing: i8,
    /// `public byte moveDir;` (`o.j`) — direction committed for the current step.
    pub move_dir: i8,
    /// `public byte animFrame;` (`o.k`) — animation frame counter (starts at -1).
    pub anim_frame: i8,
    /// `public byte knockbackTimer;` (`o.l`) — knockback countdown (state 4).
    pub knockback_timer: i8,
}

impl BattlerData {
    /// The `Battler(short,short,byte,byte)` constructor's effect on the base fields:
    /// `knockbackTimer = 0; init();`. `init()` (`o.a:()V`) allocates the empty
    /// overlay lists and sets `state = 1; facing = 2; moveDir = 2; animFrame = -1`.
    /// (Virtual dispatch means a `Hero` object routes this to `Hero.init`; the
    /// Hero-specific fields it also touches are applied by
    /// [`crate::hero::new_hero`], where the net end-state is assembled.)
    pub fn new() -> BattlerData {
        BattlerData {
            // init(): this.floaters = new Vector(2); this.statuses = new Vector(3);
            floaters: Vec::new(),
            statuses: Vec::new(),
            // init(): this.state = 1; this.facing = 2; this.moveDir = 2;
            state: STATE_IDLE,
            facing: 2,
            move_dir: 2,
            // init(): this.animFrame = -1;
            anim_frame: -1,
            // constructor: this.knockbackTimer = 0;
            knockback_timer: 0,
        }
    }
}

impl Default for BattlerData {
    fn default() -> Self {
        Self::new()
    }
}

/// `public void setState(byte newState)` (`o.a:(B)V`). Enters `new_state`, resetting
/// the animation frame counter.
pub fn set_state(b: &mut BattlerData, new_state: i8) {
    // this.animFrame = (byte) -1;
    b.anim_frame = -1;
    // this.state = newState;
    b.state = new_state;
}

/// `public final void setFacing(byte dir)` (`o.b:(B)V`). Faces `dir` and commits it
/// as the next step direction.
pub fn set_facing(b: &mut BattlerData, dir: i8) {
    // this.facing = dir; this.moveDir = dir;
    b.facing = dir;
    b.move_dir = dir;
}

/// `public final void setOccupancy()` (`o.g:()V`). Writes this actor's footprint
/// (`layer` tiles wide, plus the half-tile spill when off-grid) into the map
/// occupancy grid. Reads the entity base fields and writes `GameState.map.occupancy`;
/// modelled on `&mut Game` because it spans the arena and the map.
pub fn set_occupancy(g: &mut Game, id: EntityId) {
    // GameMap map = GameState.map;  (fields read off the Entity base up front)
    let layer = g.entity_arena[id].layer;
    let tile_x = g.entity_arena[id].tile_x as i32;
    let tile_y = g.entity_arena[id].tile_y as i32;
    let off_grid_x = g.entity_arena[id].off_grid_x;
    let off_grid_y = g.entity_arena[id].off_grid_y;
    let map = g
        .game_state
        .map
        .as_mut()
        .expect("GameState.map null in setOccupancy");
    let occupancy = map.occupancy.as_mut().expect("occupancy null");
    // byte colOffset = 0; while (col < layer) { ... col++; }
    let mut col_offset: i8 = 0;
    loop {
        // byte col = colOffset; if (col >= this.layer) return;
        let col = col_offset;
        if col >= layer {
            return;
        }
        // map.occupancy[tileY][tileX + col] = this;
        occupancy[tile_y as usize][(tile_x.wrapping_add(col as i32)) as usize] = Some(id);
        // if (offGridY) map.occupancy[tileY + 1][tileX + col] = this;
        if off_grid_y {
            occupancy[(tile_y.wrapping_add(1)) as usize]
                [(tile_x.wrapping_add(col as i32)) as usize] = Some(id);
        // else if (offGridX) map.occupancy[tileY][tileX + 1 + col] = this;
        } else if off_grid_x {
            occupancy[tile_y as usize]
                [(tile_x.wrapping_add(1).wrapping_add(col as i32)) as usize] = Some(id);
        }
        // colOffset = (byte) (col + 1);
        col_offset = (col as i32).wrapping_add(1) as i8;
    }
}

/// `public final void clearOccupancy()` (`o.f:()V => [iadd×6, i2b]`). Clears this
/// actor's footprint (`layer` tiles wide, plus the half-tile spill when off-grid)
/// from the map occupancy grid. Mirror of [`set_occupancy`] writing `null`.
pub fn clear_occupancy(g: &mut Game, id: EntityId) {
    // GameMap map = GameState.map;  (fields read off the Entity base up front)
    let layer = g.entity_arena[id].layer;
    let tile_x = g.entity_arena[id].tile_x as i32;
    let tile_y = g.entity_arena[id].tile_y as i32;
    let off_grid_x = g.entity_arena[id].off_grid_x;
    let off_grid_y = g.entity_arena[id].off_grid_y;
    let map = g
        .game_state
        .map
        .as_mut()
        .expect("GameState.map null in clearOccupancy");
    let occupancy = map.occupancy.as_mut().expect("occupancy null");
    // byte colOffset = 0; while (col < layer) { ... col++; }
    let mut col_offset: i8 = 0;
    loop {
        // byte col = colOffset; if (col >= this.layer) return;
        let col = col_offset;
        if col >= layer {
            return;
        }
        // map.occupancy[tileY][tileX + col] = null;
        occupancy[tile_y as usize][(tile_x.wrapping_add(col as i32)) as usize] = None;
        // if (offGridY) map.occupancy[tileY + 1][tileX + col] = null;
        if off_grid_y {
            occupancy[(tile_y.wrapping_add(1)) as usize]
                [(tile_x.wrapping_add(col as i32)) as usize] = None;
        // else if (offGridX) map.occupancy[tileY][tileX + 1 + col] = null;
        } else if off_grid_x {
            occupancy[tile_y as usize]
                [(tile_x.wrapping_add(1).wrapping_add(col as i32)) as usize] = None;
        }
        // colOffset = (byte) (col + 1);
        col_offset = (col as i32).wrapping_add(1) as i8;
    }
}

/// `public void update()` (`o.d:()V => []`) — per-tick update (base: advance any
/// in-progress step). `Hero` overrides this with its own FSM (see [`crate::hero::update`]),
/// so this base body is the one Enemy/Npc use.
pub fn update(g: &mut Game, id: EntityId) {
    // stepIfMoving();
    step_if_moving(g, id);
}

/// `public final void stepIfMoving()` (`o.e:()V => []`) — while stepping (state 2/4),
/// halt at a blocked tile then advance 8px.
pub fn step_if_moving(g: &mut Game, id: EntityId) {
    // if (this.state == 2 || this.state == 4) tryStepForward();
    let state = g.entity_arena[id].as_battler().expect("Battler node").state;
    if state == STATE_STEPPING || state == STATE_KNOCKBACK {
        try_step_forward(g, id);
    }
    // if (this.state == 2 || this.state == 4) move(8);
    let state = g.entity_arena[id].as_battler().expect("Battler node").state;
    if state == STATE_STEPPING || state == STATE_KNOCKBACK {
        move_(g, id, 8);
    }
}

/// `public boolean tryStepForward()` (`o.a:()Z => []`). If aligned to the grid and
/// the tile ahead is blocked, halts (state 1) and reports `true`; otherwise keeps
/// moving and reports `false`.
///
/// **DEFERRED collision.** The block test `map.canStep(this, facing)` reads the
/// map's collision grid, which comes from the not-yet-parsed `/m/<classId>/<NN>.evt`
/// (see [`crate::game_map::load`]). Collision is stubbed to *never blocked*: with
/// `canStep` treated as always `true`, the guard `offGridX || offGridY || canStep`
/// is always satisfied, so the method always returns `false` (never halts). This
/// lets the hero walk the tile grid; collision fidelity is a later lane's concern.
pub fn try_step_forward(_g: &mut Game, _id: EntityId) -> bool {
    // GameMap map = GameState.map;
    // if (offGridX || offGridY || map.canStep(this, facing)) return false;  (always, stubbed)
    // setState((byte) 1); return true;   — unreachable while collision is stubbed.
    false
}

/// `public void move(int stepPixels)` (`o.a:(I)V => [isub×6, i2s×4, iadd×4, i2b×4]`).
/// Advances the actor `stepPixels` in its facing direction (1 up, 2 down, 3 left,
/// 4 right), toggling the off-grid half-tile flags and re-registering occupancy. A
/// plain 8px sub-tile step skips [`entity::sync_tile`] (the flags are updated by
/// hand here); any other step re-derives the tile from pixels. The four
/// `Debug.assertTrue` bounds checks are preserved (the down/right forms' `- 16`
/// account for two of the six `isub`s).
pub fn move_(g: &mut Game, id: EntityId, step_pixels: i32) {
    // clearOccupancy();
    clear_occupancy(g, id);
    // switch (this.facing)
    let facing = g.entity_arena[id]
        .as_battler()
        .expect("Battler node")
        .facing;
    match facing {
        // case 1 (up):
        1 => {
            // Debug.assertTrue(this.pixelY > 0);
            debug::assert_true(g.entity_arena[id].pixel_y as i32 > 0);
            let node = &mut g.entity_arena[id];
            // this.pixelY = (short) (this.pixelY - stepPixels);
            node.pixel_y = (node.pixel_y as i32).wrapping_sub(step_pixels) as i16;
            // if (!offGridY) { offGridY = true; tileY = (byte)(tileY - 1); } else offGridY = false;
            if !node.off_grid_y {
                node.off_grid_y = true;
                node.tile_y = (node.tile_y as i32).wrapping_sub(1) as i8;
            } else {
                node.off_grid_y = false;
            }
        }
        // case 2 (down):
        2 => {
            // Debug.assertTrue(this.pixelY < GameState.map.heightPx - 16);
            let height_px = g
                .game_state
                .map
                .as_ref()
                .expect("GameState.map null in move")
                .height_px;
            debug::assert_true((g.entity_arena[id].pixel_y as i32) < height_px.wrapping_sub(16));
            let node = &mut g.entity_arena[id];
            // this.pixelY = (short) (this.pixelY + stepPixels);
            node.pixel_y = (node.pixel_y as i32).wrapping_add(step_pixels) as i16;
            // if (!offGridY) offGridY = true; else { offGridY = false; tileY = (byte)(tileY + 1); }
            if !node.off_grid_y {
                node.off_grid_y = true;
            } else {
                node.off_grid_y = false;
                node.tile_y = (node.tile_y as i32).wrapping_add(1) as i8;
            }
        }
        // case 3 (left):
        3 => {
            // Debug.assertTrue(this.pixelX > 0);
            debug::assert_true(g.entity_arena[id].pixel_x as i32 > 0);
            let node = &mut g.entity_arena[id];
            // this.pixelX = (short) (this.pixelX - stepPixels);
            node.pixel_x = (node.pixel_x as i32).wrapping_sub(step_pixels) as i16;
            // if (!offGridX) { offGridX = true; tileX = (byte)(tileX - 1); } else offGridX = false;
            if !node.off_grid_x {
                node.off_grid_x = true;
                node.tile_x = (node.tile_x as i32).wrapping_sub(1) as i8;
            } else {
                node.off_grid_x = false;
            }
        }
        // case 4 (right):
        4 => {
            // Debug.assertTrue(this.pixelX < GameState.map.widthPx - 16);
            let width_px = g
                .game_state
                .map
                .as_ref()
                .expect("GameState.map null in move")
                .width_px;
            debug::assert_true((g.entity_arena[id].pixel_x as i32) < width_px.wrapping_sub(16));
            let node = &mut g.entity_arena[id];
            // this.pixelX = (short) (this.pixelX + stepPixels);
            node.pixel_x = (node.pixel_x as i32).wrapping_add(step_pixels) as i16;
            // if (!offGridX) offGridX = true; else { offGridX = false; tileX = (byte)(tileX + 1); }
            if !node.off_grid_x {
                node.off_grid_x = true;
            } else {
                node.off_grid_x = false;
                node.tile_x = (node.tile_x as i32).wrapping_add(1) as i8;
            }
        }
        _ => {}
    }
    // if (stepPixels != 8) syncTile();
    if step_pixels != 8 {
        entity::sync_tile(&mut g.entity_arena[id]);
    }
    // setOccupancy();
    set_occupancy(g, id);
}

/// `public final void approach(Entity target, byte range)` (`o.a:(Lck;B)V`) — the
/// AI path-choice that steers toward a target with side-step fallbacks. DEFERRED
/// (enemy AI; not on the player-movement path).
pub fn approach(_g: &mut Game, _id: EntityId, _target: EntityId, _range: i8) {
    unimplemented!("DEFERRED: Battler.approach — not ported in this slice")
}
