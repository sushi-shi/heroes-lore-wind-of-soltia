//! Transliterated from `java/src/main/java/defpackage/NordHealer.java`
//! (original `cd.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Support part of the three-part phase-2 **Nord** encounter (`NordHealer extends Boss`,
//! enemy-data record 38, attack 1 — it barely hits, it heals). Spawned together with the
//! [`crate::nord_body2`] core and the [`crate::nord_tentacle`] striker, and linked to them
//! via [`set_parts`]. On each cast it first triage-heals whichever part has dropped below
//! half HP (core, then striker, then itself), otherwise it round-robins a smaller top-up
//! across the three ([`heal_rotation`]). Every heal restores one tenth of the *core's*
//! maximum HP.
//!
//! ## Entity model — [`crate::boss::BossSubclass::NordHealer`]
//!
//! A `NordHealer` is a [`crate::entity::EntityData::Boss`] node tagged
//! [`crate::boss::BossSubclass::NordHealer`] carrying [`NordHealerData`] (the two linked
//! handles — `null` until [`set_parts`] — and the round-robin cursor). It overrides
//! `update`/`chase`/`tryAttack`/`resolveAttack`/`onDeath`; the inherited `updateAi`/
//! `animate`/`stepDeathAnim`/`die`/`paint` are the `Boss`/`Enemy` bodies reached through
//! the [`crate::boss`] dispatchers.
//!
//! ## DEFERRED cross-class boundaries
//!
//! None. [`resolve_attack`] and [`heal_target`] read only the linked parts' `Enemy` state
//! (HP / `stats.maxHp` / FSM `state`) and drive `Enemy.heal` / `Battler.addFloater`, all of
//! which are ported — so the heal-target scan is transliterated in full.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `cd.<init>:(BBBB)V => []`,
//! `cd.a:(Lag;Lbd;)V (setParts) => []`,
//! `cd.a:(Lal;)V (healTarget) => ["i2s","idiv","idiv","ineg","i2s"]`,
//! `cd.d:()V (update) => ["iadd","i2b"]`, `cd.h:()V (chase) => []`,
//! `cd.i:()V (tryAttack) => []`, `cd.j:()V (resolveAttack) => ["idiv","idiv","idiv"]`,
//! `cd.m:()V (onDeath) => []`.

use crate::battler;
use crate::boss::{self, BossSubclass};
use crate::enemy;
use crate::entity::{EntityId, EntityNode};
use crate::floater;
use crate::game::Game;
use j2me_jvm::java_div;

/// The `NordHealer` (`cd`) instance fields beyond the `Boss`/`Enemy` base — carried in
/// [`crate::boss::BossSubclass::NordHealer`].
#[derive(Debug)]
pub struct NordHealerData {
    /// `private NordBody2 body;` (`cd.a`) — the core this part keeps alive. `null` until
    /// [`set_parts`] → [`Option`].
    pub body: Option<EntityId>,
    /// `private NordTentacle striker;` (`cd.f187a`) — the striker this part keeps alive.
    /// `null` until [`set_parts`] → [`Option`].
    pub striker: Option<EntityId>,
    /// `private byte healRotation;` (`cd.v`) — round-robin cursor (0=core, 1=striker,
    /// 2=self) for the top-up heal.
    pub heal_rotation: i8,
}

/// The [`NordHealerData`] behind a `NordHealer` node.
fn data(node: &EntityNode) -> &NordHealerData {
    match &node.as_boss().expect("NordHealer node is a Boss").subclass {
        BossSubclass::NordHealer(d) => d,
        _ => unreachable!("nord_healer dispatched on a non-NordHealer node"),
    }
}

/// Mutable [`data`].
fn data_mut(node: &mut EntityNode) -> &mut NordHealerData {
    match &mut node
        .as_boss_mut()
        .expect("NordHealer node is a Boss")
        .subclass
    {
        BossSubclass::NordHealer(d) => d,
        _ => unreachable!("nord_healer dispatched on a non-NordHealer node"),
    }
}

/// A part's `(state, hp, stats.maxHp)` — the three `Enemy` fields the triage scan reads.
fn state_hp_maxhp(g: &Game, id: EntityId) -> (i8, i16, i16) {
    let e = g.entity_arena[id].as_enemy().expect("Nord part enemy");
    (e.battler.state, e.hp, e.stats.max_hp)
}

/// `public NordHealer(byte tileX, byte tileY, byte kind, byte statRow)`
/// (`cd.<init>:(BBBB)V => []`). `super(tileX, tileY, kind, statRow, (byte) 2)` is
/// [`boss::new_boss`] (layer 2); the node is then tagged [`BossSubclass::NordHealer`] with
/// `healRotation = 0` and both linked handles `null` (assigned later by [`set_parts`]).
pub fn new(g: &mut Game, tile_x: i8, tile_y: i8, kind: i8, stat_row: i8) -> EntityId {
    // super(tileX, tileY, kind, statRow, (byte) 2);
    let id = boss::new_boss(g, tile_x, tile_y, kind, stat_row, 2);
    // this.healRotation = (byte) 0;  (body/striker null after construction — set by setParts)
    g.entity_arena[id]
        .as_boss_mut()
        .expect("NordHealer node is a Boss")
        .subclass = BossSubclass::NordHealer(NordHealerData {
        body: None,
        striker: None,
        heal_rotation: 0,
    });
    id
}

/// `public final void setParts(NordBody2 body, NordTentacle striker)`
/// (`cd.a:(Lag;Lbd;)V => []`) — links this healer to the core and striker parts spawned
/// alongside it.
pub fn set_parts(g: &mut Game, id: EntityId, body: EntityId, striker: EntityId) {
    // this.body = body; this.striker = striker;
    let d = data_mut(&mut g.entity_arena[id]);
    d.body = Some(body);
    d.striker = Some(striker);
}

/// `public final void update()` (`cd.d:()V => ["iadd","i2b"]`, overriding `Boss.update`) —
/// advance `animFrame`, then `updateAi()`/`animate()`. No hero-offset cache, no
/// `stepIfMoving()`.
pub fn update(g: &mut Game, id: EntityId) {
    // this.animFrame = (byte) (this.animFrame + 1);
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("NordHealer enemy");
        e.battler.anim_frame = (e.battler.anim_frame as i32).wrapping_add(1) as i8;
    }
    // updateAi();   (inherited Enemy.updateAi re-host)
    boss::update_ai(g, id);
    // animate();   (inherited Enemy.animate re-host — dispatches NordHealer.resolveAttack)
    boss::animate(g, id);
}

/// `public final void chase()` (`cd.h:()V => []`, overriding `Enemy.chase`) — end the cast
/// wind-up and return to idle once the walk frames run out.
pub fn chase(g: &mut Game, id: EntityId) {
    // if (this.animFrame >= ((Enemy) this).stats.walkFrames) setState((byte) 1);
    let (anim_frame, walk_frames) = {
        let e = g.entity_arena[id].as_enemy().expect("NordHealer enemy");
        (e.battler.anim_frame, e.stats.walk_frames)
    };
    if anim_frame as i32 >= walk_frames as i32 {
        // setState((byte) 1);   — Enemy.setState (no animFrame reset).
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("NordHealer enemy")
            .battler
            .state = 1;
    }
}

/// `public final void tryAttack()` (`cd.i:()V => []`, overriding `Enemy.tryAttack`) — begin
/// the cast the moment the hurt cooldown clears (no facing change — it heals, it does not
/// aim).
pub fn try_attack(g: &mut Game, id: EntityId) {
    // if (this.hurtCooldown == 0) beginAttack();
    let hurt_cooldown = g.entity_arena[id]
        .as_enemy()
        .expect("NordHealer enemy")
        .hurt_cooldown;
    if hurt_cooldown == 0 {
        enemy::begin_attack(g, id);
    }
}

/// `public final void resolveAttack()` (`cd.j:()V => ["idiv","idiv","idiv"]`, overriding
/// `Enemy.resolveAttack`) — on cast frame 5, emergency-triage-heal any part below half HP
/// (core → striker → self, the latter two returning early), else a routine round-robin
/// top-up across the three.
pub fn resolve_attack(g: &mut Game, id: EntityId) {
    // if (this.animFrame == 5) {
    let anim_frame = g.entity_arena[id]
        .as_enemy()
        .expect("NordHealer enemy")
        .battler
        .anim_frame;
    if anim_frame != 5 {
        return;
    }
    let (body, striker) = {
        let d = data(&g.entity_arena[id]);
        (
            d.body.expect("NordHealer.body null (setParts not called)"),
            d.striker
                .expect("NordHealer.striker null (setParts not called)"),
        )
    };
    // if (body.state != 6 && body.state != 5 && body.hp < body.stats.maxHp / 2) healTarget(body);
    let (bs, bhp, bmax) = state_hp_maxhp(g, body);
    if bs != 6 && bs != 5 && (bhp as i32) < java_div(bmax as i32, 2).expect("body.stats.maxHp / 2")
    {
        heal_target(g, id, body);
    }
    // if (striker.state != 6 && striker.state != 5 && striker.hp < striker.stats.maxHp / 2) { healTarget(striker); return; }
    let (ss, shp, smax) = state_hp_maxhp(g, striker);
    if ss != 6
        && ss != 5
        && (shp as i32) < java_div(smax as i32, 2).expect("striker.stats.maxHp / 2")
    {
        heal_target(g, id, striker);
        return;
    }
    // if (this.state != 6 && this.state != 5 && this.hp < this.stats.maxHp / 2) { healTarget(this); return; }
    let (hs, hhp, hmax) = state_hp_maxhp(g, id);
    if hs != 6 && hs != 5 && (hhp as i32) < java_div(hmax as i32, 2).expect("this.stats.maxHp / 2")
    {
        heal_target(g, id, id);
        return;
    }
    // Otherwise a routine round-robin top-up across core/striker/self.
    let heal_rotation = data(&g.entity_arena[id]).heal_rotation;
    match heal_rotation {
        0 => {
            // if (body.hp < body.stats.maxHp) healTarget(body);
            let (_, bhp, bmax) = state_hp_maxhp(g, body);
            if bhp < bmax {
                heal_target(g, id, body);
            }
            // this.healRotation = (byte) 1;
            data_mut(&mut g.entity_arena[id]).heal_rotation = 1;
        }
        1 => {
            // if (striker.hp < striker.stats.maxHp) healTarget(striker);
            let (_, shp, smax) = state_hp_maxhp(g, striker);
            if shp < smax {
                heal_target(g, id, striker);
            }
            // this.healRotation = (byte) 2;
            data_mut(&mut g.entity_arena[id]).heal_rotation = 2;
        }
        2 => {
            // if (this.hp < this.stats.maxHp) healTarget(this);
            let (_, hhp, hmax) = state_hp_maxhp(g, id);
            if hhp < hmax {
                heal_target(g, id, id);
            }
            // this.healRotation = (byte) 0;
            data_mut(&mut g.entity_arena[id]).heal_rotation = 0;
        }
        _ => {}
    }
}

/// `private void healTarget(Enemy target)` (`cd.a:(Lal;)V => ["i2s","idiv","idiv","ineg",
/// "i2s"]`) — heals one part by a tenth of the *core's* max HP, showing a green heal
/// floater. The heal amount is always keyed to the core (`this.body`), not to `target`.
fn heal_target(g: &mut Game, id: EntityId, target: EntityId) {
    // target.addFloater(new Floater((byte) 9, (short) -1, this.statRow));
    let stat_row = g.entity_arena[id]
        .as_enemy()
        .expect("NordHealer enemy")
        .stat_row;
    let f = floater::new(9, -1, stat_row as i16);
    battler::add_floater(
        g.entity_arena[target]
            .as_battler_mut()
            .expect("heal target battler"),
        f,
    );
    // ((Enemy) this.body).stats.maxHp
    let body = data(&g.entity_arena[id])
        .body
        .expect("NordHealer.body null (setParts not called)");
    let body_max_hp = g.entity_arena[body]
        .as_enemy()
        .expect("NordBody2 enemy")
        .stats
        .max_hp as i32;
    // target.heal(((Enemy) this.body).stats.maxHp / 10);
    let heal_amt = java_div(body_max_hp, 10).expect("body.stats.maxHp / 10");
    enemy::heal(g, target, heal_amt);
    // target.addFloater(new Floater((byte) 7, (short) 4, (short) (-(((Enemy) this.body).stats.maxHp / 10))));
    let neg = java_div(body_max_hp, 10)
        .expect("body.stats.maxHp / 10")
        .wrapping_neg();
    let f2 = floater::new(7, 4, neg as i16);
    battler::add_floater(
        g.entity_arena[target]
            .as_battler_mut()
            .expect("heal target battler"),
        f2,
    );
}

/// `public final void onDeath()` (`cd.m:()V => []`, overriding the abstract `Boss.onDeath`)
/// — no death-animation delay: the corpse is reaped immediately.
pub fn on_death(g: &mut Game, id: EntityId) {
    // this.deathTimer = (byte) 0;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("NordHealer enemy")
        .death_timer = 0;
}
