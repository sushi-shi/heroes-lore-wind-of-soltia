//! Transliterated from `java/src/main/java/defpackage/GebHead.java`
//! (original `cg.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Main body ("head") of the three-part **Geb** encounter (`GebHead extends Boss`,
//! enemy-data record 39, size 2). Its attack marches its two-cell occupancy footprint
//! down arena columns 6..9 one row per frame ([`crush_row`]), shoving and hitting the
//! hero if caught underneath, pausing after every three strikes. It owns its
//! [`crate::geb_hand_left`] and [`crate::geb_hand_right`]; when it dies it seals columns
//! 6..9 impassable ([`die`], guarded by `collisionSealed`) and despawns both hands
//! ([`on_death`]).
//!
//! ## Entity model — [`crate::boss::BossSubclass::GebHead`]
//!
//! A `GebHead` is a [`crate::entity::EntityData::Boss`] node tagged
//! [`crate::boss::BossSubclass::GebHead`] carrying [`GebHeadData`] (the two owned hand
//! handles, the one-shot seal guard, the burst counter). It overrides `update`/`chase`/
//! `tryAttack`/`resolveAttack`/`stepDeathAnim`/`die`/`onDeath`; `updateAi`/`animate`/
//! `paint` are the inherited `Boss`/`Enemy` bodies reached through the [`crate::boss`]
//! dispatchers.
//!
//! ## DEFERRED cross-class boundaries
//!
//! - **`Hero.takeHit` / `Hero.slide`.** [`crush_row`] and [`resolve_attack`]'s tail hit
//!   apply the hero strike/shove — DEFERRED (hero combat FSM partial), as in
//!   [`crate::enemy`]. All the occupancy bookkeeping around them IS applied.
//! - **`collisionGrid`.** [`die`] seals columns 6..9 by writing `map.collisionGrid`,
//!   which is parsed by the DEFERRED `.evt` collision pass and is null in this slice
//!   (a null grid NPEs, faithfully — in real play the boss spawns after the parse).
//!
//! Opcode shapes (R8): `cg.<init>:(BBBBLba;Lak;)V => []`,
//! `cg.d:()V (update) => ["iadd","i2b"]`, `cg.h:()V (chase) => []`,
//! `cg.i:()V (tryAttack) => ["iadd","i2b"]`,
//! `cg.a:(Lao;Lae;B)V (crushRow) => ["iadd","i2b","isub","isub","iadd","i2b","iadd","i2b"]`,
//! `cg.j:()V (resolveAttack) => [row arithmetic + "idiv","i2s" (DEFERRED attack/2)]`,
//! `cg.k:()V (stepDeathAnim) => ["isub","i2b"]`,
//! `cg.l:()V (die) => ["iadd","iadd","i2b","iadd","i2b"]`,
//! `cg.m:()V (onDeath) => ["iadd","iadd","iinc","iadd","i2b"]`.

use crate::battler;
use crate::boss::{self, BossSubclass};
use crate::debug;
use crate::enemy;
use crate::entity::{EntityId, EntityNode};
use crate::game::Game;
use crate::game_map;

/// The `GebHead` (`cg`) instance fields beyond the `Boss`/`Enemy` base — carried in
/// [`crate::boss::BossSubclass::GebHead`].
#[derive(Debug)]
pub struct GebHeadData {
    /// `private GebHandLeft leftHand;` (`cg.a`) — the left hand this head owns
    /// (despawned when the head dies).
    pub left_hand: EntityId,
    /// `private GebHandRight rightHand;` (`cg.f238a`) — the right hand this head owns.
    pub right_hand: EntityId,
    /// `private boolean collisionSealed;` (`cg.g`) — guards the one-shot collision seal
    /// in [`die`] against re-running.
    pub collision_sealed: bool,
    /// `private byte attackBurstCount;` (`cg.v`) — strikes landed in the current burst;
    /// a fourth forces a long cooldown.
    pub attack_burst_count: i8,
}

/// The [`GebHeadData`] behind a `GebHead` node.
fn data(node: &EntityNode) -> &GebHeadData {
    match &node.as_boss().expect("GebHead node is a Boss").subclass {
        BossSubclass::GebHead(d) => d,
        _ => unreachable!("geb_head dispatched on a non-GebHead node"),
    }
}

/// Mutable [`data`].
fn data_mut(node: &mut EntityNode) -> &mut GebHeadData {
    match &mut node.as_boss_mut().expect("GebHead node is a Boss").subclass {
        BossSubclass::GebHead(d) => d,
        _ => unreachable!("geb_head dispatched on a non-GebHead node"),
    }
}

/// `public GebHead(byte tileX, byte tileY, byte kind, byte statRow, GebHandLeft leftHand,
/// GebHandRight rightHand)` (`cg.<init>:(BBBBLba;Lak;)V => []`). `super(tileX, tileY, kind,
/// statRow, (byte) 2)` is [`boss::new_boss`] (layer 2); the node is tagged
/// [`BossSubclass::GebHead`] holding both hand handles.
pub fn new(
    g: &mut Game,
    tile_x: i8,
    tile_y: i8,
    kind: i8,
    stat_row: i8,
    left_hand: EntityId,
    right_hand: EntityId,
) -> EntityId {
    // super(tileX, tileY, kind, statRow, (byte) 2);
    let id = boss::new_boss(g, tile_x, tile_y, kind, stat_row, 2);
    // this.leftHand = leftHand; this.rightHand = rightHand;
    // this.collisionSealed = false; this.attackBurstCount = (byte) 0;
    g.entity_arena[id]
        .as_boss_mut()
        .expect("GebHead node is a Boss")
        .subclass = BossSubclass::GebHead(GebHeadData {
        left_hand,
        right_hand,
        collision_sealed: false,
        attack_burst_count: 0,
    });
    id
}

/// `public final void update()` (`cg.d:()V => ["iadd","i2b"]`, overriding `Boss.update`)
/// — advance `animFrame`, then `updateAi()`/`animate()`. No hero-offset cache, no
/// `stepIfMoving()`.
pub fn update(g: &mut Game, id: EntityId) {
    // this.animFrame = (byte) (this.animFrame + 1);
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("GebHead enemy");
        e.battler.anim_frame = (e.battler.anim_frame as i32).wrapping_add(1) as i8;
    }
    // updateAi();   (inherited Enemy.updateAi re-host)
    boss::update_ai(g, id);
    // animate();   (inherited Enemy.animate re-host — dispatches GebHead.resolveAttack/stepDeathAnim)
    boss::animate(g, id);
}

/// `public final void chase()` (`cg.h:()V => []`, overriding `Enemy.chase`) — ends the
/// crush pass and returns to idle once the attack frames run out.
pub fn chase(g: &mut Game, id: EntityId) {
    // if (this.animFrame >= ((Enemy) this).stats.attackFrames) setState((byte) 1);
    let (anim_frame, attack_frames) = {
        let e = g.entity_arena[id].as_enemy().expect("GebHead enemy");
        (e.battler.anim_frame, e.stats.attack_frames)
    };
    if anim_frame as i32 >= attack_frames as i32 {
        // setState((byte) 1);   — Enemy.setState (no animFrame reset).
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("GebHead enemy")
            .battler
            .state = 1;
    }
}

/// `public final void tryAttack()` (`cg.i:()V => ["iadd","i2b"]`, overriding
/// `Enemy.tryAttack`) — after a three-strike burst force a long cooldown; otherwise face
/// down and begin the next crush.
pub fn try_attack(g: &mut Game, id: EntityId) {
    // if (this.attackBurstCount >= 3) { this.hurtCooldown = (byte) 40; this.attackBurstCount = (byte) 0; }
    let attack_burst_count = data(&g.entity_arena[id]).attack_burst_count;
    if attack_burst_count >= 3 {
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("GebHead enemy")
            .hurt_cooldown = 40;
        data_mut(&mut g.entity_arena[id]).attack_burst_count = 0;
    }
    // if (this.hurtCooldown == 0) { setFacing((byte) 2); beginAttack(); attackBurstCount++; }
    let hurt_cooldown = g.entity_arena[id]
        .as_enemy()
        .expect("GebHead enemy")
        .hurt_cooldown;
    if hurt_cooldown == 0 {
        // setFacing((byte) 2);
        {
            let b = g.entity_arena[id]
                .as_battler_mut()
                .expect("GebHead battler");
            battler::set_facing(b, 2);
        }
        // beginAttack();
        enemy::begin_attack(g, id);
        // this.attackBurstCount = (byte) (this.attackBurstCount + 1);
        let d = data_mut(&mut g.entity_arena[id]);
        d.attack_burst_count = (d.attack_burst_count as i32).wrapping_add(1) as i8;
    }
}

/// `public final void resolveAttack()` (`cg.j:()V`, overriding `Enemy.resolveAttack`) —
/// per animation frame, advance the crushing footprint one row down ([`crush_row`]) and,
/// on the final frame, release the last row and land the tail-end hit.
// The tail-hit `hero.tileX >= 6 && hero.tileX <= 9` bound is the faithful Java pair.
#[allow(clippy::manual_range_contains)]
pub fn resolve_attack(g: &mut Game, id: EntityId) {
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in GebHead.resolveAttack");
    let anim_frame = g.entity_arena[id]
        .as_enemy()
        .expect("GebHead enemy")
        .battler
        .anim_frame;
    let tile_y = g.entity_arena[id].tile_y as i32;
    match anim_frame {
        // case 6: crushRow(hero, map, tileY);
        6 => crush_row(g, id, hero, tile_y as i8),
        // case 7: crushRow(hero, map, (byte)(tileY+1));
        7 => crush_row(g, id, hero, tile_y.wrapping_add(1) as i8),
        // case 8: crushRow(hero, map, (byte)(tileY+2));
        8 => crush_row(g, id, hero, tile_y.wrapping_add(2) as i8),
        // case 9: crushRow(hero, map, (byte)(tileY+3));
        9 => crush_row(g, id, hero, tile_y.wrapping_add(3) as i8),
        // case 11: crushRow(hero, map, (byte)(tileY+4));
        11 => crush_row(g, id, hero, tile_y.wrapping_add(4) as i8),
        // case 12: release the last crushed row, then the tail-end hit.
        12 => {
            // for (byte col=6; col<=9; col++) if (map.occupancy[tileY+4][col]==this) map.occupancy[tileY+4][col]=null;
            {
                let map = g
                    .game_state
                    .map
                    .as_mut()
                    .expect("GameState.map null in GebHead.resolveAttack");
                let occ = map.occupancy.as_mut().expect("occupancy null");
                let r = tile_y.wrapping_add(4) as usize;
                let mut col: i8 = 6;
                while col <= 9 {
                    if occ[r][col as usize] == Some(id) {
                        occ[r][col as usize] = None;
                    }
                    col = (col as i32).wrapping_add(1) as i8;
                }
            }
            // if (hero.tileX >= 6 && hero.tileX <= 9 && hero.tileY >= tileY+5 && hero.tileY <= tileY+8)
            //   hero.takeHit(this, (short)(stats.attack / 2), (byte) 2);
            let (hero_tx, hero_ty) = {
                let n = &g.entity_arena[hero];
                (n.tile_x as i32, n.tile_y as i32)
            };
            if hero_tx >= 6
                && hero_tx <= 9
                && hero_ty >= tile_y.wrapping_add(5)
                && hero_ty <= tile_y.wrapping_add(8)
            {
                // hero.takeHit(this, (short) (((Enemy) this).stats.attack / 2), (byte) 2);
                //   DEFERRED: Hero.takeHit (the stats.attack/2 payload applied there).
            }
        }
        _ => {}
    }
}

/// `private void crushRow(Hero hero, GameMap map, byte row)` (`cg.a:(Lao;Lae;B)V`) —
/// advance the head's crushing footprint onto tile-row `row` (columns 6..9): shove/hit
/// the hero if caught there, vacate the previously occupied row, claim the new row, then
/// re-register occupancy.
fn crush_row(g: &mut Game, id: EntityId, hero: EntityId, row: i8) {
    let row = row as i32;
    // for (byte col=6; col<=9; col++) if (occupancy[row][col]==hero) { hero.slide(2,16); hero.takeHit(this,2); break; }
    {
        let map = g
            .game_state
            .map
            .as_ref()
            .expect("GameState.map null in GebHead.crushRow");
        let occ = map.occupancy.as_ref().expect("occupancy null");
        let mut col: i8 = 6;
        while col <= 9 {
            if occ[row as usize][col as usize] == Some(hero) {
                // hero.slide((byte) 2, (byte) 16);      — DEFERRED: Hero.slide (hero FSM partial).
                // hero.takeHit((Enemy) this, (byte) 2); — DEFERRED: Hero.takeHit.
                break;
            }
            col = (col as i32).wrapping_add(1) as i8;
        }
    }
    // for (byte col=6; col<=9; col++) if (occupancy[row-1][col]==this) occupancy[row-1][col]=null;
    {
        let map = g
            .game_state
            .map
            .as_mut()
            .expect("GameState.map null in GebHead.crushRow");
        let occ = map.occupancy.as_mut().expect("occupancy null");
        let r = row.wrapping_sub(1) as usize;
        let mut col: i8 = 6;
        while col <= 9 {
            if occ[r][col as usize] == Some(id) {
                occ[r][col as usize] = None;
            }
            col = (col as i32).wrapping_add(1) as i8;
        }
    }
    // for (byte col=6; col<=9; col++) { Debug.assertTrue(occupancy[row][col] != hero); occupancy[row][col] = this; }
    {
        let map = g
            .game_state
            .map
            .as_mut()
            .expect("GameState.map null in GebHead.crushRow");
        let occ = map.occupancy.as_mut().expect("occupancy null");
        let mut col: i8 = 6;
        while col <= 9 {
            debug::assert_true(occ[row as usize][col as usize] != Some(hero));
            occ[row as usize][col as usize] = Some(id);
            col = (col as i32).wrapping_add(1) as i8;
        }
    }
    // setOccupancy();
    battler::set_occupancy(g, id);
}

/// `public final void stepDeathAnim()` (`cg.k:()V => ["isub","i2b"]`, overriding
/// `Enemy.stepDeathAnim`) — clamps the death frame at the last die-animation frame.
pub fn step_death_anim(g: &mut Game, id: EntityId) {
    // if (this.animFrame >= stats.dieFrames) this.animFrame = (byte) (stats.dieFrames - 1);
    let (anim_frame, die_frames) = {
        let e = g.entity_arena[id].as_enemy().expect("GebHead enemy");
        (e.battler.anim_frame, e.stats.die_frames)
    };
    if anim_frame as i32 >= die_frames as i32 {
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("GebHead enemy")
            .battler
            .anim_frame = (die_frames as i32).wrapping_sub(1) as i8;
    }
}

/// `public final void die()` (`cg.l:()V => ["iadd","iadd","i2b","iadd","i2b"]`, overriding
/// `Boss.die`) — the one-shot collision seal over columns 6..9. Does NOT call
/// `super.die()`.
pub fn die(g: &mut Game, id: EntityId) {
    // if (this.collisionSealed) return;
    if data(&g.entity_arena[id]).collision_sealed {
        return;
    }
    let tile_y = g.entity_arena[id].tile_y as i32;
    {
        let map = g
            .game_state
            .map
            .as_mut()
            .expect("GameState.map null in GebHead.die");
        // map.collisionGrid[row][col] = 1;  — collisionGrid is DEFERRED-loaded (the DEFERRED
        //   .evt collision parse); null in this slice → NPE, faithful.
        let grid = map
            .collision_grid
            .as_mut()
            .expect("collisionGrid null (DEFERRED .evt collision parse)");
        // for (byte col=6; col<=9; col++) for (byte row=tileY; row<=tileY+2; row++) grid[row][col]=1;
        let mut col: i8 = 6;
        while col <= 9 {
            let mut row: i8 = tile_y as i8;
            while (row as i32) <= tile_y.wrapping_add(2) {
                grid[row as usize][col as usize] = 1;
                row = (row as i32).wrapping_add(1) as i8;
            }
            col = (col as i32).wrapping_add(1) as i8;
        }
    }
    // this.collisionSealed = true;
    data_mut(&mut g.entity_arena[id]).collision_sealed = true;
}

/// `public final void onDeath()` (`cg.m:()V => ["iadd","iadd","iinc","iadd","i2b"]`,
/// overriding the abstract `Boss.onDeath`) — arm the death timer, clear the footprint,
/// and despawn both owned hands.
pub fn on_death(g: &mut Game, id: EntityId) {
    // this.deathTimer = (byte) 12;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("GebHead enemy")
        .death_timer = 12;
    let tile_y = g.entity_arena[id].tile_y as i32;
    {
        let map = g
            .game_state
            .map
            .as_mut()
            .expect("GameState.map null in GebHead.onDeath");
        let occ = map.occupancy.as_mut().expect("occupancy null");
        // for (byte col=6; col<=9; col++) for (int row=tileY+1; row<=tileY+5; row++)
        //   if (occupancy[row][col]==this) occupancy[row][col]=null;
        let mut col: i8 = 6;
        while col <= 9 {
            let mut row: i32 = tile_y.wrapping_add(1);
            while row <= tile_y.wrapping_add(5) {
                if occ[row as usize][col as usize] == Some(id) {
                    occ[row as usize][col as usize] = None;
                }
                row = row.wrapping_add(1);
            }
            col = (col as i32).wrapping_add(1) as i8;
        }
    }
    // GameState.map.removeEntity(this.leftHand); GameState.map.removeEntity(this.rightHand);
    let (left, right) = {
        let d = data(&g.entity_arena[id]);
        (d.left_hand, d.right_hand)
    };
    game_map::remove_entity(g, left);
    game_map::remove_entity(g, right);
}
