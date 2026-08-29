package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: al */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:al.class */
/**
 * A hostile monster actor (the base class of every {@link Boss}). Its behaviour
 * is driven by a shared {@link EnemyType} stat template ({@link #stats}): the AI
 * tick ({@link #update}) runs a target-selection / chase / attack / animation
 * pipeline, and two damage-intake paths resolve incoming hits — one from a
 * guardian's area attack ({@link #takeGuardianHit}) and the full hero-attack
 * resolution ({@link #takeHeroHit}) with defense, element multipliers, evasion,
 * critical and weapon-proc effects. On death ({@link #die}) it rolls the loot
 * table, drops money, and awards experience.
 */
public class Enemy extends Battler {
    /* renamed from: m */
    /** Enemy kind (category; indexes {@link EnemyType#attackHitFrame} and sprites). */
    public byte kind;

    /* renamed from: n */
    /** Stat-row: index into {@link EnemyType#types}. */
    public byte statRow;

    /* renamed from: a */
    /** Spawn tile X (home position used when queueing a respawn). */
    private int homeTileX;

    /* renamed from: b */
    /** Spawn tile Y (home position used when queueing a respawn). */
    private int homeTileY;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Shared stat template for this monster kind. */
    public EnemyType stats;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Current hit points (max is {@code stats.maxHp}). */
    public short hp;

    /* renamed from: r */
    /** Attack charge accumulated while chasing (fuels a rush attack). */
    private byte attackCharge;

    /* renamed from: o */
    /** Frames remaining before the next attack is allowed. */
    public byte attackCooldown;

    /* renamed from: p */
    /** Recovery frames remaining after being hit. */
    public byte hurtCooldown;

    /* renamed from: q */
    /** Death-animation countdown (state 5) before {@link #die} resolves. */
    public byte deathTimer;

    /* renamed from: s */
    /** Hit-recoil display countdown (shakes the sprite while > 0). */
    private byte recoilTimer;

    /* renamed from: t */
    /** Direction the recoil shake is offset toward. */
    private byte recoilDir;

    /* renamed from: d */
    /** True once the enemy has been engaged (hit or aggroed). */
    private boolean aggroed;

    /* renamed from: e */
    /** True while spawned hidden (ambush) or between summon phases; suppresses drawing. */
    private boolean hidden;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Which status effects (0..4) are currently active on this enemy. */
    private boolean[] statusFlags;

    /* renamed from: c */
    /** Current AI target entity (hero or guardian), or {@code null}. */
    private Entity target;

    /* renamed from: u */
    /** Summon cooldown for summoner enemies (-10 = idle, counts down from 40). */
    private byte summonTimer;

    /* renamed from: f */
    /** Whether the enemy was on screen last frame (off-screen AI is throttled). */
    private boolean onScreen;

    public Enemy(short pixelX, short pixelY, byte kind, byte statRow) {
        super(pixelX, pixelY, (byte) 8, (byte) 8);
        this.attackCharge = (byte) 0;
        this.attackCooldown = (byte) 0;
        this.hurtCooldown = (byte) 0;
        this.deathTimer = (byte) 0;
        this.recoilTimer = (byte) 0;
        this.recoilDir = (byte) 0;
        this.homeTileX = ((Entity) this).tileX;
        this.homeTileY = ((Entity) this).tileY;
        this.kind = kind;
        this.statRow = statRow;
        this.stats = defpackage.EnemyType.types[statRow];
        this.hp = this.stats.maxHp;
        this.attackCooldown = this.stats.attackDelay;
        this.hurtCooldown = this.stats.hurtDelay;
        this.summonTimer = (byte) -10;
        this.statusFlags = new boolean[5];
        this.aggroed = false;
        if (this.stats.ambush) {
            this.hidden = true;
        }
        ((Entity) this).layer = this.stats.size == 2 ? (byte) 2 : (byte) 1;
        setPixelPos(pixelX, pixelY);
        this.onScreen = true;
    }

    @Override // defpackage.ck
    public final void setPixelPos(short pixelX, short pixelY) {
        clearOccupancy();
        super.setPixelPos(pixelX, pixelY);
        syncTile();
        setOccupancy();
    }

    @Override // defpackage.ck
    public void paint(Graphics graphics, int originX, int originY) {
        int frameGroup;
        int screenX = originX + ((Entity) this).pixelX + ((Entity) this).halfW + ((((Entity) this).layer - 1) * 8);
        int screenY = originY + ((Entity) this).pixelY + ((Entity) this).halfH;
        if (screenX + 16 < 0 || screenY < 0 || screenX - 16 > GameScreen.width || screenY > GameScreen.worldHeight + 32) {
            drawFloaters(graphics, screenX, screenY);
            this.onScreen = false;
            return;
        }
        this.onScreen = true;
        if (this.hidden) {
            return;
        }
        int drawX = screenX;
        int drawY = screenY;
        if (this.recoilTimer == 3 || this.recoilTimer == 1) {
            drawY += defpackage.Directions.dirDy[this.recoilDir] * 3;
            drawX += defpackage.Directions.dirDx[this.recoilDir] * 3;
        }
        if ((this.kind != 22 && this.kind != 16 && !(this instanceof Boss)) || (this instanceof RockyBoss)) {
            if (((Entity) this).layer == 1) {
                graphics.drawImage(AssetCache.entityShadow, drawX, drawY - 3, 17);
            } else {
                graphics.setColor(2047807);
                graphics.fillArc(drawX - 11, drawY - 6, 22, 9, 0, 360);
            }
        }
        switch (this.state) {
            case 2:
                frameGroup = (this.statRow * 12) + 4 + (this.moveDir - 1);
                break;
            case 3:
                frameGroup = (this.statRow * 12) + 8 + (this.moveDir - 1);
                break;
            default:
                frameGroup = (this.statRow * 12) + 0 + (this.moveDir - 1);
                break;
        }
        GameScreen.drawFrameGroup(graphics, (byte[]) AssetCache.enemyFrames[frameGroup], this.animFrame, drawX, drawY);
        drawStatusIcons(graphics, screenX, screenY - (this.stats.size * 3));
        drawFloaters(graphics, screenX, screenY);
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Override: sets state without resetting the animation frame counter. */
    @Override // defpackage.o
    public final void setState(byte newState) {
        this.state = newState;
    }

    @Override // defpackage.o
    public void update() {
        this.animFrame = (byte) (this.animFrame + 1);
        tickStatuses();
        if (!this.onScreen) {
            Hero hero = defpackage.GameState.hero();
            byte distX = tileDistX(hero);
            byte distY = tileDistY(hero);
            if ((distX > this.stats.sightRange || distY > this.stats.sightRange) && this.target == null) {
                return;
            }
        }
        if (this.recoilTimer > 0) {
            this.recoilTimer = (byte) (this.recoilTimer - 1);
        }
        updateAi();
        stepIfMoving();
        animate();
    }

    /* renamed from: s */
    /** Ticks each active status icon; poison (kind 3) deals 15-25 damage every 8 frames. */
    private final void tickStatuses() {
        for (int i = ((Battler) this).statuses.size() - 1; i >= 0; i--) {
            StatusIcon icon = (StatusIcon) ((Battler) this).statuses.elementAt(i);
            icon.tick();
            if (icon.kind == 3 && icon.elapsed() % 8 == 0) {
                damage(defpackage.ByteUtil.randRange(15, 25));
            }
            if (((Overlay) icon).finished) {
                ((Battler) this).statuses.removeElementAt(i);
                this.statusFlags[icon.kind] = false;
            }
        }
    }

    /* renamed from: n */
    /** AI state machine: dispatches idle/chase/attack/knockback/death behaviour. */
    public void updateAi() {
        boolean aligned = (((Entity) this).offGridX || ((Entity) this).offGridY) ? false : true;
        if (this.state == 5) {
            if (this.deathTimer >= 1) {
                this.deathTimer = (byte) (this.deathTimer - 1);
                return;
            }
            die();
        }
        if (this.statusFlags[0] || this.statusFlags[2]) {
            enterIdle(false);
            return;
        }
        switch (this.state) {
            case 1:
                tryAttack();
                break;
            case 2:
                if (aligned) {
                    chase();
                }
                break;
            case 3:
                if (this.animFrame >= this.stats.castFrames) {
                    enterIdle(false);
                    tryAttack();
                }
                break;
            case 4:
                if (this.knockbackTimer < 1) {
                    setState((byte) 1);
                }
                this.knockbackTimer = (byte) (this.knockbackTimer - 1);
                break;
        }
    }

    /* renamed from: h */
    /** Chases: either attacks now, or builds attack charge and keeps pressing. */
    public void chase() {
        if (this.attackCharge >= this.stats.attackDelay * 2 || Entity.rng.nextInt() <= 0) {
            enterIdle(false);
            tryAttack();
        } else {
            this.attackCharge = (byte) (this.attackCharge + this.stats.attackDelay);
            this.attackCooldown = (byte) 0;
            tryAttack();
        }
    }

    /* renamed from: i */
    /** Attempts an attack: strike the hero if in range, else pick a target and approach. */
    public void tryAttack() {
        Hero hero = defpackage.GameState.hero();
        Guardian guardian = hero.getActiveGuardian();
        if (this.hurtCooldown == 0) {
            if ((this.stats.aiType != 0 && this.stats.aiType != 1) || entityInDir(this.facing, hero) != hero) {
                if (this.stats.aiType == 2 || this.stats.aiType == 3) {
                    byte dist = 1;
                    while (true) {
                        byte d = dist;
                        if (d > 3) {
                            break;
                        }
                        if (neighbor(this.facing, d) == hero) {
                            beginAttack();
                            return;
                        }
                        dist = (byte) (d + 1);
                    }
                }
            } else {
                beginAttack();
                return;
            }
        }
        if (this.attackCooldown == 0) {
            byte range = 1;
            if (this.stats.aiType == 2 || this.stats.aiType == 3) {
                range = 3;
            }
            if (this.target == guardian && guardian.castState == 2) {
                approach(guardian, range);
                return;
            }
            if (this.target == hero && !guardian.isBusy()) {
                approach(hero, range);
                return;
            }
            Entity picked = pickTarget(hero, guardian);
            if (picked == null) {
                wander();
            } else {
                approach(picked, range);
                this.target = picked;
            }
        }
    }

    /* renamed from: o */
    /** Advances animation frames, ticks cooldowns, and runs the summon timer. */
    public void animate() {
        if (this.stats.summonsAllies && this.aggroed) {
            if (this.summonTimer > 0) {
                this.summonTimer = (byte) (this.summonTimer - 1);
            }
            if (this.summonTimer == 0) {
                defpackage.GameState.map.spawnEnemyAt(((Entity) this).tileX, ((Entity) this).tileY, this.kind, this.statRow, true, (byte) 1, (byte) 5);
                this.summonTimer = (byte) -10;
            }
        }
        switch (this.state) {
            case 2:
                if (this.animFrame >= this.stats.attackFrames) {
                    this.animFrame = (byte) 0;
                }
                break;
            case 3:
                resolveAttack();
                break;
            case 4:
            default:
                if (this.animFrame >= this.stats.walkFrames) {
                    this.animFrame = (byte) 0;
                }
                if (this.hurtCooldown > 0) {
                    this.hurtCooldown = (byte) (this.hurtCooldown - 1);
                }
                if (this.attackCooldown > 0) {
                    this.attackCooldown = (byte) (this.attackCooldown - 1);
                }
                break;
            case 5:
                stepDeathAnim();
                break;
        }
    }

    /* renamed from: j */
    /** On the attack's hit frame, applies the strike to the hero (melee, ranged or projectile). */
    public void resolveAttack() {
        Hero hero = defpackage.GameState.hero();
        this.hidden = false;
        if (this.animFrame != defpackage.EnemyType.attackHitFrame[this.kind] - 1) {
            return;
        }
        if ((this.stats.aiType == 0 || this.stats.aiType == 1) && entityInDir(this.facing, hero) == hero) {
            hero.takeHit(this, this.facing);
            return;
        }
        if (this.stats.aiType == 2) {
            byte dist = 1;
            while (true) {
                byte d = dist;
                if (d > 3) {
                    return;
                }
                if (neighbor(this.facing, d) == hero) {
                    hero.addFloater(new Floater((byte) 9, (short) -1, this.statRow));
                    hero.takeHit(this, this.facing);
                    return;
                }
                dist = (byte) (d + 1);
            }
        } else {
            if (this.stats.aiType != 3) {
                return;
            }
            byte dist2 = 1;
            while (true) {
                byte d2 = dist2;
                if (d2 > 3) {
                    return;
                }
                if (neighbor(this.facing, d2) == hero) {
                    defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX + defpackage.Directions.dirDx[this.moveDir]), (byte) (((Entity) this).tileY + defpackage.Directions.dirDy[this.moveDir]), (byte[]) AssetCache.attackEffectScripts[this.statRow], this, this.moveDir, (byte) 3, (byte) 2));
                    return;
                }
                dist2 = (byte) (d2 + 1);
            }
        }
    }

    /* renamed from: k */
    /** Wraps the death animation frame counter. */
    public void stepDeathAnim() {
        if (this.animFrame >= this.stats.walkFrames) {
            this.animFrame = (byte) 0;
        }
    }

    /* renamed from: a */
    /** Spawns this enemy's effect sprite at tile ({@code tileX2},{@code tileY2}). */
    public final void spawnEffectAt(byte tileX2, byte tileY2) {
        defpackage.GameState.map.addEntity(new Effect(tileX2, tileY2, (byte[]) AssetCache.attackEffectScripts[this.statRow]));
    }

    /* renamed from: p */
    /** Spawns the death explosion effect for this enemy's size. */
    public final void deathEffect() {
        defpackage.GameState.map.addEntity(new Effect(((Entity) this).tileX, ((Entity) this).tileY, (byte[]) AssetCache.deathFxScripts[this.stats.size]));
    }

    /* renamed from: a */
    /**
     * Returns to idle (state 1), arming the attack cooldown (plus banked charge)
     * and the hurt cooldown; {@code quickRecover} randomly shortens the latter.
     */
    public final void enterIdle(boolean quickRecover) {
        this.attackCooldown = (byte) (this.stats.attackDelay + this.attackCharge);
        this.attackCharge = (byte) 0;
        if (this.statusFlags[1]) {
            this.hurtCooldown = (byte) ((this.stats.hurtDelay * 2) + 1);
        } else {
            this.hurtCooldown = (byte) (this.stats.hurtDelay + 1);
        }
        if (quickRecover) {
            this.hurtCooldown = (byte) ((this.hurtCooldown * defpackage.ByteUtil.randRange(1, 7)) / 10);
        }
        setState((byte) 1);
        this.animFrame = (byte) 0;
    }

    /* renamed from: q */
    /** Begins the attack animation (state 3), arming the cooldowns. */
    public final void beginAttack() {
        this.hidden = false;
        this.attackCooldown = (byte) (this.stats.attackDelay + this.attackCharge);
        this.attackCharge = (byte) 0;
        if (this.statusFlags[1]) {
            this.hurtCooldown = (byte) ((this.stats.hurtDelay * 2) + 1);
        } else {
            this.hurtCooldown = (byte) (this.stats.hurtDelay + 1);
        }
        setState((byte) 3);
        this.animFrame = (byte) 0;
    }

    /* renamed from: r */
    /** Random idle wander: step or stand, then face a random direction. */
    public final void wander() {
        if (Entity.rng.nextInt() > 0) {
            setState((byte) 2);
        } else {
            enterIdle(true);
        }
        setFacing((byte) (((Entity.rng.nextInt() & 255) % 4) + 1));
    }

    /* renamed from: l */
    /**
     * Death: enters the corpse state, queues a home respawn (unless unique),
     * rolls the weighted loot table, drops money and a rare item, and awards the
     * hero experience scaled by the level gap.
     */
    public void die() {
        setState((byte) 6);
        if (this.stats.elemColor != 2) {
            defpackage.GameState.map.queueEnemySpawn(this.kind, this.stats.expReward, this.homeTileX, this.homeTileY);
        }
        int dropRoll = defpackage.ByteUtil.randRange(1, 150);
        int dropCount = this.stats.dropTable.length / 3;
        byte dropKind = -1;
        byte dropParam = -1;
        for (int i = 0; i < dropCount; i++) {
            int remaining = dropRoll - this.stats.dropTable[(i * 3) + 2];
            dropRoll = remaining;
            if (remaining <= 0) {
                if (this.stats.dropTable[(i * 3) + 2] == 1) {
                    if (defpackage.ByteUtil.randRange(1, 100) <= 20) {
                        dropKind = this.stats.dropTable[i * 3];
                        dropParam = this.stats.dropTable[(i * 3) + 1];
                        break;
                    }
                } else {
                    dropKind = this.stats.dropTable[i * 3];
                    dropParam = this.stats.dropTable[(i * 3) + 1];
                    break;
                }
            }
        }
        if (dropKind != -1) {
            defpackage.GameState.map.queueEnemySpawn(((Entity) this).tileX, ((Entity) this).tileY, dropKind, dropParam);
        }
        if (defpackage.ByteUtil.randRange(1, 100) <= 60) {
            defpackage.GameState.map.dropPickup(((Entity) this).tileX, ((Entity) this).tileY, (short) (this.stats.level * 3));
        }
        if (defpackage.ByteUtil.randRange(1, 100) <= 20 + (this.stats.level - defpackage.GameState.hero().level)) {
            defpackage.GameState.map.queueEnemySpawn(((Entity) this).tileX, ((Entity) this).tileY, (byte) 11, (byte) 0);
        }
        int expScale = 20 - (defpackage.GameState.hero().level - this.stats.level);
        if (expScale > 26) {
            expScale = 26;
        }
        int expGain = (this.stats.level * expScale) / 2;
        if (expGain > 0) {
            defpackage.GameState.hero().addExp(expGain);
        }
        deathEffect();
    }

    /* renamed from: a */
    /** Absolute tile-column distance to {@code other}. */
    public final byte tileDistX(Entity other) {
        int dx = other.tileX - ((Entity) this).tileX;
        return dx > 0 ? (byte) dx : (byte) (-dx);
    }

    /* renamed from: b */
    /** Absolute tile-row distance to {@code other}. */
    public final byte tileDistY(Entity other) {
        int dy = other.tileY - ((Entity) this).tileY;
        return dy > 0 ? (byte) dy : (byte) (-dy);
    }

    /* renamed from: a */
    /**
     * Applies damage from a guardian's area attack: scaled by the element table
     * {@code Directions.elementDamageMultiplier[guardianElement][element] / 10}, then floated and, if
     * lethal, starts the death sequence.
     */
    public void takeGuardianHit(int rawDamage, byte guardianElement) {
        if (this.state == 6 || this.state == 5) {
            return;
        }
        clearStun();
        this.aggroed = true;
        this.hidden = false;
        if (rawDamage < 0) {
            rawDamage = 0;
        }
        int finalDamage = (rawDamage * defpackage.Directions.elementDamageMultiplier[guardianElement][this.stats.element]) / 10;
        this.hp = (short) (this.hp - finalDamage);
        ((Battler) this).floaters.addElement(new Floater((byte) 7, (short) 4, (short) finalDamage));
        ((Battler) this).floaters.addElement(new Floater((byte) 1));
        this.recoilTimer = (byte) 4;
        this.recoilDir = (byte) 4;
        if (this.hp <= 0) {
            setState((byte) 5);
            this.animFrame = (byte) 0;
            this.deathTimer = (byte) 3;
        }
    }

    /* renamed from: a */
    /**
     * Full hero-attack resolution.
     * <p>Damage pipeline: raw {@code rawDamage} is halved if the enemy is armoured,
     * then {@code defense} is subtracted (only half of it while defense-broken,
     * status 4), then multiplied by the element table
     * {@code Directions.elementDamageMultiplier[guardianElement][element] / 10}, then a critical adds
     * {@code weapon.critBonus/10}. Weapon procs ({@code procKind}) can instakill (2),
     * drain MP to the hero (3), life-steal (4) or double the hit (8).
     * <p>Hit chance: the attack is dodged when a 0-99 roll is below
     * {@code clamp(evasion - (agility+bonus) - accuracy/5 + 10, .., 50)}.
     *
     * @param rawDamage      pre-mitigation hero damage
     * @param knockback      whether a hit knocks the enemy back
     * @param attackerDir    direction the hero struck from
     * @param crit           whether this is a critical hit
     * @param hitFloaterKind floating-text kind shown on a landed hit
     * @param procKind       weapon-effect proc index (-1 = none)
     * @param hero           the attacking hero
     */
    public void takeHeroHit(int rawDamage, boolean knockback, byte attackerDir, boolean crit, byte hitFloaterKind, byte procKind, Hero hero) {
        if (this.state == 6 || this.state == 5) {
            return;
        }
        GameLoop.gameScreen.setTarget(this, false);
        Weapon weapon = (Weapon) hero.getEquip(0);
        byte guardianElement = hero.getActiveGuardian().element();
        if (!this.aggroed && this.stats.summonsAllies && guardianElement != this.stats.summonWardElement) {
            this.summonTimer = (byte) 40;
        }
        clearStun();
        this.aggroed = true;
        this.hidden = false;
        if (this.stats.armored) {
            rawDamage /= 2;
        }
        int afterDefense = this.statusFlags[4] ? rawDamage - (this.stats.defense / 2) : rawDamage - this.stats.defense;
        if (afterDefense < 0) {
            afterDefense = 0;
        }
        int finalDamage = (afterDefense * defpackage.Directions.elementDamageMultiplier[guardianElement][this.stats.element]) / 10;
        if (crit) {
            finalDamage += (finalDamage * weapon.critBonus) / 10;
        }
        int dodgeChance = ((this.stats.evasion - (hero.agility + hero.agilityBonus)) - (((Equipment) weapon).refineLevel / 5)) + 10;
        boolean dodged = defpackage.ByteUtil.randRange(0, 99) < (dodgeChance > 50 ? 50 : dodgeChance);
        byte statusToInflict = procKind == -1 ? (byte) -1 : defpackage.Armor.PROC_STATUS[procKind];
        if (dodged) {
            ((Battler) this).floaters.addElement(new Floater((byte) 2));
        } else {
            switch (procKind) {
                case 2:
                    finalDamage = this.stats.maxHp;
                    break;
                case 3:
                    hero.addMp((finalDamage * 80) / 100);
                    break;
                case 4:
                    hero.addHp(finalDamage / 2);
                    break;
                case 8:
                    finalDamage *= 2;
                    break;
            }
            if (statusToInflict != -1) {
                applyStatus(statusToInflict);
                this.statusFlags[statusToInflict] = true;
            }
            ((Battler) this).floaters.addElement(new Floater(hitFloaterKind));
            if (knockback && this.hp > 0 && !((Entity) this).offGridX && !((Entity) this).offGridY) {
                setState((byte) 4);
                this.knockbackTimer = (byte) 2;
                this.facing = attackerDir;
            }
            this.recoilTimer = (byte) 4;
            this.recoilDir = attackerDir;
            damage(finalDamage);
        }
        if (dodged) {
            AudioManager.playSfx((byte) 14, false);
        } else if (crit) {
            AudioManager.playSfx((byte) 15, false);
        } else {
            AudioManager.playSfx((byte) 13, false);
        }
    }

    /* renamed from: t */
    /** Clears the freeze/stun status (index 0) and removes its icon. */
    private final void clearStun() {
        if (this.statusFlags[0]) {
            this.statusFlags[0] = false;
            for (int i = 0; i < ((Battler) this).statuses.size(); i++) {
                if (((StatusIcon) ((Battler) this).statuses.elementAt(i)).kind == 0) {
                    ((Battler) this).statuses.removeElementAt(i);
                    return;
                }
            }
        }
    }

    /* renamed from: b */
    /** Subtracts {@code amount} HP, floats the number, and starts death if lethal. */
    public final void damage(int amount) {
        this.hp = (short) (this.hp - amount);
        ((Battler) this).floaters.addElement(new Floater((byte) 7, (short) 4, (short) amount));
        if (this.hp <= 0) {
            setState((byte) 5);
            this.animFrame = (byte) -1;
            this.deathTimer = (byte) 3;
        }
    }

    /* renamed from: c */
    /** Heals {@code amount} HP, clamped to the template maximum. */
    public final void heal(int amount) {
        this.hp = (short) (this.hp + amount);
        if (this.hp > this.stats.maxHp) {
            this.hp = this.stats.maxHp;
        }
    }

    /* renamed from: c */
    /** Instantly kills a living enemy by dealing its full max HP as damage. */
    public final void slay(byte unused) {
        this.hidden = false;
        if (this.state == 6 || this.state == 5) {
            return;
        }
        damage(this.stats.maxHp);
    }

    /* renamed from: a */
    /**
     * Chooses this enemy's AI target: the guardian (70% chance) when it is nearby
     * and actively casting, otherwise the hero when within search range, else
     * {@code null}. Search range widens once engaged.
     */
    private final Entity pickTarget(Hero hero, Guardian guardian) {
        byte searchRange;
        byte guardianDistX = tileDistX(guardian);
        byte guardianDistY = tileDistY(guardian);
        if (this.aggroed) {
            searchRange = (byte) (this.stats.relentless ? 100 : 8);
        } else {
            searchRange = this.stats.sightRange;
        }
        byte reach = searchRange;
        boolean heroInRange = tileDistX(hero) <= reach && tileDistY(hero) <= reach;
        if ((guardianDistX <= reach && guardianDistY <= reach) && guardian.castState == 2 && defpackage.ByteUtil.randRange(0, 9) < 7) {
            return guardian;
        }
        if (heroInRange) {
            return hero;
        }
        return null;
    }
}
