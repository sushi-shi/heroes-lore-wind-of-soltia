//! Transliterated from `java/src/main/java/defpackage/GebHandLeft.java`
//! (original `ba.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Left hand of the three-part **Geb** encounter (`GebHandLeft extends Boss`,
//! enemy-data record 40, size 2, anchored at column 0), owned by the
//! [`crate::geb_head`] core. It slides vertically to track the hero (`state == 2` bobs
//! its pixel row) and, when aligned, slams a 4x6 box that also shakes the camera. A
//! two-tick attack delay is dropped once it has idled long enough (`ticksSinceHit > 100`)
//! and re-armed after each landed slam. It paints 80px high (tall sprite), always facing
//! down.
//!
//! ## Entity model — [`crate::boss::BossSubclass::GebHandLeft`]
//!
//! A `GebHandLeft` is a [`crate::entity::EntityData::Boss`] node tagged
//! [`crate::boss::BossSubclass::GebHandLeft`] carrying [`GebHandLeftData`]
//! (`ticksSinceHit`). It overrides `update`/`paint`/`tryAttack`/`resolveAttack`/
//! `onDeath`; `updateAi`/`animate`/`chase`/`stepDeathAnim`/`die` are the inherited
//! `Boss`/`Enemy` bodies reached through the [`crate::boss`] dispatchers.
//!
//! ## The `stats.attackDelay` mutation (clone-deviation note)
//!
//! The constructor and [`update`]/[`resolve_attack`] mutate `((Enemy) this).stats.attackDelay`
//! at runtime. Java mutates the *shared* `EnemyType.types[40]` template; the port gives
//! each enemy a per-instance clone of `stats` (see [`crate::enemy_type`]). For this hand
//! the two are observably identical — record 40 has a single live instance in the boss
//! arena, so no other reader sees the mutation — consistent with the recorded
//! clone-deviation.
//!
//! ## DEFERRED cross-class boundaries
//!
//! - **`Hero.takeHit`.** The landed slam is DEFERRED (hero combat FSM partial), as in
//!   [`crate::enemy`]; the surrounding camera shake, box check, and cooldown re-arm ARE
//!   applied.
//!
//! Opcode shapes (R8): `ba.<init>:(Lae;BBBB)V => ["iadd","i2b","iadd","iadd","iadd"]`,
//! `ba.d:()V (update) => ["iadd","i2b","iadd","i2b","isub","i2s","iadd","i2s"]`,
//! `ba.a:(…Graphics;II)V (paint) => ["iinc"]`,
//! `ba.i:()V (tryAttack) => ["isub","iadd","isub","iadd"]`,
//! `ba.j:()V (resolveAttack) => ["iadd","i2b","iadd","i2b","isub","iadd","i2b","isub","iadd","i2b"]`,
//! `ba.m:()V (onDeath) => []`.

use crate::battler;
use crate::boss::{self, BossSubclass};
use crate::enemy;
use crate::entity::{self, EntityId, EntityNode};
use crate::game::Game;

/// The `GebHandLeft` (`ba`) instance fields beyond the `Boss`/`Enemy` base — carried in
/// [`crate::boss::BossSubclass::GebHandLeft`].
#[derive(Debug)]
pub struct GebHandLeftData {
    /// `private byte ticksSinceHit;` (`ba.v`) — ticks since the last landed slam; once
    /// past 100 the attack delay is removed.
    pub ticks_since_hit: i8,
}

/// The [`GebHandLeftData`] behind a `GebHandLeft` node.
fn data(node: &EntityNode) -> &GebHandLeftData {
    match &node.as_boss().expect("GebHandLeft node is a Boss").subclass {
        BossSubclass::GebHandLeft(d) => d,
        _ => unreachable!("geb_hand_left dispatched on a non-GebHandLeft node"),
    }
}

/// Mutable [`data`].
fn data_mut(node: &mut EntityNode) -> &mut GebHandLeftData {
    match &mut node
        .as_boss_mut()
        .expect("GebHandLeft node is a Boss")
        .subclass
    {
        BossSubclass::GebHandLeft(d) => d,
        _ => unreachable!("geb_hand_left dispatched on a non-GebHandLeft node"),
    }
}

/// `public GebHandLeft(GameMap map, byte tileX, byte tileY, byte kind, byte statRow)`
/// (`ba.<init>:(Lae;BBBB)V => ["iadd","i2b","iadd","iadd","iadd"]`). `super(tileX,
/// (byte)(tileY+5), kind, statRow, (byte) 2)` is [`boss::new_boss`] (anchored five rows
/// down, layer 2); the node is tagged, the two spawn cells are cleared, and the short
/// attack delay is armed.
pub fn new(g: &mut Game, tile_x: i8, tile_y: i8, kind: i8, stat_row: i8) -> EntityId {
    // super(tileX, (byte) (tileY + 5), kind, statRow, (byte) 2);
    let super_tile_y = (tile_y as i32).wrapping_add(5) as i8;
    let id = boss::new_boss(g, tile_x, super_tile_y, kind, stat_row, 2);
    // (tag; ticksSinceHit at JVM byte default 0.)
    g.entity_arena[id]
        .as_boss_mut()
        .expect("GebHandLeft node is a Boss")
        .subclass = BossSubclass::GebHandLeft(GebHandLeftData { ticks_since_hit: 0 });
    // map.occupancy[tileY + 5][tileX] = null; map.occupancy[tileY + 5][tileX + 1] = null;
    {
        let map = g
            .game_state
            .map
            .as_mut()
            .expect("GameState.map null in GebHandLeft ctor");
        let occ = map.occupancy.as_mut().expect("occupancy null");
        let r = (tile_y as i32).wrapping_add(5) as usize;
        occ[r][tile_x as usize] = None;
        occ[r][(tile_x as i32).wrapping_add(1) as usize] = None;
    }
    // ((Enemy) this).stats.attackDelay = (byte) 2;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("GebHandLeft enemy")
        .stats
        .attack_delay = 2;
    // this.ticksSinceHit = (byte) 0;  (already 0)
    id
}

/// `public final void update()` (`ba.d:()V`, overriding `Boss.update`) — advance the
/// frame + idle counter, drop the attack delay once idle, run the AI, bob the pixel row
/// while stepping (state 2), then animate.
pub fn update(g: &mut Game, id: EntityId) {
    // this.animFrame = (byte) (this.animFrame + 1);
    {
        let e = g.entity_arena[id]
            .as_enemy_mut()
            .expect("GebHandLeft enemy");
        e.battler.anim_frame = (e.battler.anim_frame as i32).wrapping_add(1) as i8;
    }
    // this.ticksSinceHit = (byte) (this.ticksSinceHit + 1);
    {
        let d = data_mut(&mut g.entity_arena[id]);
        d.ticks_since_hit = (d.ticks_since_hit as i32).wrapping_add(1) as i8;
    }
    // if (this.ticksSinceHit > 100) ((Enemy) this).stats.attackDelay = (byte) 0;
    let ticks = data(&g.entity_arena[id]).ticks_since_hit;
    if ticks as i32 > 100 {
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("GebHandLeft enemy")
            .stats
            .attack_delay = 0;
    }
    // updateAi();
    boss::update_ai(g, id);
    // if (this.state == 2) { pixelY bob; syncTile(); }
    let (state, facing) = {
        let e = g.entity_arena[id].as_enemy().expect("GebHandLeft enemy");
        (e.battler.state, e.battler.facing)
    };
    if state == 2 {
        match facing {
            // case 1: pixelY = (short) (pixelY - 8);
            1 => {
                let n = &mut g.entity_arena[id];
                n.pixel_y = (n.pixel_y as i32).wrapping_sub(8) as i16;
            }
            // case 2: pixelY = (short) (pixelY + 8);
            2 => {
                let n = &mut g.entity_arena[id];
                n.pixel_y = (n.pixel_y as i32).wrapping_add(8) as i16;
            }
            _ => {}
        }
        // syncTile();
        entity::sync_tile(&mut g.entity_arena[id]);
    }
    // animate();
    boss::animate(g, id);
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`ba.a:(…Graphics;II)V => ["iinc"]`, overriding `Boss.paint`) — draw the tall sprite
/// lifted 80px up-screen, forced to face down.
pub fn paint(g: &mut Game, id: EntityId, origin_x: i32, origin_y: i32) {
    // byte savedMoveDir = this.moveDir; this.moveDir = (byte) 1;
    let saved = g.entity_arena[id]
        .as_battler()
        .expect("GebHandLeft battler")
        .move_dir;
    g.entity_arena[id]
        .as_battler_mut()
        .expect("GebHandLeft battler")
        .move_dir = 1;
    // super.paint(graphics, originX, originY - 80);
    boss::paint_base(g, id, origin_x, origin_y.wrapping_sub(80));
    // this.moveDir = savedMoveDir;
    g.entity_arena[id]
        .as_battler_mut()
        .expect("GebHandLeft battler")
        .move_dir = saved;
}

/// `public final void tryAttack()` (`ba.i:()V => ["isub","iadd","isub","iadd"]`,
/// overriding `Enemy.tryAttack`) — slam when the hero is aligned in front, else slide up
/// / down / hold to line up on the hero's row.
// The `heroRowOffset >= -2 && heroRowOffset <= 3` bound is the faithful Java pair.
#[allow(clippy::manual_range_contains)]
pub fn try_attack(g: &mut Game, id: EntityId) {
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in GebHandLeft.tryAttack");
    let (hero_tx, hero_ty) = {
        let n = &g.entity_arena[hero];
        (n.tile_x as i32, n.tile_y as i32)
    };
    let (tile_x, tile_y) = {
        let n = &g.entity_arena[id];
        (n.tile_x as i32, n.tile_y as i32)
    };
    // int heroRowOffset = ((Entity) hero).tileY - ((((Entity) this).tileY - 5) + 3);
    let hero_row_offset = hero_ty.wrapping_sub(tile_y.wrapping_sub(5).wrapping_add(3));
    let (hurt_cooldown, attack_cooldown) = {
        let e = g.entity_arena[id].as_enemy().expect("GebHandLeft enemy");
        (e.hurt_cooldown, e.attack_cooldown)
    };
    // if (hurtCooldown == 0 && heroRowOffset >= -2 && heroRowOffset <= 3 && hero.tileX <= tileX + 5) { beginAttack(); return; }
    if hurt_cooldown == 0
        && hero_row_offset >= -2
        && hero_row_offset <= 3
        && hero_tx <= tile_x.wrapping_add(5)
    {
        enemy::begin_attack(g, id);
        return;
    }
    // if (attackCooldown == 0) { ... }
    if attack_cooldown == 0 {
        if hero_row_offset > 3 {
            // setState((byte) 2); setFacing((byte) 2);
            set_state(g, id, 2);
            set_facing(g, id, 2);
        } else if hero_row_offset < -2 {
            // setState((byte) 2); setFacing((byte) 1);
            set_state(g, id, 2);
            set_facing(g, id, 1);
        } else {
            // setState((byte) 1); setFacing((byte) 2);
            set_state(g, id, 1);
            set_facing(g, id, 2);
        }
    }
}

/// `Enemy.setState` for this hand (no `animFrame` reset — `Boss`/`GebHandLeft` do not
/// override it): set the FSM state directly.
fn set_state(g: &mut Game, id: EntityId, new_state: i8) {
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("GebHandLeft enemy")
        .battler
        .state = new_state;
}

/// `Battler.setFacing(dir)` — face `dir` and commit it as the step direction.
fn set_facing(g: &mut Game, id: EntityId, dir: i8) {
    let b = g.entity_arena[id]
        .as_battler_mut()
        .expect("GebHandLeft battler");
    battler::set_facing(b, dir);
}

/// `public final void resolveAttack()` (`ba.j:()V`, overriding `Enemy.resolveAttack`) —
/// the multi-frame camera shake + the frame-5 slam over a 4x6 box.
pub fn resolve_attack(g: &mut Game, id: EntityId) {
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in GebHandLeft.resolveAttack");
    let anim_frame = g.entity_arena[id]
        .as_enemy()
        .expect("GebHandLeft enemy")
        .battler
        .anim_frame;
    // if (animFrame == 6) { shift 2,3 } else if (7) { shift -3,-1 } else if (8) { shift 2,-3 }
    if anim_frame == 6 {
        let map = g.game_state.map.as_mut().expect("map null");
        map.camera_shift_x = 2;
        map.camera_shift_y = 3;
    } else if anim_frame == 7 {
        let map = g.game_state.map.as_mut().expect("map null");
        map.camera_shift_x = -3;
        map.camera_shift_y = -1;
    } else if anim_frame == 8 {
        let map = g.game_state.map.as_mut().expect("map null");
        map.camera_shift_x = 2;
        map.camera_shift_y = -3;
    }
    // if (animFrame == 5) { slam box; if miss return; hero.takeHit; ticksSinceHit=0; attackDelay=2; }
    if anim_frame == 5 {
        let (tile_x, tile_y) = {
            let n = &g.entity_arena[id];
            (n.tile_x as i32, n.tile_y as i32)
        };
        // byte slamMinX = (byte)(tileX+2); slamMaxX = (byte)(tileX+5);
        // byte slamMinY = (byte)((tileY-5)+1); slamMaxY = (byte)((tileY-5)+6);
        let slam_min_x = tile_x.wrapping_add(2) as i8;
        let slam_max_x = tile_x.wrapping_add(5) as i8;
        let slam_min_y = tile_y.wrapping_sub(5).wrapping_add(1) as i8;
        let slam_max_y = tile_y.wrapping_sub(5).wrapping_add(6) as i8;
        let (hero_tx, hero_ty) = {
            let n = &g.entity_arena[hero];
            (n.tile_x, n.tile_y)
        };
        // if (hero.tileX < slamMinX || hero.tileX > slamMaxX || hero.tileY < slamMinY || hero.tileY > slamMaxY) return;
        if (hero_tx as i32) < (slam_min_x as i32)
            || (hero_tx as i32) > (slam_max_x as i32)
            || (hero_ty as i32) < (slam_min_y as i32)
            || (hero_ty as i32) > (slam_max_y as i32)
        {
            return;
        }
        // hero.takeHit((Enemy) this, (byte) 2);   — DEFERRED: Hero.takeHit (hero FSM partial).
        // this.ticksSinceHit = (byte) 0;
        data_mut(&mut g.entity_arena[id]).ticks_since_hit = 0;
        // ((Enemy) this).stats.attackDelay = (byte) 2;
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("GebHandLeft enemy")
            .stats
            .attack_delay = 2;
    }
}

/// `public final void onDeath()` (`ba.m:()V => []`, overriding the abstract
/// `Boss.onDeath`) — despawn immediately (no death-animation delay).
pub fn on_death(g: &mut Game, id: EntityId) {
    // this.deathTimer = (byte) 0;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("GebHandLeft enemy")
        .death_timer = 0;
}
