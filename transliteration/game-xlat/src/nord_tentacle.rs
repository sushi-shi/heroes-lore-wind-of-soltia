//! Transliterated from `java/src/main/java/defpackage/NordTentacle.java`
//! (original `bd.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Striker tentacle of the second-phase **Nord** encounter (`NordTentacle extends Boss`,
//! enemy-data record 37, attack 100). One of the three linked parts spawned together by
//! `GameMap.spawnNordBoss(false)` alongside the [`crate::nord_body2`] core and the
//! [`crate::nord_healer`]. Its attack is telegraphed: at frame 4 it marks the hero's
//! current tile (planting a warning effect there), and at frame 7 it lands the blow only
//! if the hero is still standing on that marked tile.
//!
//! ## Entity model — [`crate::boss::BossSubclass::NordTentacle`]
//!
//! A `NordTentacle` is a [`crate::entity::EntityData::Boss`] node tagged
//! [`crate::boss::BossSubclass::NordTentacle`] carrying [`NordTentacleData`] (the marked
//! hero tile). It overrides `update`/`chase`/`tryAttack`/`resolveAttack`/`onDeath`; the
//! inherited `updateAi`/`animate`/`stepDeathAnim`/`die`/`paint` are the `Boss`/`Enemy`
//! bodies reached through the [`crate::boss`] dispatchers.
//!
//! ## DEFERRED cross-class boundaries
//!
//! - **`Hero.takeHit`.** [`resolve_attack`]'s frame-7 landed blow calls
//!   `hero.takeHit((Enemy) this, this.facing)` — DEFERRED (the hero combat FSM is partial),
//!   exactly as in [`crate::enemy::resolve_attack`]. The telegraph (frame-4 effect + tile
//!   mark) and the frame-7 re-check ARE ported.
//! - **`AssetCache.attackEffectScripts` bank.** The frame-4 telegraph effect is spawned via
//!   the inherited `Enemy.spawnEffectAt` ([`crate::enemy::spawn_effect_at`]), which reads
//!   this DEFERRED-loaded bank (a null element NPEs, faithfully; loaded before the boss
//!   acts in real play).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bd.<init>:(BBBB)V => []`,
//! `bd.d:()V (update) => ["iadd","i2b"]`, `bd.h:()V (chase) => []`,
//! `bd.i:()V (tryAttack) => []`, `bd.j:()V (resolveAttack) => []`,
//! `bd.m:()V (onDeath) => []`.

use crate::boss::{self, BossSubclass};
use crate::enemy;
use crate::entity::{EntityId, EntityNode};
use crate::game::Game;

/// The `NordTentacle` (`bd`) instance fields beyond the `Boss`/`Enemy` base — carried in
/// [`crate::boss::BossSubclass::NordTentacle`].
#[derive(Debug)]
pub struct NordTentacleData {
    /// `private byte markedTileX;` (`bd.v`) — hero tile-column recorded at the telegraph
    /// frame; re-checked on the hit frame.
    pub marked_tile_x: i8,
    /// `private byte markedTileY;` (`bd.w`) — hero tile-row recorded at the telegraph
    /// frame; re-checked on the hit frame.
    pub marked_tile_y: i8,
}

/// The [`NordTentacleData`] behind a `NordTentacle` node.
fn data(node: &EntityNode) -> &NordTentacleData {
    match &node
        .as_boss()
        .expect("NordTentacle node is a Boss")
        .subclass
    {
        BossSubclass::NordTentacle(d) => d,
        _ => unreachable!("nord_tentacle dispatched on a non-NordTentacle node"),
    }
}

/// Mutable [`data`].
fn data_mut(node: &mut EntityNode) -> &mut NordTentacleData {
    match &mut node
        .as_boss_mut()
        .expect("NordTentacle node is a Boss")
        .subclass
    {
        BossSubclass::NordTentacle(d) => d,
        _ => unreachable!("nord_tentacle dispatched on a non-NordTentacle node"),
    }
}

/// `public NordTentacle(byte tileX, byte tileY, byte kind, byte statRow)`
/// (`bd.<init>:(BBBB)V => []`). `super(tileX, tileY, kind, statRow, (byte) 2)` is
/// [`boss::new_boss`] (layer 2); the node is then tagged [`BossSubclass::NordTentacle`]
/// with the marked tile at its JVM byte default `(0, 0)`.
pub fn new(g: &mut Game, tile_x: i8, tile_y: i8, kind: i8, stat_row: i8) -> EntityId {
    // super(tileX, tileY, kind, statRow, (byte) 2);
    let id = boss::new_boss(g, tile_x, tile_y, kind, stat_row, 2);
    // (markedTileX/markedTileY at their JVM byte default 0.)
    g.entity_arena[id]
        .as_boss_mut()
        .expect("NordTentacle node is a Boss")
        .subclass = BossSubclass::NordTentacle(NordTentacleData {
        marked_tile_x: 0,
        marked_tile_y: 0,
    });
    id
}

/// `public final void update()` (`bd.d:()V => ["iadd","i2b"]`, overriding `Boss.update`) —
/// advance `animFrame`, then `updateAi()`/`animate()`. No hero-offset cache, no
/// `stepIfMoving()`.
pub fn update(g: &mut Game, id: EntityId) {
    // this.animFrame = (byte) (this.animFrame + 1);
    {
        let e = g.entity_arena[id]
            .as_enemy_mut()
            .expect("NordTentacle enemy");
        e.battler.anim_frame = (e.battler.anim_frame as i32).wrapping_add(1) as i8;
    }
    // updateAi();   (inherited Enemy.updateAi re-host)
    boss::update_ai(g, id);
    // animate();   (inherited Enemy.animate re-host — dispatches NordTentacle.resolveAttack)
    boss::animate(g, id);
}

/// `public final void chase()` (`bd.h:()V => []`, overriding `Enemy.chase`) — end the
/// telegraph wind-up and return to idle once the walk frames run out.
pub fn chase(g: &mut Game, id: EntityId) {
    // if (this.animFrame >= ((Enemy) this).stats.walkFrames) setState((byte) 1);
    let (anim_frame, walk_frames) = {
        let e = g.entity_arena[id].as_enemy().expect("NordTentacle enemy");
        (e.battler.anim_frame, e.stats.walk_frames)
    };
    if anim_frame as i32 >= walk_frames as i32 {
        // setState((byte) 1);   — Enemy.setState (no animFrame reset).
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("NordTentacle enemy")
            .battler
            .state = 1;
    }
}

/// `public final void tryAttack()` (`bd.i:()V => []`, overriding `Enemy.tryAttack`) — begin
/// the telegraphed slam the moment the hurt cooldown clears.
pub fn try_attack(g: &mut Game, id: EntityId) {
    // if (this.hurtCooldown == 0) beginAttack();
    let hurt_cooldown = g.entity_arena[id]
        .as_enemy()
        .expect("NordTentacle enemy")
        .hurt_cooldown;
    if hurt_cooldown == 0 {
        enemy::begin_attack(g, id);
    }
}

/// `public final void resolveAttack()` (`bd.j:()V => []`, overriding `Enemy.resolveAttack`)
/// — the telegraphed strike: at frame 4 plant a warning effect on the hero's tile and
/// record it; at frame 7 land the blow only if the hero is still standing there.
pub fn resolve_attack(g: &mut Game, id: EntityId) {
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in NordTentacle.resolveAttack");
    let anim_frame = g.entity_arena[id]
        .as_enemy()
        .expect("NordTentacle enemy")
        .battler
        .anim_frame;
    // if (this.animFrame == 4) { spawnEffectAt(hero.tileX, hero.tileY); markedTileX/Y = hero.tileX/Y; }
    if anim_frame == 4 {
        let (hero_tx, hero_ty) = {
            let n = &g.entity_arena[hero];
            (n.tile_x, n.tile_y)
        };
        // spawnEffectAt(hero.tileX, hero.tileY);   (inherited final Enemy.spawnEffectAt)
        enemy::spawn_effect_at(g, id, hero_tx, hero_ty);
        // this.markedTileX = hero.tileX; this.markedTileY = hero.tileY;
        let d = data_mut(&mut g.entity_arena[id]);
        d.marked_tile_x = hero_tx;
        d.marked_tile_y = hero_ty;
    }
    // if (this.animFrame == 7 && markedTileX == hero.tileX && markedTileY == hero.tileY) hero.takeHit(this, facing);
    if anim_frame == 7 {
        let (marked_x, marked_y) = {
            let d = data(&g.entity_arena[id]);
            (d.marked_tile_x, d.marked_tile_y)
        };
        let (hero_tx, hero_ty) = {
            let n = &g.entity_arena[hero];
            (n.tile_x, n.tile_y)
        };
        if marked_x == hero_tx && marked_y == hero_ty {
            // hero.takeHit((Enemy) this, this.facing);
            //   DEFERRED: Hero.takeHit — the hero-damage resolution (Hero's combat FSM is
            //   partial), exactly as in enemy.rs.
        }
    }
}

/// `public final void onDeath()` (`bd.m:()V => []`, overriding the abstract `Boss.onDeath`)
/// — no death-animation delay: the corpse is reaped immediately.
pub fn on_death(g: &mut Game, id: EntityId) {
    // this.deathTimer = (byte) 0;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("NordTentacle enemy")
        .death_timer = 0;
}
