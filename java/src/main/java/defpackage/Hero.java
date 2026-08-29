package defpackage;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.util.Vector;
import javax.microedition.lcdui.Graphics;

/* renamed from: ao */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ao.class */
/**
 * The player character: a {@link Battler} that owns the four base stats
 * (strength/vitality/agility/spirit), derived combat stats, HP/MP/exp and
 * leveling, the inventory and quick-item bags, equipment, guardian companions,
 * the combo/attack animation state, and the RMS save format.
 *
 * <p>Combat runs off a per-class combo table ({@link #comboFrames}): each attack
 * builds a chain of steps ({@link #comboSteps}) whose damage is rolled by
 * {@link #rollDamage()} (attack &times; per-step multiplier &plusmn; RNG, boosted by
 * the {@link #attackUp} buff), with a critical roll ({@link #rollCrit()}) and a
 * weapon/armour status proc ({@link #rollProc()}); the resolved hit is applied
 * to enemies by one of the three class-specific routines
 * ({@link #attackClass6}/{@link #attackClass7}/{@link #attackClass8}). Incoming
 * damage is resolved by {@link #takeHit}.
 */
public final class Hero extends Battler implements Directions {
    /* renamed from: b */
    /** Warrior (class 6) combo frame table: [attackType-1][comboStep] durations. */
    private static final byte[][] COMBO_FRAMES_CLASS6 = {new byte[]{4, 4, 4, 4, 0}, new byte[]{4, 0, 4, 4, 8}};

    /* renamed from: c */
    /** Rogue (class 7) combo frame table. */
    private static final byte[][] COMBO_FRAMES_CLASS7 = {new byte[]{3, 3, 3, 6, 0}, new byte[]{4, 0, 7, 9, 14}};

    /* renamed from: d */
    /** Mage (class 8) combo frame table. */
    private static final byte[][] COMBO_FRAMES_CLASS8 = {new byte[]{3, 3, 3, 3, 0}, new byte[]{3, 0, 3, 3, 6}};

    /* renamed from: h */
    /** Attack type of each queued combo step (1 = normal, 2 = special). */
    private byte[] comboSteps;

    /* renamed from: q */
    /** Index of the combo step currently animating (-1 = none). */
    private byte comboIndex;

    /* renamed from: r */
    /** Hit-recoil display countdown (offsets the sprite for one frame). */
    private byte recoilTimer;

    /* renamed from: s */
    /** Direction the recoil offset is applied toward. */
    private byte recoilDir;

    /* renamed from: t */
    /** Combo lockout: blocks queueing the next step until it expires. */
    private byte comboLockout;

    /* renamed from: u */
    /** Death-animation countdown (state 6) before the game-over screen. */
    private byte deathTimer;

    /* renamed from: v */
    /** HP-regen countdown (ticks down while in combat). */
    private byte hpRegenTimer;

    /* renamed from: w */
    /** MP-regen countdown. */
    private byte mpRegenTimer;

    /* renamed from: x */
    /** A turn queued mid-step, applied once aligned to the grid. */
    private byte queuedTurn;

    /* renamed from: y */
    /** Lunge distance (tiles) for a dashing special attack; drives the afterimage. */
    private byte lungeSteps;

    /* renamed from: i */
    /** Guard so a tile trigger fires at most once per step. */
    private boolean triggerChecked;

    /* renamed from: e */
    /** The selected class's combo frame table (one of COMBO_FRAMES_CLASS*). */
    private byte[][] comboFrames;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /** Attack-up buff (guardian skill; multiplies rolled damage by 3/2). */
    public boolean attackUp;

    /* JADX INFO: renamed from: e, reason: collision with other field name */
    /** Defense-up buff (guardian skill; doubles defense vs incoming hits). */
    public boolean defenseUp;

    /* renamed from: f */
    /** Invincibility buff (guardian skill; ignores all incoming damage). */
    public boolean invincible;

    /* renamed from: g */
    /** Reflect buff (guardian skill; strikes attackers back for spirit damage). */
    public boolean reflectDamage;

    /* JADX INFO: renamed from: h, reason: collision with other field name */
    /** Regen-boost buff (guardian skill; doubles HP/MP regen). */
    public boolean regenBoost;

    /* renamed from: z */
    /** Maximum combo depth unlocked (steps beyond this cannot be queued). */
    private byte maxCombo;
    public short statPoints;

    /* JADX INFO: renamed from: f, reason: collision with other field name */
    public byte classId;

    /* JADX INFO: renamed from: g, reason: collision with other field name */
    public byte level;

    /* renamed from: b */
    /** Base strength (drives attack and defense). */
    public short strength;

    /* renamed from: e */
    /** Base vitality (drives max HP). */
    public short vitality;

    /* renamed from: f */
    /** Base agility (drives hit chance and physical damage). */
    public short agility;

    /* renamed from: g */
    /** Base spirit (drives max MP). */
    public short spirit;
    /** Strength contributed by equipment. */
    public byte strengthBonus;
    /** Vitality contributed by equipment. */
    public byte vitalityBonus;
    /** Agility contributed by equipment. */
    public byte agilityBonus;
    /** Spirit contributed by equipment. */
    public byte spiritBonus;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    public int hp;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    public int mp;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    public int exp;

    /* JADX INFO: renamed from: h, reason: collision with other field name */
    public short attack;

    /* JADX INFO: renamed from: i, reason: collision with other field name */
    public short defense;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    public int maxHp;

    /* JADX INFO: renamed from: e, reason: collision with other field name */
    public int maxMp;

    /* JADX INFO: renamed from: f, reason: collision with other field name */
    public int expToNext;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    public ItemBag bag;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    private Equipment[] equipment;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    public Guardian[] guardians;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    private Guardian activeGuardian;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    public ItemBag quickItems;

    /* JADX INFO: renamed from: i, reason: collision with other field name */
    /** Damage rolled for the current strike ({@link #rollDamage()}). */
    private int rolledDamage;

    /* renamed from: j */
    /** Whether the current strike rolled a critical ({@link #rollCrit()}). */
    private boolean rolledCrit;

    /* renamed from: A */
    /** Weapon/armour status proc rolled for the current strike ({@link #rollProc()}). */
    private byte rolledProc;

    /* JADX INFO: renamed from: g, reason: collision with other field name */
    public int sessionStartSec;

    /* JADX INFO: renamed from: h, reason: collision with other field name */
    public int playSeconds;

    public Hero(short pixelX, short pixelY, byte halfWidth, byte halfHeight, byte classId) {
        super(pixelX, pixelY, halfWidth, halfHeight);
        this.recoilTimer = (byte) 0;
        this.queuedTurn = (byte) 0;
        this.bag = new ItemBag((byte) 30);
        this.equipment = new Equipment[5];
        this.guardians = new Guardian[5];
        this.quickItems = new ItemBag((byte) 15);
        init();
        resetBuffs();
        switch (classId) {
            case 6:
                this.comboFrames = COMBO_FRAMES_CLASS6;
                break;
            case 7:
                this.comboFrames = COMBO_FRAMES_CLASS7;
                break;
            case 8:
                this.comboFrames = COMBO_FRAMES_CLASS8;
                break;
        }
        this.sessionStartSec = (int) (System.currentTimeMillis() / 1000);
    }

    public final void initClass(byte classId) {
        defpackage.Debug.assertTrue(this.guardians[0] != null);
        defpackage.Debug.assertTrue(this.guardians[1] != null);
        setActiveGuardian(this.guardians[0]);
        switch (classId) {
            case 6:
                this.strength = (short) 8;
                this.vitality = (short) 5;
                this.agility = (short) 3;
                this.spirit = (short) 4;
                this.equipment[0] = (Equipment) Item.create((byte) 0, (byte) 0, true, false);
                this.equipment[0].identified = true;
                this.equipment[0].quantity = (byte) 1;
                break;
            case 7:
                this.strength = (short) 3;
                this.vitality = (short) 4;
                this.agility = (short) 8;
                this.spirit = (short) 5;
                this.equipment[0] = (Equipment) Item.create((byte) 2, (byte) 0, true, false);
                this.equipment[0].identified = true;
                this.equipment[0].quantity = (byte) 1;
                break;
            case 8:
                this.strength = (short) 5;
                this.vitality = (short) 8;
                this.agility = (short) 4;
                this.spirit = (short) 3;
                this.equipment[0] = (Equipment) Item.create((byte) 1, (byte) 0, true, false);
                this.equipment[0].identified = true;
                this.equipment[0].quantity = (byte) 1;
                this.equipment[1] = (Equipment) Item.create((byte) 3, (byte) 0, true, false);
                this.equipment[1].identified = true;
                this.equipment[1].quantity = (byte) 1;
                break;
        }
        this.equipment[2] = (Equipment) Item.create((byte) 5, (byte) 0, true, false);
        this.equipment[2].identified = true;
        this.equipment[2].quantity = (byte) 1;
        this.equipment[3] = (Equipment) Item.create((byte) 6, (byte) 0, true, false);
        this.equipment[3].identified = true;
        this.equipment[3].quantity = (byte) 1;
        this.equipment[4] = (Equipment) Item.create((byte) 4, (byte) 0, true, false);
        this.equipment[4].identified = true;
        this.equipment[4].quantity = (byte) 1;
        this.level = (byte) 1;
        this.maxCombo = (byte) 1;
        this.bag.gold = 300;
        this.statPoints = (short) 0;
        recomputeStats();
        this.hp = this.maxHp;
        this.mp = this.maxMp;
        this.exp = 0;
    }

    /* renamed from: a */
    @Override // defpackage.o
    public final void init() {
        super.init();
        this.comboSteps = new byte[5];
        this.comboIndex = (byte) -1;
        this.hpRegenTimer = 67 + this.level < 100 ? (byte) (67 + this.level) : (byte) 100;
        this.mpRegenTimer = (byte) 21;
        this.lungeSteps = (byte) 0;
        this.invincible = false;
        if (this.activeGuardian != null) {
            this.activeGuardian.dismiss();
        }
        this.triggerChecked = false;
    }

    /* renamed from: h */
    /** Clears all guardian-buff flags and the status-icon list. */
    public final void resetBuffs() {
        ((Battler) this).statuses = new Vector(3);
        this.attackUp = false;
        this.defenseUp = false;
        this.invincible = false;
        this.reflectDamage = false;
        this.regenBoost = false;
    }

    @Override // defpackage.ck
    public final void setPixelPos(short pixelX, short pixelY) {
        super.setPixelPos(pixelX, pixelY);
        syncTile();
    }

    /* renamed from: i */
    /** If nothing is directly ahead, turns toward an adjacent enemy on either side. */
    public final void turnTowardEnemy() {
        if (enemyAhead() == null) {
            if (enemyInDir(defpackage.Directions.rotateCCW[this.facing]) != null) {
                setFacing(defpackage.Directions.rotateCCW[this.facing]);
            } else if (enemyInDir(defpackage.Directions.rotateCW[this.facing]) != null) {
                setFacing(defpackage.Directions.rotateCW[this.facing]);
            } else if (enemyInDir(defpackage.Directions.reverse[this.facing]) != null) {
                setFacing(defpackage.Directions.reverse[this.facing]);
            }
        }
    }

    /* renamed from: a */
    /** Slides the hero {@code distancePixels} in direction {@code dir}, re-registering it. */
    public final void slide(byte dir, byte distancePixels) {
        clearOccupancy();
        ((Entity) this).pixelX = (short) (((Entity) this).pixelX + (defpackage.Directions.dirDx[dir] * distancePixels));
        ((Entity) this).pixelY = (short) (((Entity) this).pixelY + (defpackage.Directions.dirDy[dir] * distancePixels));
        syncTile();
        setOccupancy();
    }

    /* renamed from: d */
    @Override // defpackage.o
    public final void update() {
        this.animFrame = (byte) (this.animFrame + 1);
        if (this.comboLockout > 0) {
            this.comboLockout = (byte) (this.comboLockout - 1);
        }
        if (defpackage.GameState.screen == 2) {
            if (this.state == 1) {
                this.hpRegenTimer = (byte) (this.hpRegenTimer - 2);
            } else if (this.state == 2) {
                this.hpRegenTimer = (byte) (this.hpRegenTimer - 1);
            }
            if (this.hpRegenTimer <= 0) {
                addHp((this.vitality + this.vitalityBonus) * (this.regenBoost ? 2 : 1));
                this.hpRegenTimer = 67 + this.level < 100 ? (byte) (67 + this.level) : (byte) 100;
            }
            if (this.state == 1) {
                this.mpRegenTimer = (byte) (this.mpRegenTimer - 3);
            } else if (this.state == 2) {
                this.mpRegenTimer = (byte) (this.mpRegenTimer - 1);
            }
            if (this.mpRegenTimer <= 0) {
                addMp((this.spirit + this.spiritBonus) * (this.regenBoost ? 2 : 1));
                this.mpRegenTimer = (byte) 21;
            }
        }
        if (this.state != 6 && this.state != 5) {
            for (int size = ((Battler) this).statuses.size() - 1; size >= 0; size--) {
                StatusIcon icon = (StatusIcon) ((Battler) this).statuses.elementAt(size);
                icon.tick();
                if (icon.kind == 7 && icon.frame % 10 == 0 && this.hp > 1) {
                    int poison = this.maxHp / 25;
                    int poisonDamage = poison;
                    if (poison > this.hp - 1) {
                        poisonDamage = this.hp - 1;
                    }
                    addHp(-poisonDamage);
                    addFloater(new Floater((byte) 7, (short) 4, (short) (-(this.maxHp / 12))));
                }
                if (((Overlay) icon).finished) {
                    ((Battler) this).statuses.removeElementAt(size);
                    switch (icon.kind) {
                        case 5:
                            this.attackUp = false;
                            break;
                        case 6:
                            this.defenseUp = false;
                            break;
                    }
                }
            }
        }
        switch (this.state) {
            case 0:
                return;
            case 1:
                this.queuedTurn = (byte) 0;
                if (this.animFrame == 1) {
                    this.animFrame = (byte) 0;
                }
                break;
            case 2:
                if (this.queuedTurn != 0 && !((Entity) this).offGridX && !((Entity) this).offGridY) {
                    setFacing(this.queuedTurn);
                    this.queuedTurn = (byte) 0;
                }
                if (this.animFrame == 4) {
                    this.animFrame = (byte) 0;
                }
                break;
            case 3:
                if (this.comboIndex == -1) {
                    this.comboIndex = (byte) 0;
                }
                advanceCombo();
                break;
            case 6:
                this.animFrame = (byte) 0;
                if (this.deathTimer <= 0) {
                    onDeath();
                    return;
                }
                this.deathTimer = (byte) (this.deathTimer - 1);
                break;
        }
        byte stateBefore = this.state;
        GameMap map = defpackage.GameState.map;
        if ((this.state == 2 || this.state == 4) && tryStepForward()) {
            Enemy ahead = enemyAhead();
            if (ahead != null) {
                GameLoop.gameScreen.setTarget(ahead, false);
            }
            byte dir = this.facing;
            byte leftDir = defpackage.Directions.rotateCW[dir];
            byte rightDir = defpackage.Directions.rotateCCW[dir];
            byte leftBackDir = defpackage.Directions.diagCW[dir];
            byte rightBackDir = defpackage.Directions.diagCCW[dir];
            if (map.collisionGrid[((Entity) this).tileY + defpackage.Directions.dirDy[dir]][((Entity) this).tileX + defpackage.Directions.dirDx[dir]] == -128 && map.isWalkable(((Entity) this).tileX + defpackage.Directions.dirDx[rightDir], ((Entity) this).tileY + defpackage.Directions.dirDy[rightDir]) && map.isWalkable(((Entity) this).tileX + defpackage.Directions.dirDx[rightBackDir], ((Entity) this).tileY + defpackage.Directions.dirDy[rightBackDir])) {
                this.queuedTurn = this.facing;
                setState((byte) 2);
                setFacing(rightDir);
            } else if (map.collisionGrid[((Entity) this).tileY + defpackage.Directions.dirDy[dir]][((Entity) this).tileX + defpackage.Directions.dirDx[dir]] == -128 && map.isWalkable(((Entity) this).tileX + defpackage.Directions.dirDx[leftDir], ((Entity) this).tileY + defpackage.Directions.dirDy[leftDir]) && map.isWalkable(((Entity) this).tileX + defpackage.Directions.dirDx[leftBackDir], ((Entity) this).tileY + defpackage.Directions.dirDy[leftBackDir])) {
                this.queuedTurn = this.facing;
                setState((byte) 2);
                setFacing(leftDir);
            }
            this.animFrame = (byte) 0;
        }
        if (this.state == 2 || this.state == 4) {
            super.move(8);
            this.triggerChecked = false;
        }
        if (defpackage.GameState.screen != 4) {
            boolean triggered = false;
            if (this.state != 3 && !this.triggerChecked) {
                triggered = EventScript.checkTileTrigger(this);
                this.triggerChecked = true;
            }
            if (stateBefore == 2 && this.state == 1 && !triggered) {
                triggered = EventScript.checkFacingTrigger();
            }
            if (triggered) {
                setState((byte) 1);
                this.queuedTurn = (byte) 0;
                this.animFrame = (byte) 0;
            }
        }
    }

    /* renamed from: o */
    /** Advances the attack combo: steps to the next chain link, spending MP and firing hits. */
    private final void advanceCombo() {
        if (this.animFrame == this.comboFrames[this.comboSteps[this.comboIndex] - 1][this.comboIndex]) {
            if (this.comboIndex + 1 == this.maxCombo || this.comboSteps[this.comboIndex + 1] == 0) {
                endCombo(this.comboIndex);
            } else {
                this.comboIndex = (byte) (this.comboIndex + 1);
                this.animFrame = (byte) 0;
            }
        }
        this.lungeSteps = (byte) 0;
        if (this.animFrame == 0) {
            int mpCost = (((Equipment) ((Weapon) getEquip(0))).value / 4) + 4;
            if (this.comboSteps[this.comboIndex] == 2) {
                mpCost = (mpCost * 7) / 5;
            }
            if (this.mp < mpCost && (this.comboIndex != 0 || this.comboSteps[this.comboIndex] != 1)) {
                endCombo(this.comboIndex - 1);
                return;
            }
            addMp(-mpCost);
        }
        byte attackType = this.comboSteps[this.comboIndex];
        switch (defpackage.GameState.classId) {
            case 6:
                if (attackType == 2 && this.comboIndex == 4) {
                    if (this.animFrame == 1 || this.animFrame == 6) {
                        performAttack();
                    }
                } else if (this.animFrame == 2) {
                    performAttack();
                }
                break;
            case 7:
                if (attackType == 2 && this.comboIndex == 4) {
                    if (this.animFrame == 0 || this.animFrame == 2 || this.animFrame == 4 || this.animFrame == 6 || this.animFrame == 8 || this.animFrame == 10) {
                        performAttack();
                    }
                } else if (attackType == 2 && this.comboIndex == 3) {
                    if (this.animFrame == 4) {
                        performAttack();
                    }
                } else if (this.animFrame == 1) {
                    performAttack();
                }
                break;
            case 8:
                if (attackType == 2 && this.comboIndex == 4) {
                    if (this.animFrame == 4) {
                        performAttack();
                    }
                } else if (this.animFrame == 1) {
                    performAttack();
                }
                break;
        }
    }

    /* renamed from: p */
    /** Rolls damage/crit/proc for this strike, shows proc floaters, and applies the class hit. */
    private final void performAttack() {
        this.rolledDamage = rollDamage();
        this.rolledProc = rollProc();
        this.rolledCrit = rollCrit();
        if (enemyAhead() != null) {
            switch (this.rolledProc) {
                case 2:
                    addFloater(new Floater((byte) 10, (short) 8, (short) 8));
                    break;
                case 3:
                    addFloater(new Floater((byte) 10, (short) 8, (short) 10));
                    break;
                case 4:
                    addFloater(new Floater((byte) 10, (short) 8, (short) 11));
                    break;
                case 8:
                    int selfDamage = this.maxHp / 25;
                    addHp(-selfDamage);
                    ((Battler) this).floaters.addElement(new Floater((byte) 7, (short) 4, (short) (-selfDamage)));
                    addFloater(new Floater((byte) 10, (short) 8, (short) 0));
                    break;
            }
        }
        boolean anyHit = false;
        switch (defpackage.GameState.classId) {
            case 6:
                anyHit = attackClass6();
                break;
            case 7:
                anyHit = attackClass7();
                break;
            case 8:
                anyHit = attackClass8();
                break;
        }
        if (anyHit) {
            return;
        }
        AudioManager.playSfx((byte) 14, false);
    }

    /* renamed from: b */
    /** Warrior (class 6) hit resolution: front-arc, multi-target and lunge patterns. */
    private final boolean attackClass6() {
        Enemy lungeTarget;
        byte attackType = this.comboSteps[this.comboIndex];
        byte hitFloaterKind = 1;
        if ((attackType == 1 && this.comboIndex == 3) || (attackType == 2 && this.comboIndex == 4)) {
            hitFloaterKind = 5;
        }
        GameMap map = defpackage.GameState.map;
        boolean anyHit = false;
        if ((attackType == 1 && (this.comboIndex == 0 || this.comboIndex == 3)) || (attackType == 2 && this.comboIndex == 4 && this.animFrame == 6)) {
            Enemy target = enemyAhead();
            if (target != null) {
                target.takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, hitFloaterKind, this.rolledProc, this);
                anyHit = true;
            }
        } else if ((attackType != 1 || (this.comboIndex != 1 && this.comboIndex != 2)) && (attackType != 2 || this.comboIndex != 0)) {
            if (attackType == 2 && this.comboIndex == 4) {
                byte dir = 1;
                while (true) {
                    byte d = dir;
                    if (d > 8) {
                        break;
                    }
                    Enemy target = enemyInDir(d);
                    if (target != null) {
                        target.takeHeroHit(this.rolledDamage, true, d, this.rolledCrit, hitFloaterKind, this.rolledProc, this);
                        anyHit = true;
                    }
                    dir = (byte) (d + 1);
                }
            } else if (attackType == 2 && (this.comboIndex == 2 || this.comboIndex == 3)) {
                Enemy target = enemyAhead();
                if (target != null) {
                    target.takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, hitFloaterKind, this.rolledProc, this);
                    anyHit = true;
                }
                byte dx = defpackage.Directions.dirDx[this.facing];
                byte dy = defpackage.Directions.dirDy[this.facing];
                if (map.collisionGrid[((Entity) this).tileY + dy][((Entity) this).tileX + dx] == 0 && map.isWalkable(((Entity) this).tileX + (dx * 2), ((Entity) this).tileY + (dy * 2))) {
                    super.move(32);
                    this.triggerChecked = false;
                    this.lungeSteps = (byte) 2;
                } else if (map.isWalkable(((Entity) this).tileX + dx, ((Entity) this).tileY + dy)) {
                    super.move(16);
                    this.triggerChecked = false;
                    this.lungeSteps = (byte) 1;
                }
                if (this.comboIndex == 3 && this.lungeSteps != 0 && (lungeTarget = enemyAhead()) != null) {
                    lungeTarget.takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, hitFloaterKind, this.rolledProc, this);
                    anyHit = true;
                }
            }
        } else {
            Enemy target = enemyInDir(this.facing);
            if (target != null) {
                target.takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, hitFloaterKind, this.rolledProc, this);
                anyHit = true;
            }
            Enemy sideTarget = enemyInDir(defpackage.Directions.diagCCW[this.facing]);
            if (sideTarget != null && sideTarget != target) {
                sideTarget.takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, hitFloaterKind, this.rolledProc, this);
                anyHit = true;
            }
            Enemy sideTarget2 = enemyInDir(defpackage.Directions.diagCW[this.facing]);
            if (sideTarget2 != null && sideTarget2 != sideTarget) {
                sideTarget2.takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, hitFloaterKind, this.rolledProc, this);
                anyHit = true;
            }
        }
        return anyHit;
    }

    /* renamed from: c */
    /** Rogue (class 7) hit resolution: a spread of enemies ahead. */
    private final boolean attackClass7() {
        boolean anyHit = false;
        if (this.comboSteps[this.comboIndex] == 1 && this.comboIndex == 3) {
            Enemy target = enemyInDir(this.facing);
            if (target != null) {
                target.takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, (byte) 1, this.rolledProc, this);
                anyHit = true;
            }
            Enemy sideTarget = enemyInDir(defpackage.Directions.diagCCW[this.facing]);
            if (sideTarget != null && sideTarget != target) {
                sideTarget.takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, (byte) 1, this.rolledProc, this);
                anyHit = true;
            }
            Enemy sideTarget2 = enemyInDir(defpackage.Directions.diagCW[this.facing]);
            if (sideTarget2 != null && sideTarget2 != sideTarget) {
                sideTarget2.takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, (byte) 1, this.rolledProc, this);
                anyHit = true;
            }
        } else {
            Entity target = neighbor(this.facing, (byte) 1);
            if (target != null && (target instanceof Enemy)) {
                ((Enemy) target).takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, (byte) 1, this.rolledProc, this);
                anyHit = true;
            }
            Entity target2 = neighbor(this.facing, (byte) 2);
            if (target2 != null && (target2 instanceof Enemy)) {
                ((Enemy) target2).takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, (byte) 1, this.rolledProc, this);
                anyHit = true;
            }
        }
        return anyHit;
    }

    /* JADX INFO: renamed from: d, reason: collision with other method in class */
    /** Mage (class 8) hit resolution: fires projectiles for special steps, else a melee hit. */
    private final boolean attackClass8() {
        byte attackType = this.comboSteps[this.comboIndex];
        boolean anyHit = false;
        if (attackType == 2 && this.comboIndex == 2) {
            defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX + defpackage.Directions.dirDx[this.facing]), (byte) (((Entity) this).tileY + defpackage.Directions.dirDy[this.facing]), (byte[]) AssetCache.mageAuraScripts[0], this, true, this.facing, (byte) 3, (byte) 2, this.rolledDamage, this.rolledProc, this.rolledCrit));
        } else if (attackType == 2 && this.comboIndex == 3) {
            defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX + defpackage.Directions.dirDx[this.facing]), (byte) (((Entity) this).tileY + defpackage.Directions.dirDy[this.facing]), (byte[]) AssetCache.mageAuraScripts[1], this, true, this.facing, (byte) 3, (byte) 2, this.rolledDamage, this.rolledProc, this.rolledCrit));
        } else if (attackType == 2 && this.comboIndex == 4) {
            defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX + defpackage.Directions.dirDx[this.facing]), (byte) (((Entity) this).tileY + defpackage.Directions.dirDy[this.facing]), (byte[]) AssetCache.mageAuraScripts[2], this, true, this.facing, (byte) 3, (byte) 2, this.rolledDamage, this.rolledProc, this.rolledCrit));
        } else {
            Enemy target = enemyAhead();
            if (target != null) {
                target.takeHeroHit(this.rolledDamage, false, this.facing, this.rolledCrit, (byte) 1, this.rolledProc, this);
                anyHit = true;
            }
        }
        return anyHit;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /**
     * Rolls attack damage: base {@link #attack} (&times;3/2 while {@link #attackUp}),
     * scaled by the per-combo-step multiplier (100/120/130/140/170%, or 170% for a
     * special), plus up to 10% RNG.
     */
    private final int rollDamage() {
        int dmg = this.attack;
        if (this.attackUp) {
            dmg = (this.attack * 3) / 2;
        }
        if (this.comboSteps[this.comboIndex] != 2) {
            switch (this.comboIndex) {
                case 0:
                    dmg = (dmg * 10) / 10;
                    break;
                case 1:
                    dmg = (dmg * 12) / 10;
                    break;
                case 2:
                    dmg = (dmg * 13) / 10;
                    break;
                case 3:
                    dmg = (dmg * 14) / 10;
                    break;
                case 4:
                    dmg = (dmg * 17) / 10;
                    break;
            }
        } else {
            dmg = (dmg * 17) / 10;
        }
        return dmg + (dmg >= 10 ? Entity.rng.nextInt() % (dmg / 10) : 0);
    }

    /* renamed from: e */
    /** Rolls a critical: chance = agility/3 + spirit/10 + weapon accuracy, out of 100. */
    private final boolean rollCrit() {
        return Math.abs(Entity.rng.nextInt() % 100) < (((this.agility + this.agilityBonus) / 3) + ((this.spirit + this.spiritBonus) / 10)) + ((Weapon) this.equipment[0]).accuracy;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Rolls a weapon-then-armour status proc (their {@code c} index vs {@link Armor#i}), or -1. */
    private final byte rollProc() {
        Weapon weapon = (Weapon) this.equipment[0];
        Armor armor = (Armor) this.equipment[1];
        byte proc = -1;
        if (weapon.attribute != -1 && defpackage.ByteUtil.randRange(0, 99) < defpackage.Armor.PROC_CHANCE[weapon.attribute]) {
            proc = weapon.attribute;
        }
        if (proc == -1 && armor != null && armor.attribute != -1 && defpackage.ByteUtil.randRange(0, 99) < defpackage.Armor.PROC_CHANCE[armor.attribute]) {
            proc = armor.attribute;
        }
        return proc;
    }

    /* renamed from: h */
    /** Ends the current combo at step {@code step}, setting the recovery lockout. */
    private final void endCombo(int step) {
        if (step == -1 || (step == 0 && this.comboSteps[step] == 1)) {
            this.comboLockout = (byte) 1;
        } else if (!(step == 0 && this.comboSteps[step] == 2) && this.comboSteps[step] == 1) {
            this.comboLockout = (byte) 1;
        } else {
            this.comboLockout = (byte) 3;
        }
        resetCombo();
        setState((byte) 1);
        this.animFrame = (byte) 0;
    }

    /* renamed from: q */
    /** Player death: request the game-over screen and stop the music. */
    private final void onDeath() {
        defpackage.GameState.requestState((byte) 16);
        AudioManager.stopBgm();
    }

    /* renamed from: a */
    /**
     * Queues the next combo step (normal or special), if combat is enabled, the
     * combo has room, and it is not locked out. Returns whether a step was queued
     * (or is already pending).
     */
    public final boolean queueComboStep(boolean special) {
        if (!defpackage.GameState.map.combatEnabled || this.comboIndex + 1 >= this.maxCombo || this.comboLockout > 0) {
            return false;
        }
        if (this.comboSteps[this.comboIndex + 1] != 0) {
            return true;
        }
        if (this.comboIndex >= 0 && this.comboSteps[this.comboIndex] == 2) {
            return false;
        }
        if (this.comboIndex == 0 && special) {
            return false;
        }
        if (this.comboIndex == 3 && !special) {
            return false;
        }
        this.comboSteps[this.comboIndex + 1] = special ? (byte) 2 : (byte) 1;
        return true;
    }

    /* renamed from: j */
    /** Resets the combo chain to empty. */
    public final void resetCombo() {
        this.comboIndex = (byte) -1;
        for (int i = 0; i < this.comboSteps.length; i++) {
            this.comboSteps[i] = 0;
        }
    }

    @Override // defpackage.ck
    public final void paint(Graphics graphics, int originX, int originY) {
        int screenX = originX + ((Entity) this).pixelX + ((Entity) this).halfW;
        int screenY = originY + ((Entity) this).pixelY + ((Entity) this).halfH;
        if (this.recoilTimer == 1) {
            screenX += defpackage.Directions.dirDx[this.recoilDir] * 2;
            screenY += defpackage.Directions.dirDy[this.recoilDir] * 2;
            this.recoilTimer = (byte) (this.recoilTimer - 1);
        }
        graphics.drawImage(AssetCache.entityShadow, screenX, screenY - 3, 17);
        switch (this.state) {
            case 1:
            case 4:
                drawCharacterSprite((byte) 0, this.facing, graphics, screenX, screenY);
                break;
            case 2:
                drawCharacterSprite((byte) 1, this.facing, graphics, screenX, screenY);
                break;
            case 3:
                drawAttackSprite(graphics, screenX, screenY);
                if (this.lungeSteps != 0) {
                    drawAttackSprite(graphics, screenX + (defpackage.Directions.dirDx[defpackage.Directions.reverse[this.facing]] * 16 * this.lungeSteps), screenY + (defpackage.Directions.dirDy[defpackage.Directions.reverse[this.facing]] * 16 * this.lungeSteps));
                }
                break;
            case 6:
                drawCharacterSprite((byte) 2, (byte) 1, graphics, screenX, screenY);
                break;
        }
        drawStatusIcons(graphics, screenX, screenY - 8);
        drawFloaters(graphics, screenX, screenY);
    }

    /* renamed from: d */
    /** Draws the hero's summon/death pose (used by the guardian-summon overlay). */
    public final void drawSummonPose(Graphics graphics, int x, int y) {
        drawCharacterSprite((byte) 2, (byte) 1, graphics, x, y);
    }

    /* renamed from: a */
    /** Draws the 9-layer character sprite for pose {@code pose} facing {@code dir}. */
    private void drawCharacterSprite(byte pose, byte dir, Graphics graphics, int x, int y) {
        int baseIndex = (pose * 36) + ((dir - 1) * 9);
        for (int layer = 0; layer < 9; layer++) {
            if ((layer != 6 && layer != 7) || defpackage.GameState.map.combatEnabled) {
                if (layer == 7) {
                    GameScreen.drawFrameGroup(graphics, (byte[]) AssetCache.heroFrames[baseIndex + layer], this.animFrame, x, y);
                } else {
                    GameScreen.drawFrame(graphics, (byte[]) AssetCache.heroFrames[baseIndex + layer], this.animFrame, x, y);
                }
            }
        }
    }

    /* renamed from: e */
    /** Draws the attack-pose sprite for the current combo step. */
    private void drawAttackSprite(Graphics graphics, int x, int y) {
        byte pose = -1;
        switch (this.comboIndex) {
            case 0:
                pose = this.comboSteps[this.comboIndex] != 1 ? (byte) 7 : (byte) 3;
                break;
            case 1:
                pose = 4;
                break;
            case 2:
                pose = this.comboSteps[this.comboIndex] != 1 ? (byte) 8 : (byte) 5;
                break;
            case 3:
                pose = this.comboSteps[this.comboIndex] != 1 ? (byte) 9 : (byte) 6;
                break;
            case 4:
                pose = 10;
                break;
        }
        drawCharacterSprite(pose, this.facing, graphics, x, y);
    }

    /* renamed from: a */
    /** Takes a hit from {@code attacker} using its template attack, from {@code dir}. */
    public final void takeHit(Enemy attacker, byte dir) {
        takeHit(attacker, attacker.stats.attack, dir);
    }

    /* renamed from: a */
    /**
     * Resolves incoming damage of magnitude {@code rawDamage} from {@code attacker}.
     * Dodge chance = clamp((agility+bonus) - evasion + 10 + accessory, 8, 60)%; on
     * a hit, damage = rawDamage &plusmn;10% minus defense (doubled while
     * {@link #defenseUp}). Ignored entirely while {@link #invincible}; if
     * {@link #reflectDamage} is set, the attacker is struck back for spirit damage.
     * Melee-type-1 attackers have a 15% chance to inflict poison (status 7).
     */
    public final void takeHit(Enemy attacker, short rawDamage, byte dir) {
        if (this.state == 6 || this.state == 5 || this.invincible) {
            return;
        }
        if (this.reflectDamage) {
            attacker.damage((this.activeGuardian.level * 2) + 40 + this.spirit);
        }
        GameLoop.gameScreen.setTarget(attacker, true);
        int dodgeChance = ((this.agility + this.agilityBonus) - attacker.stats.evasion) + 10;
        if (this.equipment[2] != null) {
            dodgeChance += this.equipment[2].refineLevel;
        }
        if (dodgeChance > 60) {
            dodgeChance = 60;
        }
        if (dodgeChance < 8) {
            dodgeChance = 8;
        }
        if (defpackage.ByteUtil.randRange(0, 99) < dodgeChance) {
            ((Battler) this).floaters.addElement(new Floater((byte) 2));
            return;
        }
        int finalDamage = (rawDamage + defpackage.ByteUtil.randRange(-(rawDamage / 10), rawDamage / 10)) - (this.defenseUp ? this.defense * 2 : this.defense);
        int appliedDamage = finalDamage;
        if (finalDamage > 0) {
            addHp(-appliedDamage);
            addFloater(new Floater((byte) 6));
        }
        if (appliedDamage < 0) {
            appliedDamage = 0;
        }
        ((Battler) this).floaters.addElement(new Floater((byte) 7, (short) 4, (short) (-appliedDamage)));
        if (attacker.stats.aiType == 1 && defpackage.ByteUtil.randRange(0, 99) < 15) {
            applyStatus((byte) 7);
        }
        this.recoilTimer = (byte) 1;
        this.recoilDir = dir;
    }

    public final void addHp(int amount) {
        this.hp += amount;
        if (this.hp > this.maxHp) {
            this.hp = this.maxHp;
        }
        if (this.hp < 0) {
            this.hp = 0;
        }
        GameLoop.gameScreen.markHpDirty();
        if (this.hp == 0) {
            setState((byte) 6);
            this.animFrame = (byte) 0;
            this.deathTimer = (byte) 24;
        }
    }

    public final void addHpPercent(int percent) {
        addHp((this.maxHp * percent) / 100);
    }

    public final void addMp(int amount) {
        this.mp += amount;
        if (this.mp > this.maxMp) {
            this.mp = this.maxMp;
        }
        if (this.mp < 0) {
            this.mp = 0;
        }
        GameLoop.gameScreen.markMpDirty();
    }

    public final void addMpPercent(int percent) {
        addMp((this.maxMp * percent) / 100);
    }

    public final void addExp(int amount) {
        int scaledExp = amount * 4;
        this.exp += scaledExp;
        while (this.exp >= this.expToNext) {
            this.exp -= this.expToNext;
            levelUp();
        }
        if (this.exp < 0) {
            this.exp = 0;
        }
        GameLoop.gameScreen.markExpDirty();
        this.activeGuardian.addExp(scaledExp);
    }

    public final void addMoney(int amount) {
        this.bag.gold += amount;
        if (this.bag.gold < 0) {
            this.bag.gold = 0;
        }
    }

    private final void levelUp() {
        if (this.level < 99) {
            this.level = (byte) (this.level + 1);
            recomputeStats();
            ((Battler) this).floaters.addElement(new Floater((byte) 3));
            ((Battler) this).floaters.addElement(new Floater((byte) 4));
            this.statPoints = (short) (this.statPoints + 3);
        }
        addHpPercent(100);
        addMpPercent(100);
        if (!defpackage.AppConfig.fullVersion || this.level < 8) {
            return;
        }
        defpackage.GameState.requestState((byte) 13);
    }

    public final void setComboDepth(byte depth) {
        if (this.maxCombo < depth) {
            this.maxCombo = depth;
        }
    }

    /* renamed from: k */
    /** Uses the currently selected quick-item, if any and if alive. */
    public final void useQuickItem() {
        Item item;
        if (this.state == 6 || this.state == 5 || (item = this.bag.currentQuickItem()) == null) {
            return;
        }
        useItem(item);
    }

    public final byte[] serializeItems(boolean z, byte b2) {
        byte[] bagBytes = this.bag.equipmentSlots(z, b2);
        int equipCount = 0;
        for (int i = 0; i < 5; i++) {
            if (this.equipment[i] != null && ((!z || (this.equipment[i] instanceof Armor)) && ((b2 != 1 || this.equipment[i].identified) && (b2 != -1 || !this.equipment[i].identified)))) {
                equipCount++;
            }
        }
        byte[] out = new byte[bagBytes.length + equipCount];
        int pos = 0;
        for (int i = 0; i < 5; i++) {
            if (this.equipment[i] != null && ((!z || (this.equipment[i] instanceof Armor)) && ((b2 != 1 || this.equipment[i].identified) && (b2 != -1 || !this.equipment[i].identified)))) {
                int p = pos;
                pos++;
                out[p] = (byte) (i + 100);
            }
        }
        System.arraycopy(bagBytes, 0, out, pos, bagBytes.length);
        return out;
    }

    /* renamed from: a */
    /** Returns the storage slot code of {@code item}: a bag slot, or 100+equipSlot, or -1. */
    public final byte slotOf(Item item) {
        byte bagSlot = this.bag.slotOf(item);
        if (bagSlot != -1) {
            return bagSlot;
        }
        for (int i = 0; i < 5; i++) {
            if (item == this.equipment[i]) {
                return (byte) (i + 100);
            }
        }
        return (byte) -1;
    }

    /* renamed from: b */
    /** Equips bag item {@code bagSlot} into equipment slot {@code equipSlot} and recomputes stats. */
    public final void equipItem(byte bagSlot, byte equipSlot) {
        this.equipment[equipSlot] = (Equipment) this.bag.replaceAt((Item) this.equipment[equipSlot], bagSlot);
        recomputeStats();
    }

    public final Item getEquip(int slot) {
        return this.equipment[slot];
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    public final Weapon getWeapon() {
        return (Weapon) this.equipment[0];
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    public final Armor getArmor() {
        return (Armor) this.equipment[1];
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    public final Equipment getAccessory1() {
        return this.equipment[2];
    }

    /* JADX INFO: renamed from: b, reason: collision with other method in class */
    public final Equipment getAccessory2() {
        return this.equipment[3];
    }

    /* JADX INFO: renamed from: c, reason: collision with other method in class */
    public final Equipment getAccessory3() {
        return this.equipment[4];
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    public final void useItem(Item item) {
        byte kind = item.type;
        if (kind == 7) {
            addHpPercent(20);
        } else if (kind == 8) {
            addHpPercent(40);
        } else if (kind == 10) {
            for (int i = 0; i < ((Battler) this).statuses.size(); i++) {
                StatusIcon icon = (StatusIcon) ((Battler) this).statuses.elementAt(i);
                if (icon.kind == 7) {
                    icon.expire();
                    break;
                }
            }
        } else if (kind == 9) {
            addMpPercent(30);
        } else {
            defpackage.Debug.assertTrue(false);
        }
        this.bag.decrementItem(item, (byte) 1);
        GameLoop.gameScreen.markRedraw();
    }

    /* renamed from: e */
    /** Reloads the sprite bank for equipment slot {@code slot} after an equip change. */
    public final void reloadEquipSprite(byte slot) {
        byte guardianElement = 0;
        if (this.activeGuardian != null) {
            guardianElement = this.activeGuardian.element();
        }
        switch (slot) {
            case 0:
                AssetLoader.loadWeaponSprite(defpackage.GameState.classId, (Weapon) this.equipment[0], false, guardianElement);
                break;
            case 1:
                AssetLoader.loadShieldSprite(defpackage.GameState.classId, this.equipment[1].subId);
                break;
            case 2:
                AssetLoader.loadArmorSprite(defpackage.GameState.classId, this.equipment[2].subId);
                break;
            case 3:
                AssetLoader.loadHeadSprite(defpackage.GameState.classId, this.equipment[3].subId);
                break;
        }
    }

    /* renamed from: a */
    /** Finds the existing guardian of {@code type}, or creates one in a free slot. */
    public final Guardian findOrCreateGuardian(byte type) {
        for (int i = 0; i < this.guardians.length; i++) {
            if (this.guardians[i] != null && this.guardians[i].type == type) {
                return null;
            }
        }
        for (int i = 0; i < this.guardians.length; i++) {
            if (this.guardians[i] == null) {
                this.guardians[i] = new Guardian((short) 0, (short) 0, type);
                return this.guardians[i];
            }
        }
        return null;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    public final Guardian getActiveGuardian() {
        defpackage.Debug.assertTrue(this.activeGuardian != null);
        return this.activeGuardian;
    }

    public final boolean setActiveGuardian(Guardian guardian) {
        if (this.activeGuardian != null && this.activeGuardian.castState != 0) {
            return false;
        }
        detachGuardian();
        this.activeGuardian = guardian;
        return true;
    }

    /* renamed from: l */
    /** Begins the guardian-summon sequence: switch screen, add it to the map, and load its assets. */
    public final void beginGuardianSummon() {
        defpackage.GameState.setScreen(1);
        addGuardianToMap();
        GameLoop.instance.setLoadingFps();
        AssetLoader.loadGuardian();
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Casts the active guardian's slot-A or slot-B skill (if idle and in combat). */
    public final void castGuardianSkill(boolean useSlotA) {
        if (defpackage.GameState.map.combatEnabled && this.activeGuardian != null && this.activeGuardian.castState == 0) {
            this.activeGuardian.castSkill(useSlotA, this.facing, ((Entity) this).tileX, ((Entity) this).tileY);
        }
    }

    /* JADX INFO: renamed from: b, reason: collision with other method in class */
    /** Removes the active guardian from the map and clears it; returns the detached guardian. */
    public final Guardian detachGuardian() {
        Guardian guardian = this.activeGuardian;
        if (guardian == null) {
            return null;
        }
        if (defpackage.GameState.map != null) {
            defpackage.GameState.map.removeEntity(guardian);
        }
        this.activeGuardian = null;
        return guardian;
    }

    /* renamed from: m */
    /** Adds the active guardian entity to the current map. */
    public final void addGuardianToMap() {
        defpackage.GameState.map.addEntity(this.activeGuardian);
    }

    public final void recomputeStats() {
        Equipment[] equip = this.equipment;
        this.strengthBonus = (byte) 0;
        this.vitalityBonus = (byte) 0;
        this.agilityBonus = (byte) 0;
        this.spiritBonus = (byte) 0;
        for (int i = 0; i < 5; i++) {
            if (equip[i] != null) {
                this.strengthBonus = (byte) (this.strengthBonus + equip[i].enchant[0]);
                this.vitalityBonus = (byte) (this.vitalityBonus + equip[i].enchant[1]);
                this.agilityBonus = (byte) (this.agilityBonus + equip[i].enchant[2]);
                this.spiritBonus = (byte) (this.spiritBonus + equip[i].enchant[3]);
            }
        }
        this.maxHp = 0;
        this.maxMp = 0;
        this.expToNext = 0;
        this.attack = (short) 0;
        this.defense = (short) 0;
        this.maxHp = (this.vitality + this.vitalityBonus + this.level) * 12;
        this.maxMp = (this.spirit + this.spiritBonus + this.level) * 12;
        this.expToNext = (((this.level * this.level) * this.level) - (this.level * this.level)) + (80 * this.level);
        this.attack = (short) (this.attack + (equip[0] != null ? equip[0].value + ((equip[0].refineLevel * 5) / 2) : 0));
        this.attack = (short) (this.attack + (((this.strength + this.strengthBonus) * 4) / 5));
        this.defense = (short) (this.defense + (equip[1] != null ? equip[1].value + equip[1].refineLevel : 0));
        this.defense = (short) (this.defense + (equip[2] != null ? equip[2].value + (equip[2].refineLevel * 2) : 0));
        this.defense = (short) (this.defense + (equip[3] != null ? equip[3].value : (short) 0));
        this.defense = (short) (this.defense + (equip[4] != null ? equip[4].value : (short) 0));
        this.defense = (short) (this.defense + ((this.strength + this.strengthBonus) / 5));
        this.defense = (short) (this.defense + (this.level / 3));
        if (this.hp > this.maxHp) {
            this.hp = this.maxHp;
        }
        if (this.mp > this.maxMp) {
            this.mp = this.maxMp;
        }
        GameLoop.gameScreen.markRedraw();
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    public final byte[] save() {
        ByteArrayOutputStream byteArrayOutputStream = null;
        byte[] byteArray = null;
        DataOutputStream dataOutputStream = null;
        try {
            try {
                byteArrayOutputStream = new ByteArrayOutputStream();
                DataOutputStream dataOutputStream2 = new DataOutputStream(byteArrayOutputStream);
                dataOutputStream = dataOutputStream2;
                dataOutputStream2.writeByte(this.classId);
                dataOutputStream.writeByte(this.level);
                dataOutputStream.writeInt(this.hp);
                dataOutputStream.writeInt(this.mp);
                dataOutputStream.writeInt(this.exp);
                dataOutputStream.writeInt(this.maxHp);
                dataOutputStream.writeInt(this.maxMp);
                dataOutputStream.writeInt(this.expToNext);
                dataOutputStream.writeByte(this.maxCombo);
                dataOutputStream.writeShort(this.statPoints);
                dataOutputStream.writeShort(this.strength);
                dataOutputStream.writeShort(this.vitality);
                dataOutputStream.writeShort(this.agility);
                dataOutputStream.writeShort(this.spirit);
                for (int i = 0; i < 5; i++) {
                    if (this.equipment[i] == null) {
                        dataOutputStream.writeByte(0);
                    } else {
                        dataOutputStream.writeByte(1);
                        dataOutputStream.write(this.equipment[i].serialize());
                    }
                }
                for (int i2 = 0; i2 < this.guardians.length; i2++) {
                    if (this.guardians[i2] == null) {
                        dataOutputStream.writeByte(0);
                    } else {
                        dataOutputStream.writeByte(1);
                        dataOutputStream.writeByte(this.guardians[i2].type);
                        dataOutputStream.writeShort(this.guardians[i2].level);
                        dataOutputStream.writeInt(1);
                        dataOutputStream.writeInt(1);
                        dataOutputStream.writeInt(this.guardians[i2].exp);
                        dataOutputStream.writeByte(this.guardians[i2].skillSlotA);
                        dataOutputStream.writeByte(this.guardians[i2].skillSlotB);
                    }
                }
                defpackage.Debug.assertTrue(this.activeGuardian != null);
                byte activeIndex = -1;
                for (byte b3 = 0; b3 < this.guardians.length; b3 = (byte) (b3 + 1)) {
                    if (this.activeGuardian == this.guardians[b3]) {
                        activeIndex = b3;
                        break;
                    }
                }
                defpackage.Debug.assertTrue(activeIndex != -1);
                dataOutputStream.writeByte(activeIndex);
                dataOutputStream.writeInt(this.playSeconds + ((int) ((System.currentTimeMillis() / 1000) - ((long) this.sessionStartSec))));
                byteArray = byteArrayOutputStream.toByteArray();
                try {
                    dataOutputStream.close();
                    byteArrayOutputStream.close();
                } catch (IOException unused) {
                }
                return byteArray;
            } catch (IOException e) {
                e.printStackTrace();
                if (dataOutputStream != null) {
                    try {
                        dataOutputStream.close();
                    } catch (IOException unused2) {
                        return null;
                    }
                }
                if (byteArrayOutputStream != null) {
                    try {
                        byteArrayOutputStream.close();
                    } catch (IOException unusedC) {
                    }
                }
                return null;
            }
        } catch (Throwable th) {
            if (dataOutputStream != null) {
                try {
                    dataOutputStream.close();
                } catch (IOException unused3) {
                    throw th;
                }
            }
            if (byteArrayOutputStream != null) {
                try {
                    byteArrayOutputStream.close();
                } catch (IOException unusedC) {
                }
            }
            throw th;
        }
    }

    /* JADX WARN: Multi-variable type inference failed */
    public final void load(byte[] bArr) {
        ByteArrayInputStream byteArrayInputStream = null;
        DataInputStream dataInputStream = null;
        try {
            try {
                byteArrayInputStream = new ByteArrayInputStream(bArr);
                dataInputStream = new DataInputStream(byteArrayInputStream);
                this.classId = dataInputStream.readByte();
                this.level = dataInputStream.readByte();
                this.hp = dataInputStream.readInt();
                this.mp = dataInputStream.readInt();
                this.exp = dataInputStream.readInt();
                dataInputStream.readInt();
                dataInputStream.readInt();
                dataInputStream.readInt();
                this.maxCombo = dataInputStream.readByte();
                this.statPoints = dataInputStream.readShort();
                this.strength = dataInputStream.readShort();
                this.vitality = dataInputStream.readShort();
                this.agility = dataInputStream.readShort();
                this.spirit = dataInputStream.readShort();
                for (int i = 0; i < 5; i++) {
                    if (dataInputStream.readByte() != 0) {
                        byte[] bArr2 = new byte[10];
                        dataInputStream.read(bArr2);
                        this.equipment[i] = (Equipment) Item.deserialize(bArr2);
                    }
                }
                defpackage.Debug.assertTrue(this.guardians[0] == null);
                defpackage.Debug.assertTrue(this.activeGuardian == null);
                for (int i2 = 0; i2 < this.guardians.length; i2++) {
                    if (dataInputStream.readByte() != 0) {
                        Guardian guardian = findOrCreateGuardian(dataInputStream.readByte());
                        guardian.level = dataInputStream.readShort();
                        dataInputStream.readInt();
                        dataInputStream.readInt();
                        guardian.exp = dataInputStream.readInt();
                        guardian.equipSkill(true, dataInputStream.readByte(), true);
                        guardian.equipSkill(false, dataInputStream.readByte(), true);
                        guardian.recomputeExpToNext();
                    }
                }
                setActiveGuardian(this.guardians[dataInputStream.readByte()]);
                this.playSeconds = dataInputStream.readInt();
                try {
                    dataInputStream.close();
                    byteArrayInputStream.close();
                } catch (IOException unused) {
                }
            } catch (IOException e) {
                e.printStackTrace();
                if (dataInputStream != null) {
                    try {
                        dataInputStream.close();
                    } catch (IOException unused2) {
                    }
                }
                if (byteArrayInputStream != null) {
                    try {
                        byteArrayInputStream.close();
                    } catch (IOException unusedC) {
                    }
                }
            }
            recomputeStats();
        } catch (Throwable th) {
            if (dataInputStream != null) {
                try {
                    dataInputStream.close();
                } catch (IOException unused3) {
                    throw th;
                }
            }
            if (byteArrayInputStream != null) {
                try {
                    byteArrayInputStream.close();
                } catch (IOException unusedC) {
                }
            }
            throw th;
        }
    }
}
