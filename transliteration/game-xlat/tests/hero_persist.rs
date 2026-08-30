//! Hero persistence gate: `Hero.initClass` equipment + gear stats, and the
//! `Hero.save` / `Hero.load` round-trip (class `ao`).
//!
//! Two things this gate proves, both driven through the public API over the real
//! `/itm/*` item records pulled from `_originals/…v207.jar`:
//!
//! 1. **`initClass` equips the character.** For the mage (class 8) it fills all five
//!    equipment slots via `Item.create` and `recomputeStats` folds their `value` (+
//!    enchant/refine) into `attack`/`defense`. A **negative control** removes the gear
//!    and re-runs `recomputeStats`, proving the boost came from the equipment.
//! 2. **`Hero.save` / `Hero.load` round-trip.** A hero's core stats/level/exp,
//!    derived stats, base stats, and five equipment slots are serialized, then loaded
//!    into a **fresh** hero and asserted equal to the pre-save snapshot; the bag gold
//!    round-trips through `ItemBag.serialize`/`deserialize` (the slice
//!    `GameState.saveGame` pairs with the hero blob). Teeth (GATES.md R3): the fresh
//!    hero is CLOBBERED with sentinels before the load, and a one-unit perturbation of
//!    the snapshot is proven to differ.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::entity::EntityId;
use heroes_lore_wind_of_soltia_game_xlat::{byte_util, hero, item_bag, Game};

const SEED: i64 = 0x1234_5678;
const CLASS_MAGE: i8 = 8;

fn load_resources(g: &mut Game) {
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
}

/// A minimal Game with the JAR resources loaded — enough for `Item.create`'s
/// `AssetCache.loadItemRecord` and for `recomputeStats`' `markRedraw`.
fn setup() -> Game {
    let mut g = Game::new();
    g.byte_util = byte_util::ByteUtilState::seeded(SEED);
    load_resources(&mut g);
    g
}

/// Allocates a `new Hero((short)0,(short)0,(byte)8,(byte)8,classId)` and records it as
/// the active hero. Returns its arena handle.
fn new_hero_of(g: &mut Game, class_id: i8) -> EntityId {
    let id = {
        let Game {
            entity_arena,
            clock,
            ..
        } = &mut *g;
        hero::new_hero(entity_arena, clock, 0, 0, 8, 8, class_id)
    };
    g.game_state.hero = Some(id);
    g.game_state.class_id = class_id;
    id
}

/// A per-slot equipment fingerprint — everything the 10-byte serialize form + record
/// re-parse must reproduce.
#[derive(Clone, Debug, PartialEq, Eq)]
struct EquipDesc {
    r#type: i8,
    sub_id: i8,
    value: i16,
    identified: bool,
    refine_level: i8,
    attribute: i8,
    enchant: Vec<i8>,
    quantity: i8,
}

/// The hero fields the save/load pair (plus the paired bag) must round-trip.
#[derive(Clone, Debug, PartialEq, Eq)]
struct HeroSnapshot {
    class_id: i8,
    level: i8,
    hp: i32,
    mp: i32,
    exp: i32,
    max_hp: i32,
    max_mp: i32,
    exp_to_next: i32,
    max_combo: i8,
    stat_points: i16,
    strength: i16,
    vitality: i16,
    agility: i16,
    spirit: i16,
    play_seconds: i32,
    gold: i32,
    equipment: Vec<Option<EquipDesc>>,
}

fn snapshot(g: &Game, id: EntityId) -> HeroSnapshot {
    let h = g.entity_arena[id].as_hero().expect("hero");
    let equipment = (0..5)
        .map(|s| {
            h.equipment[s].as_ref().map(|it| {
                let b = it.borrow();
                EquipDesc {
                    r#type: b.r#type,
                    sub_id: b.sub_id,
                    value: b.value,
                    identified: b.identified,
                    refine_level: b.refine_level,
                    attribute: b.attribute,
                    enchant: b.enchant.clone(),
                    quantity: b.quantity,
                }
            })
        })
        .collect();
    HeroSnapshot {
        class_id: h.class_id,
        level: h.level,
        hp: h.hp,
        mp: h.mp,
        exp: h.exp,
        max_hp: h.max_hp,
        max_mp: h.max_mp,
        exp_to_next: h.exp_to_next,
        max_combo: h.max_combo,
        stat_points: h.stat_points,
        strength: h.strength,
        vitality: h.vitality,
        agility: h.agility,
        spirit: h.spirit,
        play_seconds: h.play_seconds,
        gold: h.bag.gold,
        equipment,
    }
}

#[test]
fn init_class_populates_five_equipment_slots_and_boosts_stats() {
    let mut g = setup();
    let id = new_hero_of(&mut g, CLASS_MAGE);
    hero::init_class(&mut g, id, CLASS_MAGE);

    // --- The five equipment slots are populated (mage fills all five). ---
    let (types, weapon_value, def_values) = {
        let h = g.entity_arena[id].as_hero().expect("hero");
        for slot in 0..5 {
            assert!(
                h.equipment[slot].is_some(),
                "mage equipment slot {slot} populated by initClass"
            );
        }
        let types: Vec<i8> = (0..5)
            .map(|s| h.equipment[s].as_ref().unwrap().borrow().r#type)
            .collect();
        // slot 0 weapon (type 1), slot 1 shield (type 3), slots 2/3/4 accessories (5/6/4).
        assert_eq!(types, vec![1, 3, 5, 6, 4], "mage starting equipment types");
        // initClass marks each identified and quantity 1.
        for slot in 0..5 {
            let it = h.equipment[slot].as_ref().unwrap().borrow();
            assert!(it.identified, "slot {slot} identified");
            assert_eq!(it.quantity, 1, "slot {slot} quantity 1");
        }
        let weapon_value = h.equipment[0].as_ref().unwrap().borrow().value as i32;
        let def_values: Vec<i32> = (1..5)
            .map(|s| h.equipment[s].as_ref().unwrap().borrow().value as i32)
            .collect();
        (types, weapon_value, def_values)
    };
    let _ = types;

    // --- Base + derived stats, gear folded in. ---
    // Base stats for the mage class.
    {
        let h = g.entity_arena[id].as_hero().expect("hero");
        assert_eq!(h.strength, 5, "mage strength");
        assert_eq!(h.vitality, 8, "mage vitality");
        assert_eq!(h.agility, 4, "mage agility");
        assert_eq!(h.spirit, 3, "mage spirit");
        assert_eq!(h.level, 1, "level 1");
        // maxHp = (vitality + vitalityBonus + level) * 12; maxMp likewise with spirit.
        // (vitalityBonus/spiritBonus are 0 — the starting gear has no enchants.)
        assert_eq!(h.max_hp, (8 + 1) * 12, "maxHp");
        assert_eq!(h.max_mp, (3 + 1) * 12, "maxMp");
        assert_eq!(h.hp, h.max_hp, "hp filled to maxHp");
        assert_eq!(h.mp, h.max_mp, "mp filled to maxMp");
        assert_eq!(h.exp, 0, "exp reset");
    }

    // The gear boost is real: the weapon has a positive value and it lands in attack.
    assert!(
        weapon_value > 0,
        "the starting weapon carries a positive value (gear can boost attack)"
    );
    // attack = (strength*4)/5 + weapon.value   (enchant 0, refine 0, strengthBonus 0).
    let expected_attack = (5 * 4) / 5 + weapon_value;
    // defense = sum(equip[1..4].value) + strength/5 + level/3.
    // strength/5 = 5/5 = 1; level/3 = 1/3 = 0 (dropped).
    let expected_defense: i32 = def_values.iter().sum::<i32>() + 1;
    {
        let h = g.entity_arena[id].as_hero().expect("hero");
        assert_eq!(
            h.attack as i32, expected_attack,
            "attack folds in the weapon value"
        );
        assert_eq!(
            h.defense as i32, expected_defense,
            "defense folds in the four armour/accessory values"
        );
    }

    // --- Negative control: strip the gear, recompute, and confirm attack drops back
    //     to the ungeared value — the boost above was equipment-driven, not a constant. ---
    {
        let h = g.entity_arena[id].as_hero_mut().expect("hero");
        for slot in 0..5 {
            h.equipment[slot] = None;
        }
    }
    hero::recompute_stats(&mut g, id);
    {
        let h = g.entity_arena[id].as_hero().expect("hero");
        assert_eq!(
            h.attack as i32,
            (5 * 4) / 5,
            "ungeared attack = (strength*4)/5 only"
        );
        assert!(
            (h.attack as i32) < expected_attack,
            "removing the gear lowered attack ({} < {expected_attack}) — the boost was the equipment",
            h.attack
        );
    }
}

#[test]
fn save_then_load_round_trips_core_stats_gold_and_equipment() {
    let mut g = setup();
    let src = new_hero_of(&mut g, CLASS_MAGE);
    hero::init_class(&mut g, src, CLASS_MAGE);

    // Stamp distinctive progression, then recompute so the derived stats are consistent
    // with it (as a real level-up would leave them).
    {
        let h = g.entity_arena[src].as_hero_mut().expect("hero");
        h.level = 7;
        h.exp = 4321;
        h.max_combo = 3;
        h.stat_points = 9;
        h.strength = 12;
        h.vitality = 13;
        h.agility = 14;
        h.spirit = 15;
        h.play_seconds = 12345;
        h.session_start_sec = 0;
        h.bag.gold = 0x0051_7A93; // 5_339_795 — a value no code path sets by chance.
    }
    hero::recompute_stats(&mut g, src);
    {
        // Drop HP/MP below their (now larger) caps so they are exercised distinctly.
        let h = g.entity_arena[src].as_hero_mut().expect("hero");
        h.hp = 55;
        h.mp = 66;
    }
    // Keep the clock at 0: save writes playSeconds + (0/1000 - sessionStartSec) = playSeconds.
    g.clock.set(0);

    let snap = snapshot(&g, src);

    // --- Act: hero.save() + bag.serialize() (the two hero slices GameState.saveGame writes). ---
    let hero_bytes = hero::save(&mut g, src);
    let bag_bytes = {
        let h = g.entity_arena[src].as_hero().expect("hero");
        item_bag::serialize(&h.bag).expect("bag serialize")
    };

    // A fresh hero, CLOBBERED with sentinels so a real restore is unmistakable.
    let dst = new_hero_of(&mut g, CLASS_MAGE);
    {
        let h = g.entity_arena[dst].as_hero_mut().expect("hero");
        h.class_id = 42;
        h.level = 99;
        h.exp = -1;
        h.max_combo = 100;
        h.stat_points = -5;
        h.strength = 111;
        h.vitality = 112;
        h.agility = 113;
        h.spirit = 114;
        h.play_seconds = -7;
        h.bag.gold = -999;
    }

    // --- Load: hero core + equipment, then the bag (gold). ---
    hero::load(&mut g, dst, &hero_bytes);
    {
        // ItemBag.deserialize needs &mut Game (item creation); move the bag out, fill it,
        // and move it back (the store never aliases the arena) — the saveGame idiom.
        let mut bag = {
            let h = g.entity_arena[dst].as_hero_mut().expect("hero");
            std::mem::replace(&mut h.bag, item_bag::new(30))
        };
        item_bag::deserialize(&mut bag, &mut g, &bag_bytes);
        g.entity_arena[dst].as_hero_mut().expect("hero").bag = bag;
    }

    // --- Assert: the restored hero equals the pre-save snapshot. ---
    let restored = snapshot(&g, dst);
    assert_eq!(
        restored, snap,
        "hero core stats/level/exp/derived stats/equipment + bag gold round-tripped"
    );

    // Teeth: the sentinels were overwritten (nothing left clobbered).
    {
        let h = g.entity_arena[dst].as_hero().expect("hero");
        assert_ne!(h.level, 99, "level not left clobbered");
        assert_ne!(h.class_id, 42, "classId not left clobbered");
        assert_ne!(h.bag.gold, -999, "gold not left clobbered");
    }

    // Blindness: a one-unit perturbation of the snapshot must not read as a match.
    let mut perturbed = snap.clone();
    perturbed.level = perturbed.level.wrapping_add(1);
    assert_ne!(
        restored, perturbed,
        "a one-unit perturbation must differ — the comparison is not blind"
    );

    // The saved playSeconds survived (clock held at 0).
    assert_eq!(
        g.entity_arena[dst].as_hero().expect("hero").play_seconds,
        12345,
        "playSeconds round-tripped"
    );
}
