//! Transliterated from `java/src/main/java/defpackage/EnemyType.java`
//! (original `j.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Stat template for one kind of monster, parsed from the `/enm/data` blob. Every
//! [`crate::enemy::EnemyData`] (and, later, `Boss`) holds a reference to its shared
//! template ([`crate::enemy::EnemyData::stats`]) supplying name, size, element, AI
//! type, combat stats (HP/attack/defense/evasion), timing (sight/attack/hurt delays),
//! the level, the loot [`EnemyTypeData::drop_table`], and the per-state animation
//! frame spans (bound later from the loaded sprite banks). The full set of templates
//! lives in the static [`EnemyTypeState::types`] array.
//!
//! ## The shared-template seam
//!
//! Java references one `EnemyType` object from the static `types[]` array and from
//! every `Enemy.stats` field at once. The templates are **immutable after the
//! load-time [`bind_sprites`]** (which runs during map/resource load, before any
//! `Enemy` is constructed — a fresh map clears the entity list, so no live enemy
//! survives a re-bind). The transliteration therefore keeps the sole mutable owner in
//! [`EnemyTypeState::types`] and gives each [`crate::enemy::EnemyData`] a per-instance
//! **clone** of its template as `stats`; because the template never changes after an
//! enemy spawns, the copy is observably identical to the shared Java reference. This
//! is the accepted deviation for this class (recorded in `docs/TRANSLITERATION.md`).
//!
//! ## DEFERRED name resolution
//!
//! [`parse`] reads the name field as `[u8 len][ASCII-decimal id]` and Java resolves
//! it to display text via `FontManager.getStringChars` → `StringTable` (the loaded
//! lang blob). Exactly as [`crate::item`] handles the identical pattern, the id stays
//! an indirection here: the ASCII id chars are kept in `name` and the
//! FontManager/StringTable resolution is DEFERRED.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `j.<clinit>:()V => []`,
//! `j.a:(I)V (alloc) => []`, `j.a:([BBB)V (parse) => ["iadd","iadd", … "ishr"×7,
//! "iand"×8, …]` (the packed-byte unpack + the walking cursor arithmetic),
//! `j.a:(B)V (bindSprites) => ["imul","iadd","imul","iadd","imul","iadd"]`,
//! `j.b:(B)V (bindSpritesBoss) => ["imul","iadd"×3]`.

use crate::debug;
use crate::game::Game;
use j2me_jvm::ishr;

/// `public static final byte[] attackHitFrame` (`j.a:[B`) — per-kind frame at which a
/// melee attack connects (indexed by [`crate::enemy::EnemyData::kind`]). A
/// `static final` constant table reproduced as a `const` (the `directions`/
/// `Effect::FRAME_COUNTS` precedent).
pub const ATTACK_HIT_FRAME: [i8; 42] = [
    3, 2, 6, 2, 2, 1, 3, 4, 3, 2, 3, 4, 2, 3, 2, 2, 2, 3, 3, 3, 3, 3, 6, 3, 3, 3, 2, 2, 2, 3, 3, 2,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
];

/// One parsed monster template — the instance fields of a Java `EnemyType` (`j`).
/// A per-instance clone lands in [`crate::enemy::EnemyData::stats`] (see the module
/// header); the sole mutable owner is [`EnemyTypeState::types`].
#[derive(Debug, Clone, Default)]
pub struct EnemyTypeData {
    /// `public char[] name;` (`j.a`) — display name (DEFERRED ASCII id indirection).
    pub name: Vec<u16>,
    /// `public byte size;` (`j.a`) — draw/footprint size (0..3; 2 = double-wide boss).
    pub size: i8,
    /// `public byte elemColor;` (`j.b`) — nameplate colour index.
    pub elem_color: i8,
    /// `public byte element;` (`j.c`) — element index (keys the element table).
    pub element: i8,
    /// `public byte aiType;` (`j.d`) — AI behaviour (0/1 melee, 2 ranged, 3 caster).
    pub ai_type: i8,
    /// `public boolean relentless;` (`j.a`) — pursues across the whole map when engaged.
    pub relentless: bool,
    /// `public boolean armored;` (`j.b`) — halves incoming hero damage.
    pub armored: bool,
    /// `public boolean summonsAllies;` (`j.c`) — respawns copies while engaged.
    pub summons_allies: bool,
    /// `public boolean ambush;` (`j.d`) — spawns hidden until it acts.
    pub ambush: bool,
    /// `public byte summonWardElement;` (`j.e`) — guardian element that does not
    /// provoke a summon.
    pub summon_ward_element: i8,
    /// `public byte level;` (`j.f`) — monster level.
    pub level: i8,
    /// `public short maxHp;` (`j.a`) — maximum hit points.
    pub max_hp: i16,
    /// `public short attack;` (`j.b`) — base attack value.
    pub attack: i16,
    /// `public short defense;` (`j.c`) — defense subtracted from incoming damage.
    pub defense: i16,
    /// `public short evasion;` (`j.d`) — evasion term in the hero's hit-chance roll.
    pub evasion: i16,
    /// `public byte sightRange;` (`j.g`) — sight range in tiles.
    pub sight_range: i8,
    /// `public byte attackDelay;` (`j.h`) — frames between attacks.
    pub attack_delay: i8,
    /// `public byte hurtDelay;` (`j.i`) — recovery frames after being hit.
    pub hurt_delay: i8,
    /// `public short expReward;` (`j.e`) — experience awarded on death.
    pub exp_reward: i16,
    /// `public byte[] dropTable;` (`j.b`) — loot table (flat 3-byte records).
    pub drop_table: Vec<i8>,
    /// `public byte walkFrames;` (`j.j`) — number of walk-animation frames.
    pub walk_frames: i8,
    /// `public byte attackFrames;` (`j.k`) — number of attack-animation frames.
    pub attack_frames: i8,
    /// `public byte castFrames;` (`j.l`) — number of cast-animation frames.
    pub cast_frames: i8,
    /// `public byte dieFrames;` (`j.m`) — number of death-animation frames.
    pub die_frames: i8,
}

/// Java `j` / `EnemyType` state — its one mutable static (`java/reconstruction/
/// ownership.tsv`). `attackHitFrame` is a `static final` constant → the module
/// [`ATTACK_HIT_FRAME`] const, so it is not carried here.
#[derive(Debug, Default)]
pub struct EnemyTypeState {
    /// `public static EnemyType[] types;` (`j.a`) — all parsed templates, indexed by
    /// stat-row ([`crate::enemy::EnemyData::stat_row`]). `None` == Java null (before
    /// [`alloc`]).
    pub types: Option<Vec<Option<EnemyTypeData>>>,
}

/// The DEFERRED name-id indirection: keep the raw ASCII id chars (mirrors
/// [`crate::item`]'s `ascii_string_chars`). `new String(data, offset, len)` +
/// `FontManager.getStringChars` resolution is DEFERRED.
fn ascii_string_chars(data: &[i8], offset: i32, len: i32) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::with_capacity(len.max(0) as usize);
    let mut i: i32 = 0;
    while i < len {
        out.push((data[offset.wrapping_add(i) as usize] as i32 & 255) as u16);
        i = i.wrapping_add(1);
    }
    out
}

/// `public static final void alloc(int count)` (`j.a:(I)V => []`). Allocates the
/// template array for `count` monster kinds.
pub fn alloc(s: &mut EnemyTypeState, count: i32) {
    // types = new EnemyType[count];
    s.types = Some((0..count).map(|_| None).collect());
}

/// `public static final void parse(byte[] data, byte recordIndex, byte slot)`
/// (`j.a:([BBB)V`). Decodes one variable-length `/enm/data` record (skipping the
/// first `recordIndex` records) into `types[slot]`.
///
/// **DEFERRED name.** `name = FontManager.getStringChars(new String(data, namePos,
/// nameLen))` keeps the raw ASCII id chars (the FontManager/StringTable resolution is
/// DEFERRED, exactly as [`crate::item::parse_name`]); the cursor still advances past
/// the name, so every following field decodes faithfully.
pub fn parse(s: &mut EnemyTypeState, data: &[i8], record_index: i8, slot: i8) {
    // int cursor = 1;
    let mut cursor: i32 = 1;
    // for (int i = 0; i < recordIndex; i++) cursor += 2 + ByteUtil.readU16(data, cursor);
    let mut i: i32 = 0;
    while i < record_index as i32 {
        cursor =
            cursor.wrapping_add(2i32.wrapping_add(crate::byte_util::read_u16(data, cursor) as i32));
        i = i.wrapping_add(1);
    }
    // EnemyType template = new EnemyType();
    let mut template = EnemyTypeData::default();
    // int nameLenPos = cursor + 2 + 1; int namePos = nameLenPos + 1;
    let name_len_pos = cursor.wrapping_add(2).wrapping_add(1);
    let name_pos = name_len_pos.wrapping_add(1);
    // byte nameLen = data[nameLenPos];
    let name_len = data[name_len_pos as usize];
    // template.name = FontManager.getStringChars(new String(data, namePos, (int) nameLen));
    //   DEFERRED text resolution: keep the ASCII-decimal id chars (indirection).
    template.name = ascii_string_chars(data, name_pos, name_len as i32);
    // int typesPos = namePos + nameLen; int flagsPos = typesPos + 1;
    let types_pos = name_pos.wrapping_add(name_len as i32);
    let flags_pos = types_pos.wrapping_add(1);
    // byte packedTypes = data[typesPos];
    let packed_types = data[types_pos as usize];
    // template.size = (byte) ((packedTypes >> 6) & 3);
    template.size = (ishr(packed_types as i32, 6) & 3) as i8;
    // template.elemColor = (byte) ((packedTypes >> 4) & 3);
    template.elem_color = (ishr(packed_types as i32, 4) & 3) as i8;
    // template.element = (byte) ((packedTypes >> 2) & 3);
    template.element = (ishr(packed_types as i32, 2) & 3) as i8;
    // template.aiType = (byte) (packedTypes & 3);
    template.ai_type = (packed_types as i32 & 3) as i8;
    // int levelPos = flagsPos + 1; byte packedFlags = data[flagsPos];
    let level_pos = flags_pos.wrapping_add(1);
    let packed_flags = data[flags_pos as usize];
    // template.relentless = ((packedFlags >> 3) & 1) == 1;
    template.relentless = (ishr(packed_flags as i32, 3) & 1) == 1;
    // template.armored = ((packedFlags >> 2) & 1) == 1;
    template.armored = (ishr(packed_flags as i32, 2) & 1) == 1;
    // template.summonsAllies = ((packedFlags >> 1) & 1) == 1;
    template.summons_allies = (ishr(packed_flags as i32, 1) & 1) == 1;
    // template.ambush = (packedFlags & 1) == 1;
    template.ambush = (packed_flags as i32 & 1) == 1;
    // if (template.summonsAllies) template.summonWardElement = (byte) ((packedFlags >> 6) & 3);
    if template.summons_allies {
        template.summon_ward_element = (ishr(packed_flags as i32, 6) & 3) as i8;
    }
    // int hpPos = levelPos + 1; template.level = data[levelPos];
    let hp_pos = level_pos.wrapping_add(1);
    template.level = data[level_pos as usize];
    // template.maxHp = ByteUtil.readU16(data, hpPos);
    template.max_hp = crate::byte_util::read_u16(data, hp_pos);
    // int attackPos = hpPos + 2; template.attack = ByteUtil.readU16(data, attackPos);
    let attack_pos = hp_pos.wrapping_add(2);
    template.attack = crate::byte_util::read_u16(data, attack_pos);
    // int defensePos = attackPos + 2; template.defense = ByteUtil.readU16(data, defensePos);
    let defense_pos = attack_pos.wrapping_add(2);
    template.defense = crate::byte_util::read_u16(data, defense_pos);
    // int evasionPos = defensePos + 2; template.evasion = ByteUtil.readU16(data, evasionPos);
    let evasion_pos = defense_pos.wrapping_add(2);
    template.evasion = crate::byte_util::read_u16(data, evasion_pos);
    // int sightPos = evasionPos + 2; int attackDelayPos = sightPos + 1;
    let sight_pos = evasion_pos.wrapping_add(2);
    let attack_delay_pos = sight_pos.wrapping_add(1);
    // template.sightRange = data[sightPos];
    template.sight_range = data[sight_pos as usize];
    // int hurtDelayPos = attackDelayPos + 1; template.attackDelay = data[attackDelayPos];
    let hurt_delay_pos = attack_delay_pos.wrapping_add(1);
    template.attack_delay = data[attack_delay_pos as usize];
    // int expPos = hurtDelayPos + 1; template.hurtDelay = data[hurtDelayPos];
    let exp_pos = hurt_delay_pos.wrapping_add(1);
    template.hurt_delay = data[hurt_delay_pos as usize];
    // template.expReward = ByteUtil.readU16(data, expPos);
    template.exp_reward = crate::byte_util::read_u16(data, exp_pos);
    // int dropCountPos = expPos + 2;
    let drop_count_pos = exp_pos.wrapping_add(2);
    // template.dropTable = new byte[3 * data[dropCountPos]];
    let drop_len = 3i32.wrapping_mul(data[drop_count_pos as usize] as i32);
    let mut drop_table = vec![0i8; drop_len as usize];
    // System.arraycopy(data, dropCountPos + 1, template.dropTable, 0, template.dropTable.length);
    let src = drop_count_pos.wrapping_add(1) as usize;
    drop_table.copy_from_slice(&data[src..src + drop_len as usize]);
    template.drop_table = drop_table;
    // types[slot] = template;
    s.types.as_mut().expect("EnemyType.types null in parse")[slot as usize] = Some(template);
}

/// `public static final void bindSprites(byte kind)` (`j.a:(B)V`). Binds walk/attack/
/// cast frame counts from the standard sprite bank `AssetCache.enemyFrames`. Not
/// driven in this slice (the enemy sprite banks are DEFERRED-loaded); ported faithfully
/// against the bank layout.
pub fn bind_sprites(g: &mut Game, kind: i8) {
    let Game {
        enemy_type,
        asset_cache,
        ..
    } = g;
    let frames = asset_cache
        .enemy_frames
        .as_ref()
        .expect("AssetCache.enemyFrames null in bindSprites");
    let template = enemy_type
        .types
        .as_mut()
        .expect("EnemyType.types null in bindSprites")[kind as usize]
        .as_mut()
        .expect("EnemyType.types[kind] null in bindSprites");
    // byte[] walkFrames = (byte[]) AssetCache.enemyFrames[(kind * 12) + 0];
    let walk = frames[(kind as i32).wrapping_mul(12).wrapping_add(0) as usize].as_ref();
    // Debug.assertTrue(walkFrames != null);
    debug::assert_true(walk.is_some());
    // template.walkFrames = walkFrames[0];
    template.walk_frames = walk.expect("enemyFrames walk bank null")[0];
    // byte[] attackFrames = (byte[]) AssetCache.enemyFrames[(kind * 12) + 4];
    let atk = frames[(kind as i32).wrapping_mul(12).wrapping_add(4) as usize].as_ref();
    // Debug.assertTrue(attackFrames != null);
    debug::assert_true(atk.is_some());
    // template.attackFrames = attackFrames[0];
    template.attack_frames = atk.expect("enemyFrames attack bank null")[0];
    // byte[] castFrames = (byte[]) AssetCache.enemyFrames[(kind * 12) + 8];
    let cast = frames[(kind as i32).wrapping_mul(12).wrapping_add(8) as usize].as_ref();
    // Debug.assertTrue(castFrames != null);
    debug::assert_true(cast.is_some());
    // template.castFrames = castFrames[0];
    template.cast_frames = cast.expect("enemyFrames cast bank null")[0];
}

/// `public static final void bindSpritesBoss(byte kind)` (`j.b:(B)V`). Binds frame
/// counts for a multi-tile boss from the 16-wide `AssetCache.bossFrames` bank
/// (missing banks → `-1`). Not driven in this slice (`Boss` is a later batch); ported
/// faithfully against the bank layout.
pub fn bind_sprites_boss(g: &mut Game, kind: i8) {
    let Game {
        enemy_type,
        asset_cache,
        ..
    } = g;
    let frames = asset_cache
        .boss_frames
        .as_ref()
        .expect("AssetCache.bossFrames null in bindSpritesBoss");
    let template = enemy_type
        .types
        .as_mut()
        .expect("EnemyType.types null in bindSpritesBoss")[kind as usize]
        .as_mut()
        .expect("EnemyType.types[kind] null in bindSpritesBoss");
    // byte[] walkFrames = (byte[]) AssetCache.bossFrames[(kind * 16) + 0];
    let walk = frames[(kind as i32).wrapping_mul(16).wrapping_add(0) as usize].as_ref();
    // Debug.assertTrue(walkFrames != null);
    debug::assert_true(walk.is_some());
    // template.walkFrames = walkFrames[0];
    template.walk_frames = walk.expect("bossFrames walk bank null")[0];
    // byte[] attackFrames = (byte[]) AssetCache.bossFrames[(kind * 16) + 4];
    let atk = frames[(kind as i32).wrapping_mul(16).wrapping_add(4) as usize].as_ref();
    // if (attackFrames != null) template.attackFrames = attackFrames[0]; else template.attackFrames = -1;
    template.attack_frames = match atk {
        Some(a) => a[0],
        None => -1,
    };
    // byte[] castFrames = (byte[]) AssetCache.bossFrames[(kind * 16) + 12];
    let cast = frames[(kind as i32).wrapping_mul(16).wrapping_add(12) as usize].as_ref();
    // if (castFrames != null) template.castFrames = castFrames[0]; else template.castFrames = -1;
    template.cast_frames = match cast {
        Some(c) => c[0],
        None => -1,
    };
    // byte[] dieFrames = (byte[]) AssetCache.bossFrames[(kind * 16) + 8];
    let die = frames[(kind as i32).wrapping_mul(16).wrapping_add(8) as usize].as_ref();
    // if (dieFrames != null) template.dieFrames = dieFrames[0]; else template.dieFrames = -1;
    template.die_frames = match die {
        Some(d) => d[0],
        None => -1,
    };
}
