//! Transliterated from `java/src/main/java/defpackage/GebHandRight.java`
//! (original `ak.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Right hand of the three-part **Geb** encounter (`GebHandRight extends Boss`,
//! enemy-data record 41, size 1, anchored at column 13), owned by the
//! [`crate::geb_head`] core. Like the left hand it slides vertically to line up on the
//! hero, but it alternates two sweep attacks — a long reach connecting on frame 5
//! (`swingSide == 1`) and a shorter reach on frame 8 (`swingSide == 2`) — flipping the
//! side each swing through the direction turn-table. Its retract (`state == 3`) plays out
//! the attack sprite before returning to idle. It paints 64px high.
//!
//! ## Entity model — [`crate::boss::BossSubclass::GebHandRight`]
//!
//! A `GebHandRight` is a [`crate::entity::EntityData::Boss`] node tagged
//! [`crate::boss::BossSubclass::GebHandRight`] carrying [`GebHandRightData`]
//! (`ticksSinceHit`, `swingSide`). It overrides `update`/`paint`/`tryAttack`/
//! `resolveAttack`/`onDeath`; `updateAi`/`animate`/`chase`/`stepDeathAnim`/`die` are the
//! inherited `Boss`/`Enemy` bodies reached through the [`crate::boss`] dispatchers.
//! The `stats.attackDelay` runtime mutation is the clone-deviation noted in
//! [`crate::geb_hand_left`] (record 41's single live instance makes it observably
//! identical).
//!
//! ## DEFERRED cross-class boundaries
//!
//! - **`Hero.takeHit`.** The landed sweeps are DEFERRED (hero combat FSM partial), as in
//!   [`crate::enemy`]; the box checks + cooldown re-arm ARE applied.
//! - **`AssetCache.bossFrames` bank.** [`update`]'s `state == 3` retract reads the
//!   selected attack script's frame count from the DEFERRED-loaded `bossFrames` bank (a
//!   null element NPEs, faithfully — the bank is loaded before the boss acts in real
//!   play), mirroring [`crate::enemy`].
//!
//! Opcode shapes (R8): `ak.<init>:(Lae;BBBB)V => ["iadd","i2b","iadd","iadd","iadd"]`,
//! `ak.d:()V (update) => ["iadd","i2b","iadd","i2b","imul","iadd","isub","iadd","isub","i2s","iadd","i2s"]`,
//! `ak.a:(…Graphics;II)V (paint) => ["iinc"]`,
//! `ak.i:()V (tryAttack) => ["isub","iadd","isub","isub"]`,
//! `ak.j:()V (resolveAttack) => [twin hit-box arithmetic]`, `ak.m:()V (onDeath) => []`.

use crate::battler;
use crate::boss::{self, BossSubclass};
use crate::directions::REVERSE;
use crate::enemy;
use crate::entity::{self, EntityId, EntityNode};
use crate::game::Game;

/// The `GebHandRight` (`ak`) instance fields beyond the `Boss`/`Enemy` base — carried in
/// [`crate::boss::BossSubclass::GebHandRight`].
#[derive(Debug)]
pub struct GebHandRightData {
    /// `private byte ticksSinceHit;` (`ak.v`) — ticks since the last landed sweep; once
    /// past 100 the attack delay is removed.
    pub ticks_since_hit: i8,
    /// `private byte swingSide;` (`ak.w`) — which sweep to run next (1 = long reach @f5,
    /// 2 = short reach @f8); toggles each swing.
    pub swing_side: i8,
}

/// The [`GebHandRightData`] behind a `GebHandRight` node.
fn data(node: &EntityNode) -> &GebHandRightData {
    match &node
        .as_boss()
        .expect("GebHandRight node is a Boss")
        .subclass
    {
        BossSubclass::GebHandRight(d) => d,
        _ => unreachable!("geb_hand_right dispatched on a non-GebHandRight node"),
    }
}

/// Mutable [`data`].
fn data_mut(node: &mut EntityNode) -> &mut GebHandRightData {
    match &mut node
        .as_boss_mut()
        .expect("GebHandRight node is a Boss")
        .subclass
    {
        BossSubclass::GebHandRight(d) => d,
        _ => unreachable!("geb_hand_right dispatched on a non-GebHandRight node"),
    }
}

/// `public GebHandRight(GameMap map, byte tileX, byte tileY, byte kind, byte statRow)`
/// (`ak.<init>:(Lae;BBBB)V => ["iadd","i2b","iadd","iadd","iadd"]`). `super(tileX,
/// (byte)(tileY+4), kind, statRow, (byte) 1)` is [`boss::new_boss`] (four rows down,
/// layer 1); the node is tagged with `swingSide = 2`, the two spawn cells cleared, and
/// the short attack delay armed.
pub fn new(g: &mut Game, tile_x: i8, tile_y: i8, kind: i8, stat_row: i8) -> EntityId {
    // super(tileX, (byte) (tileY + 4), kind, statRow, (byte) 1);
    let super_tile_y = (tile_y as i32).wrapping_add(4) as i8;
    let id = boss::new_boss(g, tile_x, super_tile_y, kind, stat_row, 1);
    // this.swingSide = (byte) 2;  (ticksSinceHit at JVM byte default 0)
    g.entity_arena[id]
        .as_boss_mut()
        .expect("GebHandRight node is a Boss")
        .subclass = BossSubclass::GebHandRight(GebHandRightData {
        ticks_since_hit: 0,
        swing_side: 2,
    });
    // map.occupancy[tileY + 4][tileX] = null; map.occupancy[tileY + 4][tileX + 1] = null;
    {
        let map = g
            .game_state
            .map
            .as_mut()
            .expect("GameState.map null in GebHandRight ctor");
        let occ = map.occupancy.as_mut().expect("occupancy null");
        let r = (tile_y as i32).wrapping_add(4) as usize;
        occ[r][tile_x as usize] = None;
        occ[r][(tile_x as i32).wrapping_add(1) as usize] = None;
    }
    // ((Enemy) this).stats.attackDelay = (byte) 2;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("GebHandRight enemy")
        .stats
        .attack_delay = 2;
    // this.ticksSinceHit = (byte) 0;  (already 0)
    id
}

/// `public final void update()` (`ak.d:()V`, overriding `Boss.update`) — advance frame +
/// idle counter, drop the attack delay once idle, play out the retract (state 3) or run
/// the AI, bob the pixel row while stepping (state 2), then animate.
pub fn update(g: &mut Game, id: EntityId) {
    // this.animFrame = (byte) (this.animFrame + 1);
    {
        let e = g.entity_arena[id]
            .as_enemy_mut()
            .expect("GebHandRight enemy");
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
            .expect("GebHandRight enemy")
            .stats
            .attack_delay = 0;
    }
    // if (this.state == 3) { if (animFrame >= bossFrames[(statRow*16)+12+(moveDir-1)][0]) { enterIdle(false); tryAttack(); } } else updateAi();
    let (state, anim_frame, stat_row, move_dir) = {
        let e = g.entity_arena[id].as_enemy().expect("GebHandRight enemy");
        (
            e.battler.state,
            e.battler.anim_frame,
            e.stat_row,
            e.battler.move_dir,
        )
    };
    if state == 3 {
        // (byte[]) AssetCache.bossFrames[(statRow*16)+12+(moveDir-1)]  — DEFERRED-loaded bank.
        let idx = (stat_row as i32)
            .wrapping_mul(16)
            .wrapping_add(12)
            .wrapping_add((move_dir as i32).wrapping_sub(1));
        let frame0 = g
            .asset_cache
            .boss_frames
            .as_ref()
            .expect("AssetCache.bossFrames null in GebHandRight.update")[idx as usize]
            .as_ref()
            .expect("bossFrames element null (DEFERRED-loaded bank)")[0];
        if anim_frame as i32 >= frame0 as i32 {
            // enterIdle(false);
            enemy::enter_idle(g, id, false);
            // tryAttack();   (GebHandRight.tryAttack — this leaf's own override)
            try_attack(g, id);
        }
    } else {
        // updateAi();   (inherited Enemy.updateAi re-host)
        boss::update_ai(g, id);
    }
    // if (this.state == 2) { pixelY bob; syncTile(); }
    let (state, facing) = {
        let e = g.entity_arena[id].as_enemy().expect("GebHandRight enemy");
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
/// (`ak.a:(…Graphics;II)V => ["iinc"]`, overriding `Boss.paint`) — draw the tall sprite
/// lifted 64px up-screen; force facing down UNLESS mid-retract (state 3).
pub fn paint(g: &mut Game, id: EntityId, origin_x: i32, origin_y: i32) {
    // byte savedMoveDir = this.moveDir;
    let saved = g.entity_arena[id]
        .as_battler()
        .expect("GebHandRight battler")
        .move_dir;
    // if (this.state != 3) this.moveDir = (byte) 1;
    let state = g.entity_arena[id]
        .as_enemy()
        .expect("GebHandRight enemy")
        .battler
        .state;
    if state != 3 {
        g.entity_arena[id]
            .as_battler_mut()
            .expect("GebHandRight battler")
            .move_dir = 1;
    }
    // super.paint(graphics, originX, originY - 64);
    boss::paint_base(g, id, origin_x, origin_y.wrapping_sub(64));
    // this.moveDir = savedMoveDir;
    g.entity_arena[id]
        .as_battler_mut()
        .expect("GebHandRight battler")
        .move_dir = saved;
}

/// `public final void tryAttack()` (`ak.i:()V => ["isub","iadd","isub","isub"]`,
/// overriding `Enemy.tryAttack`) — sweep when the hero is aligned to the left, flipping
/// the swing side; else slide up / down / hold to line up.
// The `heroRowOffset >= -1 && heroRowOffset <= 2` bound is the faithful Java pair.
#[allow(clippy::manual_range_contains)]
pub fn try_attack(g: &mut Game, id: EntityId) {
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in GebHandRight.tryAttack");
    let (hero_tx, hero_ty) = {
        let n = &g.entity_arena[hero];
        (n.tile_x as i32, n.tile_y as i32)
    };
    let (tile_x, tile_y) = {
        let n = &g.entity_arena[id];
        (n.tile_x as i32, n.tile_y as i32)
    };
    // int heroRowOffset = ((Entity) hero).tileY - ((((Entity) this).tileY - 4) + 2);
    let hero_row_offset = hero_ty.wrapping_sub(tile_y.wrapping_sub(4).wrapping_add(2));
    let (hurt_cooldown, attack_cooldown) = {
        let e = g.entity_arena[id].as_enemy().expect("GebHandRight enemy");
        (e.hurt_cooldown, e.attack_cooldown)
    };
    // if (hurtCooldown == 0 && heroRowOffset >= -1 && heroRowOffset <= 2 && hero.tileX >= tileX - 7) {
    //   beginAttack(); swingSide = Directions.reverse[swingSide]; setFacing(swingSide); }
    if hurt_cooldown == 0
        && hero_row_offset >= -1
        && hero_row_offset <= 2
        && hero_tx >= tile_x.wrapping_sub(7)
    {
        enemy::begin_attack(g, id);
        // this.swingSide = Directions.reverse[this.swingSide];
        let new_swing = {
            let d = data_mut(&mut g.entity_arena[id]);
            d.swing_side = REVERSE[d.swing_side as usize];
            d.swing_side
        };
        // setFacing(this.swingSide);
        set_facing(g, id, new_swing);
    } else if attack_cooldown == 0 {
        if hero_row_offset > 2 {
            // setState((byte) 2); setFacing((byte) 2);
            set_state(g, id, 2);
            set_facing(g, id, 2);
        } else if hero_row_offset < -1 {
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

/// `Enemy.setState` for this hand (no `animFrame` reset — not overridden by
/// `Boss`/`GebHandRight`): set the FSM state directly.
fn set_state(g: &mut Game, id: EntityId, new_state: i8) {
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("GebHandRight enemy")
        .battler
        .state = new_state;
}

/// `Battler.setFacing(dir)` — face `dir` and commit it as the step direction.
fn set_facing(g: &mut Game, id: EntityId, dir: i8) {
    let b = g.entity_arena[id]
        .as_battler_mut()
        .expect("GebHandRight battler");
    battler::set_facing(b, dir);
}

/// `public final void resolveAttack()` (`ak.j:()V`, overriding `Enemy.resolveAttack`) —
/// the twin sweep: the long reach on frame 5 (`swingSide == 1`), the short reach on frame
/// 8 (`swingSide == 2`).
pub fn resolve_attack(g: &mut Game, id: EntityId) {
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in GebHandRight.resolveAttack");
    let anim_frame = g.entity_arena[id]
        .as_enemy()
        .expect("GebHandRight enemy")
        .battler
        .anim_frame;
    let swing_side = data(&g.entity_arena[id]).swing_side;
    let (tile_x, tile_y) = {
        let n = &g.entity_arena[id];
        (n.tile_x as i32, n.tile_y as i32)
    };
    let (hero_tx, hero_ty) = {
        let n = &g.entity_arena[hero];
        (n.tile_x, n.tile_y)
    };
    // if (animFrame == 5 && swingSide == 1) { long reach @ tileX-7 .. tileX-1 }
    if anim_frame == 5 && swing_side == 1 {
        let hit_min_x = tile_x.wrapping_sub(7) as i8;
        let hit_max_x = tile_x.wrapping_sub(1) as i8;
        let hit_min_y = tile_y.wrapping_sub(4).wrapping_add(1) as i8;
        let hit_max_y = tile_y.wrapping_sub(4).wrapping_add(4) as i8;
        // if (out of box) return;
        if (hero_tx as i32) < (hit_min_x as i32)
            || (hero_tx as i32) > (hit_max_x as i32)
            || (hero_ty as i32) < (hit_min_y as i32)
            || (hero_ty as i32) > (hit_max_y as i32)
        {
            return;
        }
        // hero.takeHit((Enemy) this, (byte) 3);   — DEFERRED: Hero.takeHit.
        return;
    }
    // if (animFrame == 8 && swingSide == 2) { short reach @ tileX-5 .. tileX-1 }
    if anim_frame == 8 && swing_side == 2 {
        let hit_min_x = tile_x.wrapping_sub(5) as i8;
        let hit_max_x = tile_x.wrapping_sub(1) as i8;
        let hit_min_y = tile_y.wrapping_sub(4).wrapping_add(1) as i8;
        let hit_max_y = tile_y.wrapping_sub(4).wrapping_add(4) as i8;
        if (hero_tx as i32) < (hit_min_x as i32)
            || (hero_tx as i32) > (hit_max_x as i32)
            || (hero_ty as i32) < (hit_min_y as i32)
            || (hero_ty as i32) > (hit_max_y as i32)
        {
            return;
        }
        // hero.takeHit((Enemy) this, (byte) 2);   — DEFERRED: Hero.takeHit.
        // this.ticksSinceHit = (byte) 0;
        data_mut(&mut g.entity_arena[id]).ticks_since_hit = 0;
        // ((Enemy) this).stats.attackDelay = (byte) 2;
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("GebHandRight enemy")
            .stats
            .attack_delay = 2;
    }
}

/// `public final void onDeath()` (`ak.m:()V => []`, overriding the abstract
/// `Boss.onDeath`) — despawn immediately (no death-animation delay).
pub fn on_death(g: &mut Game, id: EntityId) {
    // this.deathTimer = (byte) 0;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("GebHandRight enemy")
        .death_timer = 0;
}
