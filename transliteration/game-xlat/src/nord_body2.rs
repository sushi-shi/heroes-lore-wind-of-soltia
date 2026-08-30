//! Transliterated from `java/src/main/java/defpackage/NordBody2.java`
//! (original `ag.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Core body of the three-part phase-2 **Nord** encounter (`NordBody2 extends Boss`,
//! enemy-data record 36), spawned by `GameMap.spawnNordBoss(false)`. It links to its two
//! companion parts — the [`crate::nord_healer`] and the [`crate::nord_tentacle`] striker —
//! via [`set_parts`]; killing this core ends the whole encounter. Its attack is a five-way
//! (plus/X) projectile volley, and on death it despawns both companion parts and fires
//! story trigger 1.
//!
//! ## Entity model — [`crate::boss::BossSubclass::NordBody2`]
//!
//! A `NordBody2` is a [`crate::entity::EntityData::Boss`] node tagged
//! [`crate::boss::BossSubclass::NordBody2`] carrying [`NordBody2Data`] (the two companion
//! handles, `null` until [`set_parts`]). It overrides `update`/`chase`/`tryAttack`/
//! `resolveAttack`/`stepDeathAnim`/`die`/`onDeath`; the inherited `updateAi`/`animate`/
//! `paint` are the `Boss`/`Enemy` bodies reached through the [`crate::boss`] dispatchers.
//! Its `update` (`ag.d`) omits the `heroDistX`/`heroDistY` cache and the `stepIfMoving()`
//! call that `Boss.update` makes (matching [`crate::geb_head::update`]).
//!
//! ## DEFERRED cross-class boundaries
//!
//! - **`EventScript.fire`.** [`die`] fires story trigger 1 (`EventScript.fire((byte) 1)`)
//!   to end the encounter; `EventScript` is unported, so the call is DEFERRED. The base
//!   `super.die()` despawn ([`crate::boss::die_base`]) IS run.
//! - **`AssetCache.attackEffectScripts` bank.** [`resolve_attack`]'s five-way volley and
//!   [`step_death_anim`]'s spatter read this DEFERRED-loaded bank (a null element NPEs,
//!   faithfully; loaded before the boss acts in real play).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `ag.<init>:(BBBB)V => []`,
//! `ag.a:(Lcd;Lbd;)V (setParts) => []`, `ag.d:()V (update) => ["iadd","i2b"]`,
//! `ag.h:()V (chase) => []`, `ag.i:()V (tryAttack) => []`,
//! `ag.j:()V (resolveAttack) => ["isub","i2b","isub","i2b","iadd","i2b","isub","i2b",
//! "iadd","i2b","iadd","i2b","iadd","i2b"]`,
//! `ag.k:()V (stepDeathAnim) => ["iadd","i2b","iadd","i2b","iadd","i2b","iadd","i2b"]`,
//! `ag.l:()V (die) => []`, `ag.m:()V (onDeath) => []`.

use crate::battler;
use crate::boss::{self, BossSubclass};
use crate::byte_util;
use crate::effect;
use crate::enemy;
use crate::entity::{EntityId, EntityNode};
use crate::game::Game;
use crate::game_map;
use crate::projectile;

/// The `NordBody2` (`ag`) instance fields beyond the `Boss`/`Enemy` base — carried in
/// [`crate::boss::BossSubclass::NordBody2`].
#[derive(Debug)]
pub struct NordBody2Data {
    /// `private NordHealer healer;` (`ag.a`) — the support part that tops up this core's
    /// (and the striker's) HP. `null` until [`set_parts`] → [`Option`].
    pub healer: Option<EntityId>,
    /// `private NordTentacle striker;` (`ag.f31a`) — the companion telegraphed-slam
    /// tentacle. `null` until [`set_parts`] → [`Option`].
    pub striker: Option<EntityId>,
}

/// The [`NordBody2Data`] behind a `NordBody2` node.
fn data(node: &EntityNode) -> &NordBody2Data {
    match &node.as_boss().expect("NordBody2 node is a Boss").subclass {
        BossSubclass::NordBody2(d) => d,
        _ => unreachable!("nord_body2 dispatched on a non-NordBody2 node"),
    }
}

/// Mutable [`data`].
fn data_mut(node: &mut EntityNode) -> &mut NordBody2Data {
    match &mut node
        .as_boss_mut()
        .expect("NordBody2 node is a Boss")
        .subclass
    {
        BossSubclass::NordBody2(d) => d,
        _ => unreachable!("nord_body2 dispatched on a non-NordBody2 node"),
    }
}

/// `(byte[]) AssetCache.attackEffectScripts[statRow]` — the DEFERRED-loaded sprite bank (a
/// null element NPEs, faithfully; loaded before the boss acts in real play).
fn attack_script(g: &Game, stat_row: i8) -> Vec<i8> {
    g.asset_cache
        .attack_effect_scripts
        .as_ref()
        .expect("AssetCache.attackEffectScripts null in NordBody2")[stat_row as usize]
        .clone()
        .expect("attackEffectScripts[statRow] null (DEFERRED-loaded bank)")
}

/// `public NordBody2(byte tileX, byte tileY, byte kind, byte statRow)`
/// (`ag.<init>:(BBBB)V => []`). `super(tileX, tileY, kind, statRow, (byte) 3)` is
/// [`boss::new_boss`] (layer 3); the node is then tagged [`BossSubclass::NordBody2`] with
/// both companion handles `null` (assigned later by [`set_parts`]).
pub fn new(g: &mut Game, tile_x: i8, tile_y: i8, kind: i8, stat_row: i8) -> EntityId {
    // super(tileX, tileY, kind, statRow, (byte) 3);
    let id = boss::new_boss(g, tile_x, tile_y, kind, stat_row, 3);
    // (healer/striker null after construction — set by setParts.)
    g.entity_arena[id]
        .as_boss_mut()
        .expect("NordBody2 node is a Boss")
        .subclass = BossSubclass::NordBody2(NordBody2Data {
        healer: None,
        striker: None,
    });
    id
}

/// `public final void setParts(NordHealer healer, NordTentacle striker)`
/// (`ag.a:(Lcd;Lbd;)V => []`) — links this core to the healer and striker parts spawned
/// alongside it.
pub fn set_parts(g: &mut Game, id: EntityId, healer: EntityId, striker: EntityId) {
    // this.healer = healer; this.striker = striker;
    let d = data_mut(&mut g.entity_arena[id]);
    d.healer = Some(healer);
    d.striker = Some(striker);
}

/// `public final void update()` (`ag.d:()V => ["iadd","i2b"]`, overriding `Boss.update`) —
/// advance `animFrame`, then `updateAi()`/`animate()`. No hero-offset cache, no
/// `stepIfMoving()`.
pub fn update(g: &mut Game, id: EntityId) {
    // this.animFrame = (byte) (this.animFrame + 1);
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("NordBody2 enemy");
        e.battler.anim_frame = (e.battler.anim_frame as i32).wrapping_add(1) as i8;
    }
    // updateAi();   (inherited Enemy.updateAi re-host)
    boss::update_ai(g, id);
    // animate();   (inherited Enemy.animate re-host — dispatches NordBody2.resolveAttack/stepDeathAnim)
    boss::animate(g, id);
}

/// `public final void chase()` (`ag.h:()V => []`, overriding `Enemy.chase`) — end the
/// volley wind-up and return to idle once the attack frames run out.
pub fn chase(g: &mut Game, id: EntityId) {
    // if (this.animFrame >= ((Enemy) this).stats.attackFrames) setState((byte) 1);
    let (anim_frame, attack_frames) = {
        let e = g.entity_arena[id].as_enemy().expect("NordBody2 enemy");
        (e.battler.anim_frame, e.stats.attack_frames)
    };
    if anim_frame as i32 >= attack_frames as i32 {
        // setState((byte) 1);   — Enemy.setState (no animFrame reset).
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("NordBody2 enemy")
            .battler
            .state = 1;
    }
}

/// `public final void tryAttack()` (`ag.i:()V => []`, overriding `Enemy.tryAttack`) — face
/// down and begin the volley the moment the hurt cooldown clears.
pub fn try_attack(g: &mut Game, id: EntityId) {
    // if (this.hurtCooldown == 0) { setFacing((byte) 2); beginAttack(); }
    let hurt_cooldown = g.entity_arena[id]
        .as_enemy()
        .expect("NordBody2 enemy")
        .hurt_cooldown;
    if hurt_cooldown == 0 {
        // setFacing((byte) 2);
        {
            let b = g.entity_arena[id]
                .as_battler_mut()
                .expect("NordBody2 battler");
            battler::set_facing(b, 2);
        }
        // beginAttack();
        enemy::begin_attack(g, id);
    }
}

/// `public final void resolveAttack()` (`ag.j:()V`, overriding `Enemy.resolveAttack`) — on
/// hit frame 2, lob the five-way (plus/X) projectile volley.
pub fn resolve_attack(g: &mut Game, id: EntityId) {
    // if (this.animFrame == 2) {
    let anim_frame = g.entity_arena[id]
        .as_enemy()
        .expect("NordBody2 enemy")
        .battler
        .anim_frame;
    if anim_frame == 2 {
        let (tile_x, tile_y, stat_row, facing) = {
            let n = &g.entity_arena[id];
            let e = n.as_enemy().expect("NordBody2 enemy");
            (
                n.tile_x as i32,
                n.tile_y as i32,
                e.stat_row,
                e.battler.facing,
            )
        };
        // Five projectiles: (x-1,y-1), (x+3,y-1), (x,y), (x+2,y), (x+1,y+1).
        let offsets: [(i32, i32); 5] = [(-1, -1), (3, -1), (0, 0), (2, 0), (1, 1)];
        for (dx, dy) in offsets {
            // new Projectile((byte)(tileX + dx), (byte)(tileY + dy), script, this, facing, (byte) 13, (byte) 2)
            let script = attack_script(g, stat_row);
            let ptx = tile_x.wrapping_add(dx) as i8;
            let pty = tile_y.wrapping_add(dy) as i8;
            let p = projectile::new_projectile_enemy(
                &mut g.entity_arena,
                ptx,
                pty,
                script,
                id,
                facing,
                13,
                2,
            );
            game_map::add_entity(g, p);
        }
    }
}

/// `public final void stepDeathAnim()` (`ag.k:()V => ["iadd","i2b","iadd","i2b","iadd",
/// "i2b","iadd","i2b"]`, overriding `Enemy.stepDeathAnim`) — while the death timer is high,
/// spatter two random effects (keyed to the healer's stat row) around the collapsing core.
pub fn step_death_anim(g: &mut Game, id: EntityId) {
    // if (this.deathTimer > 8) {
    let death_timer = g.entity_arena[id]
        .as_enemy()
        .expect("NordBody2 enemy")
        .death_timer;
    if death_timer > 8 {
        let (tile_x, tile_y) = {
            let n = &g.entity_arena[id];
            (n.tile_x as i32, n.tile_y as i32)
        };
        // this.healer.statRow
        let healer = data(&g.entity_arena[id])
            .healer
            .expect("NordBody2.healer null (setParts not called)");
        let healer_stat_row = g.entity_arena[healer]
            .as_enemy()
            .expect("NordHealer enemy")
            .stat_row;
        // new Effect((byte)(tileX + randRange(-2,2)), (byte)(tileY + randRange(-2,2)), script);
        let r1 = byte_util::rand_range(&mut g.byte_util, -2, 2);
        let r2 = byte_util::rand_range(&mut g.byte_util, -2, 2);
        let etx = tile_x.wrapping_add(r1) as i8;
        let ety = tile_y.wrapping_add(r2) as i8;
        let script = attack_script(g, healer_stat_row);
        let eff = effect::new_effect_from_script(&mut g.entity_arena, etx, ety, script);
        game_map::add_entity(g, eff);
        // new Effect((byte)(tileX + randRange(-2,2)), (byte)(tileY + randRange(-2,2)), script);
        let r1 = byte_util::rand_range(&mut g.byte_util, -2, 2);
        let r2 = byte_util::rand_range(&mut g.byte_util, -2, 2);
        let etx = tile_x.wrapping_add(r1) as i8;
        let ety = tile_y.wrapping_add(r2) as i8;
        let script = attack_script(g, healer_stat_row);
        let eff = effect::new_effect_from_script(&mut g.entity_arena, etx, ety, script);
        game_map::add_entity(g, eff);
    }
}

/// `public final void die()` (`ag.l:()V => []`, overriding `Boss.die`) — the base despawn
/// plus the encounter-ending story trigger.
pub fn die(g: &mut Game, id: EntityId) {
    // super.die();   (Boss.die → removeEntity)
    boss::die_base(g, id);
    // EventScript.fire((byte) 1);
    //   DEFERRED: EventScript.fire (the story-trigger machinery is unported).
}

/// `public final void onDeath()` (`ag.m:()V => []`, overriding the abstract `Boss.onDeath`)
/// — despawn both companion parts, then arm the (long) death-animation countdown.
pub fn on_death(g: &mut Game, id: EntityId) {
    let (healer, striker) = {
        let d = data(&g.entity_arena[id]);
        (
            d.healer
                .expect("NordBody2.healer null (setParts not called)"),
            d.striker
                .expect("NordBody2.striker null (setParts not called)"),
        )
    };
    // this.healer.die();   — VIRTUAL: NordHealer inherits Boss.die (die_base → removeEntity).
    boss::die(g, healer);
    // this.striker.die();  — VIRTUAL: NordTentacle inherits Boss.die (die_base → removeEntity).
    boss::die(g, striker);
    // this.deathTimer = (byte) 24;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("NordBody2 enemy")
        .death_timer = 24;
}
