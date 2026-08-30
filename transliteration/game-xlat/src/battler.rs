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

use crate::entity::EntityId;
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

// --- DEFERRED: the per-tick movement/AI FSM ----------------------------------
// The following bodies reach the map occupancy grid, `Directions`, `GameState.map`,
// `GameLoop.gameScreen`, and the enemy/status machinery; they are DEFERRED to a
// later world-logic lane. Signatures name the real dependency (`&mut Game`).

/// `public void update()` (`o.d:()V`) — per-tick update (base: advance any
/// in-progress step). DEFERRED.
pub fn update(_g: &mut Game, _id: EntityId) {
    unimplemented!("DEFERRED: Battler.update — not ported in this slice")
}

/// `public void move(int stepPixels)` (`o.a:(I)V`) — advance in the facing
/// direction, toggling the off-grid flags and re-registering occupancy. DEFERRED.
pub fn move_(_g: &mut Game, _id: EntityId, _step_pixels: i32) {
    unimplemented!("DEFERRED: Battler.move — not ported in this slice")
}

/// `public final void stepIfMoving()` (`o.e:()V`) — while stepping, halt at a
/// blocked tile then advance 8px. DEFERRED.
pub fn step_if_moving(_g: &mut Game, _id: EntityId) {
    unimplemented!("DEFERRED: Battler.stepIfMoving — not ported in this slice")
}

/// `public final void approach(Entity target, byte range)` (`o.a:(Lck;B)V`) — the
/// AI path-choice that steers toward a target with side-step fallbacks. DEFERRED.
pub fn approach(_g: &mut Game, _id: EntityId, _target: EntityId, _range: i8) {
    unimplemented!("DEFERRED: Battler.approach — not ported in this slice")
}
