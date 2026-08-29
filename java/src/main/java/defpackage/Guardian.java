package defpackage;

import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/* renamed from: p */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:p.class */
/**
 * A summoned elemental companion (the hero's guardian). It carries its own
 * {@link #level}/{@link #exp} progression, two equippable skill slots
 * ({@link #skillSlotA}/{@link #skillSlotB}) gated by {@link #skillUnlockLevel},
 * and a per-skill charge/cooldown ({@link #skillCharges}). Casting runs a small
 * finite-state machine ({@link #castState}: 0 idle &rarr; 1 charge &rarr; 2
 * active &rarr; 3 finish) that each finish-frame invokes the big
 * {@link #applySkillEffect} dispatcher — dealing area damage to nearby enemies
 * (scaled by the element multiplier), buffing the hero, and spawning cast
 * effects and SFX.
 */
public final class Guardian extends Entity implements Directions {
    /* renamed from: a */
    /** Starting level per guardian type. */
    public static final short[] levelStartTable = {1, 1, 1, 1, 10, 20};

    /* renamed from: h */
    /** Minimum guardian level to unlock skill slot index 0..3. */
    public static final byte[] skillUnlockLevel = {1, 9, 20, 30};

    /* renamed from: b */
    /** MP/charge cost per (type*3 + skill). */
    public static final short[] skillCostTable = {56, 280, 280, 220, 160, 270, 56, 270, 220, 80, 270, 270, 80, 270, 220, 80, 270, 270};

    /* renamed from: i */
    /** Cast (channel) length per (type*3 + skill). */
    private static final byte[] castDurationTable = {0, 0, 10, 4, 4, 10, 0, 4, 0, 4, 10, 8, 0, 6, 6, 0, 10, 8};

    /* renamed from: d */
    /** Effect duration per (type*3 + skill). */
    private static final short[] effectDurationTable = {16, 9, 161, 9, 10, 81, 16, 9, 13, 25, 81, 8, 31, 81, 16, 31, 87, 8};

    /* renamed from: j */
    /** Two random AoE tile offsets [dx0,dy0,dx1,dy1] (scratch for scatter skills). */
    private byte[] aoeOffsets;

    /* renamed from: f */
    /** Guardian type (0..5; folds to an element via {@link #element()}). */
    public byte type;

    /* renamed from: g */
    /** Skill equipped in slot A (skill index, or -1). */
    public byte skillSlotA;

    /* JADX INFO: renamed from: h, reason: collision with other field name */
    /** Skill equipped in slot B (skill index, or -1). */
    public byte skillSlotB;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Guardian level. */
    public short level;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Accumulated experience. */
    public int exp;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Experience required for the next level. */
    public int expToNext;

    /* JADX INFO: renamed from: i, reason: collision with other field name */
    /** Cast state machine: 0 idle, 1 charge, 2 active, 3 finish. */
    public byte castState;

    /* JADX INFO: renamed from: j, reason: collision with other field name */
    /** Direction the current cast is aimed. */
    public byte castDir;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Cast animation sub-frame counter. */
    private short animFrame;

    /* renamed from: e */
    /** Effect progress counter during the finish phase. */
    private short effectFrame;

    /* renamed from: k */
    /** Total cast (channel) frames for the active skill. */
    private byte castTotalFrames;

    /* renamed from: l */
    /** Remaining cast frames. */
    private byte castRemaining;

    /* renamed from: m */
    /** Skill slot currently being cast. */
    private byte activeSkillSlot;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /** True once the current skill's effect has finished. */
    private boolean effectDone;

    /* renamed from: c */
    /** Per-skill charge/cooldown counters (0 = ready to cast). */
    public short[] skillCharges;

    public Guardian(short pixelX, short pixelY, byte type) {
        super(pixelX, pixelY, (byte) 8, (byte) 8);
        this.aoeOffsets = new byte[4];
        this.type = type;
        this.skillCharges = new short[3];
        this.skillCharges[0] = skillCostTable[(type * 3) + 0];
        this.skillCharges[1] = skillCostTable[(type * 3) + 1];
        this.skillCharges[2] = skillCostTable[(type * 3) + 2];
        this.level = levelStartTable[type];
        recomputeExpToNext();
        this.exp = 0;
        clearSlots();
        equipSkill(true, (byte) 0, true);
        equipSkill(false, (byte) 1, true);
    }

    /* renamed from: a */
    /** Adds experience, leveling up as many times as the curve allows. */
    public final void addExp(int amount) {
        this.exp += amount;
        while (this.exp >= this.expToNext) {
            this.exp -= this.expToNext;
            levelUp();
        }
    }

    /* renamed from: f */
    /** Advances one level, recomputes the curve, and unlocks slot 1 at its level. */
    private final void levelUp() {
        this.level = (short) (this.level + 1);
        recomputeExpToNext();
        if (this.level == skillUnlockLevel[1]) {
            equipSkill(false, (byte) 1, true);
        }
    }

    /* renamed from: a */
    /** Recomputes {@link #expToNext} from the level curve (L^3 - L^2 + 80L). */
    public final void recomputeExpToNext() {
        this.expToNext = (((this.level * this.level) * this.level) - (this.level * this.level)) + (this.level * 80);
    }

    /* renamed from: c */
    /** Cancels any active cast (returns to idle). */
    public final void dismiss() {
        this.castState = (byte) 0;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Folds the guardian type (0..5) to its element (1, 2 or 3). */
    public final byte element() {
        if (this.type == 0 || this.type == 3) {
            return (byte) 1;
        }
        if (this.type == 1 || this.type == 4) {
            return (byte) 2;
        }
        return (this.type == 2 || this.type == 5) ? (byte) 3 : (byte) 0;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Whether the guardian is early in a cast (less than 10 frames elapsed). */
    public final boolean isBusy() {
        return this.castTotalFrames - this.castRemaining < 10;
    }

    /* renamed from: d */
    /** Clears both skill slots. */
    public final void clearSlots() {
        equipSkill(true, (byte) -1, true);
        equipSkill(false, (byte) -1, true);
    }

    /* renamed from: a */
    /**
     * Equips (or clears with {@code skill == -1}) a skill in slot A or B. Skills
     * are gated by {@link #skillUnlockLevel}; when {@code refill} is set the
     * skill's charge counter is topped up. Returns whether the change was applied.
     */
    public final boolean equipSkill(boolean slotA, byte skill, boolean refill) {
        if (skill == -1) {
            if (slotA) {
                this.skillSlotA = skill;
                return true;
            }
            this.skillSlotB = skill;
            return true;
        }
        if (skillUnlockLevel[skill] > this.level) {
            return false;
        }
        if (slotA) {
            this.skillSlotA = skill;
            if (!refill || skill == -1) {
                return true;
            }
            this.skillCharges[skill] = skillCostTable[(this.type * 3) + skill];
            return true;
        }
        this.skillSlotB = skill;
        if (!refill || skill == -1) {
            return true;
        }
        this.skillCharges[skill] = skillCostTable[(this.type * 3) + skill];
        return true;
    }

    /* renamed from: a */
    /**
     * Begins casting the slot-A or slot-B skill (if charged) at the tile one step
     * from ({@code originTileX},{@code originTileY}) in direction {@code dir}.
     */
    public final void castSkill(boolean useSlotA, byte dir, int originTileX, int originTileY) {
        if (useSlotA && this.skillSlotA >= 0 && this.skillSlotA <= 2 && this.skillCharges[this.skillSlotA] == 0) {
            this.activeSkillSlot = this.skillSlotA;
            this.skillCharges[this.skillSlotA] = skillCostTable[(this.type * 3) + this.skillSlotA];
        } else {
            if (useSlotA || this.skillSlotB < 0 || this.skillSlotB > 2 || this.skillCharges[this.skillSlotB] != 0) {
                return;
            }
            this.activeSkillSlot = this.skillSlotB;
            this.skillCharges[this.skillSlotB] = skillCostTable[(this.type * 3) + this.skillSlotB];
        }
        this.castState = (byte) 1;
        this.animFrame = (short) -1;
        this.castTotalFrames = castDurationTable[(this.type * 3) + this.activeSkillSlot];
        this.castRemaining = this.castTotalFrames;
        this.effectDone = false;
        setPixelPos((short) ((originTileX + Directions.dirDx[dir]) * 16), (short) ((originTileY + Directions.dirDy[dir]) * 16));
        this.castDir = dir;
        ((Entity) this).tileY = (byte) (((Entity) this).pixelY >> 4);
        ((Entity) this).tileX = (byte) (((Entity) this).pixelX >> 4);
        ((Entity) this).offGridY = false;
        ((Entity) this).offGridX = false;
    }

    /* renamed from: e */
    /** Per-tick update: drains skill charges, steps the cast FSM, and applies the effect. */
    public final void update() {
        if ((this.castState == 0 || this.activeSkillSlot != this.skillSlotA) && this.skillSlotA != -1 && this.skillCharges[this.skillSlotA] > 0) {
            this.skillCharges[this.skillSlotA] = (short) (this.skillCharges[this.skillSlotA] - 1);
        }
        if ((this.castState == 0 || this.activeSkillSlot != this.skillSlotB) && this.skillSlotB != -1 && this.skillCharges[this.skillSlotB] > 0) {
            this.skillCharges[this.skillSlotB] = (short) (this.skillCharges[this.skillSlotB] - 1);
        }
        switch (this.castState) {
            case 0:
                return;
            case 1:
                this.animFrame = (short) (this.animFrame + 1);
                if (this.animFrame >= 10) {
                    if (this.castRemaining > 0) {
                        this.castState = (byte) 2;
                    } else {
                        this.castState = (byte) 3;
                        this.animFrame = (short) 0;
                        this.effectFrame = (short) 0;
                    }
                    this.animFrame = (short) 0;
                }
                break;
            case 2:
                this.animFrame = (short) (this.animFrame + 1);
                this.castRemaining = (byte) (this.castRemaining - 1);
                if (this.animFrame >= 4) {
                    this.animFrame = (short) 0;
                }
                if (this.castRemaining <= 0) {
                    this.castState = (byte) 3;
                    this.animFrame = (short) 0;
                    this.effectFrame = (short) 0;
                }
                break;
            case 3:
                this.animFrame = (short) (this.animFrame + 1);
                if (this.animFrame >= 11) {
                    this.animFrame = (short) 11;
                    if (this.effectDone) {
                        this.castState = (byte) 0;
                        this.castDir = (byte) 0;
                    }
                }
                break;
        }
        if (this.castState == 3) {
            applySkillEffect(this.type, this.activeSkillSlot);
            this.effectFrame = (short) (this.effectFrame + 1);
        }
    }

    /* renamed from: a */
    /**
     * Dispatches the effect of skill {@code skill} for guardian type
     * {@code guardianType}: area damage to nearby enemies, hero buffs (attack,
     * defense, invincibility, reflect, regen, heals), cast effects and SFX. Runs
     * once per finish-phase frame ({@link #effectFrame}); marks {@link #effectDone}
     * when the effect duration is reached.
     */
    private final void applySkillEffect(byte guardianType, byte skill) {
        Hero hero = GameState.hero();
        switch (guardianType) {
            case 0:
                switch (skill) {
                    case 0:
                        dealForwardStrikeDamage();
                        spawnForwardStrikeEffects();
                        break;
                    case 1:
                        if (this.effectFrame == 0) {
                            hero.addFloater(new GuardianCastFx((short) 0, (short) 8, guardianType, skill));
                            hero.applyStatus((byte) 5);
                            hero.attackUp = true;
                            AudioManager.playSfx((byte) 21, false);
                        }
                        break;
                    case 2:
                        if (this.effectFrame == 0) {
                            hero.addFloater(new GuardianCastFx((short) 0, (short) 160, guardianType, skill));
                            hero.reflectDamage = true;
                            AudioManager.playSfx((byte) 21, false);
                        }
                        if (this.effectFrame == effectDurationTable[(guardianType * 3) + skill]) {
                            hero.reflectDamage = false;
                        }
                        break;
                }
                break;
            case 1:
                switch (skill) {
                    case 0:
                        if (this.effectFrame == 0) {
                            hero.addFloater(new GuardianCastFx((short) 0, (short) 8, guardianType, skill));
                            hero.addHpPercent(30);
                            AudioManager.playSfx((byte) 20, false);
                        }
                        break;
                    case 1:
                        if (this.effectFrame == 0) {
                            hero.addFloater(new GuardianCastFx((short) 0, (short) 9, guardianType, skill));
                            hero.addMpPercent(20);
                            AudioManager.playSfx((byte) 20, false);
                        }
                        break;
                    case 2:
                        if (this.effectFrame == 0) {
                            hero.addFloater(new GuardianCastFx((short) 0, (short) 80, guardianType, skill));
                            hero.addFloater(new GuardianCastFx((short) 4, (short) 8, guardianType, (byte) 0));
                            hero.addFloater(new GuardianCastFx((short) 24, (short) 8, guardianType, (byte) 0));
                            hero.addFloater(new GuardianCastFx((short) 44, (short) 8, guardianType, (byte) 0));
                            hero.regenBoost = true;
                            AudioManager.playSfx((byte) 20, false);
                        }
                        if (this.effectFrame == effectDurationTable[(guardianType * 3) + skill]) {
                            hero.regenBoost = false;
                        }
                        break;
                }
                break;
            case 2:
                switch (skill) {
                    case 0:
                        dealQuakeStrikeDamage();
                        spawnQuakeStrikeEffects();
                        break;
                    case 1:
                        if (this.effectFrame == 0) {
                            hero.addFloater(new GuardianCastFx((short) 0, (short) 8, guardianType, skill));
                            hero.applyStatus((byte) 6);
                            hero.defenseUp = true;
                            AudioManager.playSfx((byte) 21, false);
                        }
                        break;
                    case 2:
                        if (this.effectFrame == 0) {
                            hero.addFloater(new GuardianCastFx((short) 0, (short) 12, guardianType, skill));
                            ((Battler) hero).statuses.removeAllElements();
                            hero.attackUp = false;
                            hero.defenseUp = false;
                            AudioManager.playSfx((byte) 21, false);
                        }
                        break;
                }
                break;
            case 3:
                switch (skill) {
                    case 0:
                        dealRingPulse(4, 3, 3, (this.level * 3) + 35 + GameState.hero().spirit);
                        dealRingPulse(10, 3, 3, (this.level * 3) + 35 + GameState.hero().spirit);
                        dealRingPulse(16, 3, 3, (this.level * 3) + 35 + GameState.hero().spirit);
                        spawnRingEffects((byte) 5, 0, 3);
                        spawnRingEffects((byte) 5, 6, 3);
                        spawnRingEffects((byte) 5, 12, 3);
                        if (this.effectFrame == 0 || this.effectFrame == 6 || this.effectFrame == 12) {
                            AudioManager.playSfx((byte) 16, false);
                        }
                        break;
                    case 1:
                        randomizeAoeOffsets(2);
                        dealRandomAoeDamage((short) 80, this.level + 45 + ((GameState.hero().spirit * 3) / 2));
                        spawnScatterEffects((int) ((Entity) this).pixelX, (int) ((Entity) this).pixelY, (short) 80, (byte) 6);
                        if (this.effectFrame % 8 == 0) {
                            AudioManager.playSfx((byte) 16, false);
                        }
                        break;
                    case 2:
                        if (this.effectFrame == 0) {
                            banishMatchingEnemies((byte) 7);
                            spawnBanishEffects((short) 7);
                            AudioManager.playSfx((byte) 16, false);
                        }
                        break;
                }
                break;
            case 4:
                switch (skill) {
                    case 0:
                        dealRingPulse(4, 5, 3, (this.level * 3) + 35 + GameState.hero().spirit);
                        dealRingPulse(10, 5, 3, (this.level * 3) + 35 + GameState.hero().spirit);
                        dealRingPulse(16, 5, 3, (this.level * 3) + 35 + GameState.hero().spirit);
                        spawnRingEffects((byte) 7, 0, 5);
                        spawnRingEffects((byte) 7, 6, 5);
                        spawnRingEffects((byte) 7, 12, 5);
                        if (this.effectFrame == 0 || this.effectFrame == 6 || this.effectFrame == 12 || this.effectFrame == 18 || this.effectFrame == 24) {
                            AudioManager.playSfx((byte) 18, false);
                        }
                        break;
                    case 1:
                        randomizeAoeOffsets(2);
                        if (Math.abs(((Entity) this).tileX - ((Entity) hero).tileX) + Math.abs(((Entity) this).tileY - ((Entity) hero).tileY) <= 2) {
                            hero.invincible = true;
                        } else {
                            hero.invincible = false;
                        }
                        spawnScatterEffects((int) ((Entity) this).pixelX, (int) ((Entity) this).pixelY, (short) 80, (byte) 8);
                        if (this.effectFrame % 8 == 0) {
                            AudioManager.playSfx((byte) 18, false);
                        }
                        if (this.effectFrame == effectDurationTable[(guardianType * 3) + skill]) {
                            hero.invincible = false;
                        }
                        break;
                    case 2:
                        if (this.effectFrame == 0) {
                            hero.addFloater(new GuardianCastFx((short) 0, (short) 15, guardianType, skill));
                            hero.addHpPercent(20);
                            hero.addMpPercent(20);
                            AudioManager.playSfx((byte) 20, false);
                        }
                        break;
                }
                break;
            case 5:
                switch (skill) {
                    case 0:
                        dealRingPulse(4, 5, 3, (this.level * 3) + 35 + GameState.hero().spirit);
                        dealRingPulse(10, 5, 3, (this.level * 3) + 35 + GameState.hero().spirit);
                        dealRingPulse(16, 5, 3, (this.level * 3) + 35 + GameState.hero().spirit);
                        spawnRingEffects((byte) 4, 0, 5);
                        spawnRingEffects((byte) 4, 6, 5);
                        spawnRingEffects((byte) 4, 12, 5);
                        if (this.effectFrame == 0 || this.effectFrame == 6 || this.effectFrame == 12 || this.effectFrame == 18 || this.effectFrame == 24) {
                            AudioManager.playSfx((byte) 17, false);
                        }
                        break;
                    case 1:
                        randomizeAoeOffsets(2);
                        dealRandomAoeDamage((short) 80, this.level + 45 + ((GameState.hero().spirit * 3) / 2));
                        spawnScatterEffects((int) ((Entity) this).pixelX, (int) ((Entity) this).pixelY, (short) 80, (byte) 9);
                        if (this.effectFrame % 8 == 0) {
                            AudioManager.playSfx((byte) 17, false);
                        }
                        break;
                    case 2:
                        if (this.effectFrame == 0) {
                            banishMatchingEnemies((byte) 9);
                            spawnBanishEffects((short) 9);
                            AudioManager.playSfx((byte) 17, false);
                        }
                        break;
                }
                break;
        }
        if (this.effectFrame == effectDurationTable[(guardianType * 3) + skill]) {
            this.effectDone = true;
        }
    }

    /* renamed from: g */
    /** Deals the forward-strike skill's damage: a lunge line plus two diagonal hits. */
    private final void dealForwardStrikeDamage() {
        Entity frontEnemy;
        Entity frontEnemy2;
        Entity frontEnemy3;
        Entity frontEnemy4;
        Entity diagEnemy;
        Entity diagEnemy2;
        int damage = (this.level * 3) + 40 + GameState.hero().spirit;
        if ((this.effectFrame == 1 || this.effectFrame == 5) && (frontEnemy = neighbor(this.castDir, (byte) 0)) != null && (frontEnemy instanceof Enemy)) {
            ((Enemy) frontEnemy).takeGuardianHit(damage, element());
        }
        if ((this.effectFrame == 2 || this.effectFrame == 6) && (frontEnemy2 = neighbor(this.castDir, (byte) 1)) != null && (frontEnemy2 instanceof Enemy)) {
            ((Enemy) frontEnemy2).takeGuardianHit(damage, element());
        }
        if ((this.effectFrame == 3 || this.effectFrame == 7) && (frontEnemy3 = neighbor(this.castDir, (byte) 2)) != null && (frontEnemy3 instanceof Enemy)) {
            ((Enemy) frontEnemy3).takeGuardianHit(damage, element());
        }
        if ((this.effectFrame == 5 || this.effectFrame == 9) && (frontEnemy4 = neighbor(this.castDir, (byte) 3)) != null && (frontEnemy4 instanceof Enemy)) {
            ((Enemy) frontEnemy4).takeGuardianHit(damage, element());
        }
        if (this.effectFrame == 8 || this.effectFrame == 12) {
            int leftX = ((Entity) this).tileX + (Directions.dirDx[this.castDir] * 3) + Directions.dirDx[Directions.rotateCW[this.castDir]];
            int leftY = ((Entity) this).tileY + (Directions.dirDy[this.castDir] * 3) + Directions.dirDy[Directions.rotateCW[this.castDir]];
            int rightX = ((Entity) this).tileX + (Directions.dirDx[this.castDir] * 3) + Directions.dirDx[Directions.rotateCCW[this.castDir]];
            int rightY = ((Entity) this).tileY + (Directions.dirDy[this.castDir] * 3) + Directions.dirDy[Directions.rotateCCW[this.castDir]];
            if (leftX > 0 && leftX < GameState.map.widthTiles - 1 && leftY > 0 && leftY < GameState.map.heightTiles - 1 && (diagEnemy2 = GameState.map.occupancy[leftY][leftX]) != null && (diagEnemy2 instanceof Enemy)) {
                ((Enemy) diagEnemy2).takeGuardianHit(damage, element());
            }
            if (rightX <= 0 || rightX >= GameState.map.widthTiles - 1 || rightY <= 0 || rightY >= GameState.map.heightTiles - 1 || (diagEnemy = GameState.map.occupancy[rightY][rightX]) == null || !(diagEnemy instanceof Enemy)) {
                return;
            }
            ((Enemy) diagEnemy).takeGuardianHit(damage, element());
        }
    }

    /* renamed from: h */
    /** Deals the quake-strike skill's damage (with a screen shake) to enemies ahead. */
    private final void dealQuakeStrikeDamage() {
        Entity frontEnemy;
        Entity frontEnemy2;
        Entity frontEnemy3;
        Entity frontEnemy4;
        if (this.effectFrame >= 0 && this.effectFrame <= 5) {
            GameState.map.cameraShiftX = defpackage.ByteUtil.randRange(-4, 4);
            GameState.map.cameraShiftY = defpackage.ByteUtil.randRange(-4, 4);
        }
        int damage = (this.level * 3) + 35 + ((GameState.hero().spirit * 3) / 2);
        if ((this.effectFrame == 2 || this.effectFrame == 6) && (frontEnemy = neighbor(this.castDir, (byte) 0)) != null && (frontEnemy instanceof Enemy)) {
            ((Enemy) frontEnemy).takeGuardianHit(damage, element());
        }
        if ((this.effectFrame == 3 || this.effectFrame == 7) && (frontEnemy2 = neighbor(this.castDir, (byte) 1)) != null && (frontEnemy2 instanceof Enemy)) {
            ((Enemy) frontEnemy2).takeGuardianHit(damage, element());
        }
        if ((this.effectFrame == 4 || this.effectFrame == 8) && (frontEnemy3 = neighbor(this.castDir, (byte) 2)) != null && (frontEnemy3 instanceof Enemy)) {
            ((Enemy) frontEnemy3).takeGuardianHit(damage, element());
        }
        if ((this.effectFrame == 5 || this.effectFrame == 9) && (frontEnemy4 = neighbor(this.castDir, (byte) 3)) != null && (frontEnemy4 instanceof Enemy)) {
            ((Enemy) frontEnemy4).takeGuardianHit(damage, element());
        }
    }

    /* renamed from: a */
    /** Deals scatter damage to the two random {@link #aoeOffsets} tiles while active. */
    private final void dealRandomAoeDamage(short duration, int damage) {
        Entity target;
        Entity target2;
        GameMap map = GameState.map;
        if (this.effectFrame < duration) {
            int x0 = ((Entity) this).tileX + this.aoeOffsets[0];
            int y0 = ((Entity) this).tileY + this.aoeOffsets[1];
            if (x0 >= 0 && y0 >= 0 && x0 < map.widthTiles && y0 < map.heightTiles && (target2 = map.occupancy[y0][x0]) != null && (target2 instanceof Enemy)) {
                ((Enemy) target2).takeGuardianHit(damage, element());
            }
            int x1 = ((Entity) this).tileX + this.aoeOffsets[2];
            int y1 = ((Entity) this).tileY + this.aoeOffsets[3];
            if (x1 < 0 || y1 < 0 || x1 >= map.widthTiles || y1 >= map.heightTiles || (target = map.occupancy[y1][x1]) == null || !(target instanceof Enemy)) {
                return;
            }
            ((Enemy) target).takeGuardianHit(damage, element());
        }
    }

    /* renamed from: a */
    /**
     * Deals an expanding-ring pulse: on frames {@code startFrame}, {@code +interval},
     * ... hits all enemies at the corresponding ring distance for {@code damage}.
     */
    private final void dealRingPulse(int startFrame, int interval, int unusedRadius, int damage) {
        int sinceStart = this.effectFrame - startFrame;
        int ring = sinceStart / interval;
        if (sinceStart < 0 || sinceStart % interval != 0) {
            return;
        }
        if (ring == 0) {
            Entity target = neighbor(this.castDir, (byte) 0);
            if (target == null || !(target instanceof Enemy)) {
                return;
            }
            ((Enemy) target).takeGuardianHit(damage, element());
            return;
        }
        byte dir = 1;
        while (true) {
            byte d = dir;
            if (d > 4) {
                return;
            }
            Entity target = neighbor(d, (byte) ring);
            if (target != null && (target instanceof Enemy)) {
                ((Enemy) target).takeGuardianHit(damage, element());
            }
            dir = (byte) (d + 1);
        }
    }

    /* renamed from: a */
    /** Instantly slays same-element, non-boss enemies in the four tiles ahead. */
    private final void banishMatchingEnemies(byte slayArg) {
        if (this.effectFrame == 0) {
            for (int dist = 0; dist <= 3; dist++) {
                Entity target = neighbor(this.castDir, (byte) dist);
                if (target != null && (target instanceof Enemy) && !(target instanceof Boss) && ((Enemy) target).stats.element == element()) {
                    ((Enemy) target).slay(slayArg);
                }
            }
        }
    }

    @Override // defpackage.ck
    public final void paint(Graphics graphics, int originX, int originY) {
        int screenX = originX + ((Entity) this).pixelX + ((Entity) this).halfW;
        int screenY = originY + ((Entity) this).pixelY + ((Entity) this).halfH;
        int height = AssetCache.guardianBeam[0].getHeight();
        Image[] guardianBank = AssetCache.spriteBanks[12];
        Image piece0 = guardianBank[0];
        Image piece1 = guardianBank[1];
        Image piece2 = guardianBank[2];
        Image piece3 = guardianBank[3];
        Image beam0 = AssetCache.guardianBeam[0];
        Image beam1 = AssetCache.guardianBeam[1];
        switch (this.castState) {
            case 1:
                switch (this.animFrame) {
                    case 0:
                        graphics.drawImage(piece2, screenX, screenY, 33);
                        break;
                    case 1:
                        graphics.drawImage(piece3, screenX, screenY, 33);
                        break;
                    case 2:
                        graphics.drawImage(piece3, screenX, screenY, 33);
                        graphics.drawImage(piece0, screenX, screenY + 3, 33);
                        break;
                    case 3:
                        graphics.drawImage(piece3, screenX, screenY, 33);
                        graphics.drawImage(piece0, screenX, screenY + 3, 33);
                        graphics.drawImage(piece1, screenX, screenY + 6, 33);
                        break;
                    case 4:
                        graphics.drawImage(piece2, screenX, screenY, 33);
                        graphics.drawImage(piece1, screenX, screenY + 6, 33);
                        break;
                    case 5:
                        GameScreen.clipToWorld(graphics, screenX - 20, screenY - 50, 40, 50);
                        graphics.drawImage(beam0, screenX, screenY + ((height * 7) / 10), 33);
                        graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                        break;
                    case 6:
                        GameScreen.clipToWorld(graphics, screenX - 20, screenY - 50, 40, 50);
                        graphics.drawImage(beam0, screenX, screenY + ((height * 5) / 10), 33);
                        graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                        graphics.drawImage(piece2, screenX, screenY, 33);
                        break;
                    case 7:
                        GameScreen.clipToWorld(graphics, screenX - 20, screenY - 50, 40, 50);
                        graphics.drawImage(beam0, screenX, screenY + ((height * 3) / 10), 33);
                        graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                        graphics.drawImage(piece3, screenX, screenY, 33);
                        break;
                    case 8:
                        GameScreen.clipToWorld(graphics, screenX - 20, screenY - 50, 40, 50);
                        graphics.drawImage(beam0, screenX, screenY + ((height * 1) / 5), 33);
                        graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                        graphics.drawImage(piece3, screenX, screenY, 33);
                        break;
                    case 9:
                        graphics.drawImage(beam0, screenX, screenY, 33);
                        graphics.drawImage(piece2, screenX, screenY, 33);
                        break;
                }
                break;
            case 2:
                graphics.drawImage(beam1, screenX, screenY + (this.castRemaining % 3 == 0 ? 1 : 0), 33);
                switch (this.animFrame) {
                    case 1:
                        graphics.drawImage(piece0, screenX, screenY + 3, 33);
                        break;
                    case 2:
                        graphics.drawImage(piece0, screenX, screenY + 3, 33);
                        graphics.drawImage(piece1, screenX, screenY + 6, 33);
                        break;
                    case 3:
                        graphics.drawImage(piece1, screenX, screenY + 6, 33);
                        break;
                }
                break;
            case 3:
                if (this.animFrame < 6) {
                    graphics.drawImage(beam1, screenX, screenY, 33);
                }
                switch (this.animFrame) {
                    case 6:
                        GameScreen.clipToWorld(graphics, screenX - 20, screenY - 50, 40, 50);
                        graphics.drawImage(beam0, screenX, screenY + ((height * 1) / 10), 33);
                        graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                        graphics.drawImage(piece3, screenX, screenY, 33);
                        graphics.drawImage(piece0, screenX, screenY + 3, 33);
                        graphics.drawImage(piece1, screenX, screenY + 6, 33);
                        break;
                    case 7:
                        GameScreen.clipToWorld(graphics, screenX - 20, screenY - 50, 40, 50);
                        graphics.drawImage(beam0, screenX, screenY + ((height * 3) / 10), 33);
                        graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                        graphics.drawImage(piece3, screenX, screenY, 33);
                        graphics.drawImage(piece0, screenX, screenY + 3, 33);
                        break;
                    case 8:
                        GameScreen.clipToWorld(graphics, screenX - 20, screenY - 50, 40, 50);
                        graphics.drawImage(beam0, screenX, screenY + ((height * 5) / 10), 33);
                        graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                        graphics.drawImage(piece3, screenX, screenY, 33);
                        break;
                    case 9:
                        GameScreen.clipToWorld(graphics, screenX - 20, screenY - 50, 40, 50);
                        graphics.drawImage(beam0, screenX, screenY + ((height * 7) / 10), 33);
                        graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                        graphics.drawImage(piece2, screenX, screenY, 33);
                        break;
                }
                break;
        }
    }

    /* renamed from: a */
    /** Draws the skill-name banner and cast-progress bar during a cast. */
    public final void drawSkillBanner(Graphics graphics) {
        int panelX = (BaseCanvas.width - 80) / 2;
        Menu.drawSelectableBox(graphics, panelX, 2, 80, 25, false);
        graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);
        graphics.translate(panelX + 2, 4);
        Menu.drawTextField(graphics, 0, 0, 80, 21, AssetCache.guardianSkillText.get((this.type * 8) + (this.activeSkillSlot * 2)), 0, 1, 6233919, 16777215);
        graphics.setColor(0);
        graphics.fillRect(3, 18, 74, 2);
        graphics.translate(-(panelX + 2), -4);
        graphics.setColor(16733525);
        graphics.fillRect(panelX + 5, 22, (70 * ((this.castTotalFrames - this.castRemaining) + 1)) / this.castTotalFrames, 2);
    }

    /* renamed from: i */
    /** Spawns the forward-strike skill's advancing-line effects. */
    private final void spawnForwardStrikeEffects() {
        GameMap map = GameState.map;
        switch (this.effectFrame) {
            case 0:
                AudioManager.playSfx((byte) 16, false);
                break;
            case 1:
            case 6:
                spawnEffect(map, (short) (((Entity) this).pixelX + (Directions.dirDx[this.castDir] * 16)), (short) (((Entity) this).pixelY + (Directions.dirDy[this.castDir] * 16)), (byte) 1);
                return;
            case 2:
            case 7:
                spawnEffect(map, (short) (((Entity) this).pixelX + (Directions.dirDx[this.castDir] * 32)), (short) (((Entity) this).pixelY + (Directions.dirDy[this.castDir] * 32)), (byte) 1);
                return;
            case 3:
            default:
                return;
            case 4:
            case 8:
                AudioManager.playSfx((byte) 16, false);
                spawnEffect(map, (short) (((Entity) this).pixelX + (Directions.dirDx[this.castDir] * 48)), (short) (((Entity) this).pixelY + (Directions.dirDy[this.castDir] * 48)), (byte) 2);
                spawnEffect(map, (short) (((Entity) this).pixelX + (Directions.dirDx[this.castDir] * 48) + (Directions.dirDx[Directions.rotateCW[this.castDir]] * 16)), (short) (((Entity) this).pixelY + (Directions.dirDy[this.castDir] * 48) + (Directions.dirDy[Directions.rotateCW[this.castDir]] * 16)), (byte) 2);
                spawnEffect(map, (short) (((Entity) this).pixelX + (Directions.dirDx[this.castDir] * 48) + (Directions.dirDx[Directions.rotateCCW[this.castDir]] * 16)), (short) (((Entity) this).pixelY + (Directions.dirDy[this.castDir] * 48) + (Directions.dirDy[Directions.rotateCCW[this.castDir]] * 16)), (byte) 2);
                return;
            case 5:
                break;
        }
        spawnEffect(map, ((Entity) this).pixelX, ((Entity) this).pixelY, (byte) 1);
    }

    /* renamed from: j */
    /** Spawns the quake-strike skill's advancing-line effects. */
    private final void spawnQuakeStrikeEffects() {
        GameMap map = GameState.map;
        switch (this.effectFrame) {
            case 0:
                AudioManager.playSfx((byte) 17, false);
                break;
            case 1:
            case 5:
                spawnEffect(map, (short) (((Entity) this).pixelX + (Directions.dirDx[this.castDir] * 16)), (short) (((Entity) this).pixelY + (Directions.dirDy[this.castDir] * 16)), (byte) 4);
                return;
            case 2:
            case 6:
                spawnEffect(map, (short) (((Entity) this).pixelX + (Directions.dirDx[this.castDir] * 32)), (short) (((Entity) this).pixelY + (Directions.dirDy[this.castDir] * 32)), (byte) 4);
                return;
            case 3:
            case 7:
                AudioManager.playSfx((byte) 17, false);
                spawnEffect(map, (short) (((Entity) this).pixelX + (Directions.dirDx[this.castDir] * 48)), (short) (((Entity) this).pixelY + (Directions.dirDy[this.castDir] * 48)), (byte) 4);
                return;
            case 4:
                break;
            default:
                return;
        }
        spawnEffect(map, ((Entity) this).pixelX, ((Entity) this).pixelY, (byte) 4);
    }

    /* renamed from: a */
    /** Spawns effect {@code effectType} at the two random {@link #aoeOffsets} tiles while active. */
    private final void spawnScatterEffects(int centerX, int centerY, short duration, byte effectType) {
        GameMap map = GameState.map;
        if (this.effectFrame < duration) {
            spawnEffect(map, (short) (centerX + (16 * this.aoeOffsets[0])), (short) (centerY + (16 * this.aoeOffsets[1])), effectType);
            spawnEffect(map, (short) (centerX + (16 * this.aoeOffsets[2])), (short) (centerY + (16 * this.aoeOffsets[3])), effectType);
        }
    }

    /* renamed from: a */
    /** Spawns effect {@code effectType} in an expanding ring around the guardian. */
    private final void spawnRingEffects(byte effectType, int startFrame, int interval) {
        int sinceStart = this.effectFrame - startFrame;
        if (sinceStart >= 0 && sinceStart % interval == 0) {
            GameMap map = GameState.map;
            if (sinceStart / interval == 0) {
                spawnEffect(map, ((Entity) this).pixelX, ((Entity) this).pixelY, effectType);
                return;
            }
            if (sinceStart / interval == 1) {
                spawnEffect(map, (short) (((Entity) this).pixelX + 16), ((Entity) this).pixelY, effectType);
                spawnEffect(map, (short) (((Entity) this).pixelX - 16), ((Entity) this).pixelY, effectType);
                spawnEffect(map, ((Entity) this).pixelX, (short) (((Entity) this).pixelY + 16), effectType);
                spawnEffect(map, ((Entity) this).pixelX, (short) (((Entity) this).pixelY - 16), effectType);
                return;
            }
            if (sinceStart / interval == 2) {
                spawnEffect(map, (short) (((Entity) this).pixelX + 32), ((Entity) this).pixelY, effectType);
                spawnEffect(map, (short) (((Entity) this).pixelX - 32), ((Entity) this).pixelY, effectType);
                spawnEffect(map, ((Entity) this).pixelX, (short) (((Entity) this).pixelY + 32), effectType);
                spawnEffect(map, ((Entity) this).pixelX, (short) (((Entity) this).pixelY - 32), effectType);
                return;
            }
            if (sinceStart / interval == 3) {
                spawnEffect(map, (short) (((Entity) this).pixelX + 48), ((Entity) this).pixelY, effectType);
                spawnEffect(map, (short) (((Entity) this).pixelX - 48), ((Entity) this).pixelY, effectType);
                spawnEffect(map, ((Entity) this).pixelX, (short) (((Entity) this).pixelY + 48), effectType);
                spawnEffect(map, ((Entity) this).pixelX, (short) (((Entity) this).pixelY - 48), effectType);
            }
        }
    }

    /* renamed from: a */
    /** Banish visuals: a cast effect on matching enemies ahead, else a puff. */
    private final void spawnBanishEffects(short effectDuration) {
        if (this.effectFrame == 0) {
            GameMap map = GameState.map;
            for (int dist = 1; dist <= 3; dist++) {
                Entity target = neighbor(this.castDir, (byte) dist);
                if (target == null || !(target instanceof Enemy) || (target instanceof Boss) || ((Enemy) target).stats.element != element()) {
                    spawnEffect(map, (short) (((Entity) this).pixelX + (Directions.dirDx[this.castDir] * 16 * dist)), (short) (((Entity) this).pixelY + (Directions.dirDy[this.castDir] * 16 * dist)), (byte) 10);
                } else {
                    ((Enemy) target).addFloater(new GuardianCastFx((short) 0, effectDuration, this.type, this.activeSkillSlot));
                }
            }
        }
    }

    /* renamed from: a */
    /** Adds an {@link Effect} of type {@code effectType} to {@code map} at a pixel position. */
    private static final void spawnEffect(GameMap map, short pixelX, short pixelY, byte effectType) {
        map.addEntity(new Effect(pixelX, pixelY, effectType));
    }

    /* renamed from: b */
    /** Picks two random AoE tile offsets within a diamond of the given {@code radius}. */
    private final void randomizeAoeOffsets(int radius) {
        this.aoeOffsets[0] = (byte) (Entity.rng.nextInt() % (radius + 1));
        this.aoeOffsets[1] = (byte) (Entity.rng.nextInt() % ((radius - Math.abs((int) this.aoeOffsets[0])) + 1));
        this.aoeOffsets[2] = (byte) (Entity.rng.nextInt() % (radius + 1));
        this.aoeOffsets[3] = (byte) (Entity.rng.nextInt() % ((radius - Math.abs((int) this.aoeOffsets[2])) + 1));
    }
}
