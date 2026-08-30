//! Transliterated from `java/src/main/java/defpackage/GebCore.java`
//! (original `bv.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Final phase of the three-part **Geb** encounter: the passive "Geb (Hack)" core
//! (`GebCore extends Boss`, enemy-data record 42), a heavily-armoured high-HP weak point
//! with **no attack of its own**. It adds no instance fields, so its
//! [`crate::boss::BossSubclass::GebCore`] tag is a unit variant; it overrides only
//! `tryAttack` (a no-op), `die` (base despawn + a story trigger), and `onDeath`
//! (a short death timer). Everything else — `update`/`updateAi`/`paint`/`animate`/
//! `chase`/`resolveAttack`/`stepDeathAnim` — is the inherited `Boss`/`Enemy` behaviour
//! reached through the [`crate::boss`] dispatchers.
//!
//! ## DEFERRED cross-class boundaries
//!
//! - **`EventScript.fire`.** [`die`] fires story trigger 1 (`EventScript.fire((byte) 1)`)
//!   to end the encounter; `EventScript` is unported, so the call is DEFERRED. The base
//!   `super.die()` despawn ([`crate::boss::die_base`]) IS run.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bv.<init>:(Lae;BBBB)V => []`,
//! `bv.i:()V (tryAttack) => []`, `bv.l:()V (die) => []`, `bv.m:()V (onDeath) => []`.

use crate::boss::{self, BossSubclass};
use crate::entity::EntityId;
use crate::game::Game;

/// `public GebCore(GameMap map, byte tileX, byte tileY, byte kind, byte statRow)`
/// (`bv.<init>:(Lae;BBBB)V => []`). Allocates the core boss node and returns its
/// [`EntityId`]. `super(tileX, tileY, kind, statRow, (byte) 1)` is [`boss::new_boss`]
/// (layer 1); the `map` parameter is unused by the constructor body. The node is then
/// tagged [`BossSubclass::GebCore`].
pub fn new(g: &mut Game, tile_x: i8, tile_y: i8, kind: i8, stat_row: i8) -> EntityId {
    // super(tileX, tileY, kind, statRow, (byte) 1);
    let id = boss::new_boss(g, tile_x, tile_y, kind, stat_row, 1);
    // (no instance fields — GebCore just re-tags the Boss node.)
    g.entity_arena[id]
        .as_boss_mut()
        .expect("GebCore node is a Boss")
        .subclass = BossSubclass::GebCore;
    id
}

/// `public final void tryAttack()` (`bv.i:()V => []`, overriding `Enemy.tryAttack`) —
/// the core is a passive weak point: it never initiates an attack.
pub fn try_attack(g: &mut Game, id: EntityId) {
    // (empty override.)
    let _ = (g, id);
}

/// `public final void die()` (`bv.l:()V => []`, overriding `Boss.die`) — the base
/// despawn plus the encounter-ending story trigger.
pub fn die(g: &mut Game, id: EntityId) {
    // super.die();   (Boss.die → removeEntity)
    boss::die_base(g, id);
    // EventScript.fire((byte) 1);
    //   DEFERRED: EventScript.fire (the story-trigger machinery is unported).
}

/// `public final void onDeath()` (`bv.m:()V => []`, overriding the abstract
/// `Boss.onDeath`) — arms the death-animation countdown.
pub fn on_death(g: &mut Game, id: EntityId) {
    // this.deathTimer = (byte) 16;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("GebCore enemy")
        .death_timer = 16;
}
