package defpackage;

/* renamed from: j */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:j.class */
/**
 * Stat template for one kind of monster, parsed from the {@code /enm/data} blob.
 * Every {@link Enemy}/{@link Boss} instance holds a reference to its shared
 * template ({@link Enemy#stats}) which supplies name, size, element, AI type,
 * combat stats (HP/attack/defense/evasion), timing (sight/attack/hurt delays),
 * the level, the loot {@link #dropTable}, and the per-state animation frame
 * spans (bound later from the loaded sprite banks). The full set of templates
 * lives in the static {@link #types} array.
 */
public final class EnemyType {
    /* renamed from: a */
    /** All parsed templates, indexed by stat-row (see {@link Enemy#statRow}). */
    public static EnemyType[] types;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Per-kind frame at which a melee attack connects (indexed by {@link Enemy#kind}). */
    public static final byte[] attackHitFrame = {3, 2, 6, 2, 2, 1, 3, 4, 3, 2, 3, 4, 2, 3, 2, 2, 2, 3, 3, 3, 3, 3, 6, 3, 3, 3, 2, 2, 2, 3, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1};

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Display name (decoded characters). */
    public char[] name;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Draw/footprint size (0..3; 2 = double-wide multi-tile boss). */
    public byte size;

    /* renamed from: b */
    /** Nameplate colour index. */
    public byte elemColor;

    /* renamed from: c */
    /** Element index (keys the {@link Directions#a} element-multiplier table). */
    public byte element;

    /* renamed from: d */
    /** AI behaviour type (0/1 melee-chaser, 2 ranged, 3 caster). */
    public byte aiType;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** When engaged, pursues across the whole map (search radius 100 vs 8). */
    public boolean relentless;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Armoured: halves incoming hero damage. */
    public boolean armored;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Summoner: respawns copies of itself while engaged. */
    public boolean summonsAllies;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /** Ambusher: spawns hidden until it acts. */
    public boolean ambush;

    /* renamed from: e */
    /** For summoners, the guardian element that does NOT provoke a summon. */
    public byte summonWardElement;

    /* renamed from: f */
    /** Monster level (drives exp/money/rare-drop scaling vs the hero's level). */
    public byte level;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Maximum hit points. */
    public short maxHp;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Base attack (damage) value. */
    public short attack;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Defense subtracted from incoming hero damage. */
    public short defense;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /** Evasion term in the hero's hit-chance roll. */
    public short evasion;

    /* renamed from: g */
    /** Sight range in tiles (aggro distance). */
    public byte sightRange;

    /* renamed from: h */
    /** Frames between attacks (attack-cooldown base). */
    public byte attackDelay;

    /* renamed from: i */
    /** Recovery frames after being hit (hurt-cooldown base). */
    public byte hurtDelay;

    /* JADX INFO: renamed from: e, reason: collision with other field name */
    /** Experience awarded on death. */
    public short expReward;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Loot table as flat 3-byte records {itemKind, param, weight}. */
    public byte[] dropTable;

    /* renamed from: j */
    /** Number of walk-animation frames. */
    public byte walkFrames;

    /* renamed from: k */
    /** Number of attack-animation frames. */
    public byte attackFrames;

    /* renamed from: l */
    /** Number of cast-animation frames. */
    public byte castFrames;

    /* renamed from: m */
    /** Number of death-animation frames. */
    public byte dieFrames;

    /* renamed from: a */
    /** Allocates the template array for {@code count} monster kinds. */
    public static final void alloc(int count) {
        types = new EnemyType[count];
    }

    /* renamed from: a */
    /**
     * Decodes one variable-length {@code /enm/data} record (skipping the first
     * {@code recordIndex} records) into {@code types[slot]}.
     */
    public static final void parse(byte[] data, byte recordIndex, byte slot) {
        int cursor = 1;
        for (int i = 0; i < recordIndex; i++) {
            cursor += 2 + defpackage.ByteUtil.readU16(data, cursor);
        }
        EnemyType template = new EnemyType();
        int nameLenPos = cursor + 2 + 1;
        int namePos = nameLenPos + 1;
        byte nameLen = data[nameLenPos];
        template.name = FontManager.getStringChars(new String(data, namePos, (int) nameLen));
        int typesPos = namePos + nameLen;
        int flagsPos = typesPos + 1;
        byte packedTypes = data[typesPos];
        template.size = (byte) ((packedTypes >> 6) & 3);
        template.elemColor = (byte) ((packedTypes >> 4) & 3);
        template.element = (byte) ((packedTypes >> 2) & 3);
        template.aiType = (byte) (packedTypes & 3);
        int levelPos = flagsPos + 1;
        byte packedFlags = data[flagsPos];
        template.relentless = ((packedFlags >> 3) & 1) == 1;
        template.armored = ((packedFlags >> 2) & 1) == 1;
        template.summonsAllies = ((packedFlags >> 1) & 1) == 1;
        template.ambush = (packedFlags & 1) == 1;
        if (template.summonsAllies) {
            template.summonWardElement = (byte) ((packedFlags >> 6) & 3);
        }
        int hpPos = levelPos + 1;
        template.level = data[levelPos];
        template.maxHp = defpackage.ByteUtil.readU16(data, hpPos);
        int attackPos = hpPos + 2;
        template.attack = defpackage.ByteUtil.readU16(data, attackPos);
        int defensePos = attackPos + 2;
        template.defense = defpackage.ByteUtil.readU16(data, defensePos);
        int evasionPos = defensePos + 2;
        template.evasion = defpackage.ByteUtil.readU16(data, evasionPos);
        int sightPos = evasionPos + 2;
        int attackDelayPos = sightPos + 1;
        template.sightRange = data[sightPos];
        int hurtDelayPos = attackDelayPos + 1;
        template.attackDelay = data[attackDelayPos];
        int expPos = hurtDelayPos + 1;
        template.hurtDelay = data[hurtDelayPos];
        template.expReward = defpackage.ByteUtil.readU16(data, expPos);
        int dropCountPos = expPos + 2;
        template.dropTable = new byte[3 * data[dropCountPos]];
        System.arraycopy(data, dropCountPos + 1, template.dropTable, 0, template.dropTable.length);
        types[slot] = template;
    }

    /* renamed from: a */
    /** Binds walk/attack/cast frame counts from the standard sprite bank. */
    public static final void bindSprites(byte kind) {
        EnemyType template = types[kind];
        byte[] walkFrames = (byte[]) AssetCache.enemyFrames[(kind * 12) + 0];
        Debug.assertTrue(walkFrames != null);
        template.walkFrames = walkFrames[0];
        byte[] attackFrames = (byte[]) AssetCache.enemyFrames[(kind * 12) + 4];
        Debug.assertTrue(attackFrames != null);
        template.attackFrames = attackFrames[0];
        byte[] castFrames = (byte[]) AssetCache.enemyFrames[(kind * 12) + 8];
        Debug.assertTrue(castFrames != null);
        template.castFrames = castFrames[0];
    }

    /* renamed from: b */
    /** Binds frame counts for a multi-tile boss from the 16-wide boss sprite bank. */
    public static final void bindSpritesBoss(byte kind) {
        EnemyType template = types[kind];
        byte[] walkFrames = (byte[]) AssetCache.bossFrames[(kind * 16) + 0];
        Debug.assertTrue(walkFrames != null);
        template.walkFrames = walkFrames[0];
        byte[] attackFrames = (byte[]) AssetCache.bossFrames[(kind * 16) + 4];
        if (attackFrames != null) {
            template.attackFrames = attackFrames[0];
        } else {
            template.attackFrames = (byte) -1;
        }
        byte[] castFrames = (byte[]) AssetCache.bossFrames[(kind * 16) + 12];
        if (castFrames != null) {
            template.castFrames = castFrames[0];
        } else {
            template.castFrames = (byte) -1;
        }
        byte[] dieFrames = (byte[]) AssetCache.bossFrames[(kind * 16) + 8];
        if (dieFrames != null) {
            template.dieFrames = dieFrames[0];
        } else {
            template.dieFrames = (byte) -1;
        }
    }
}
