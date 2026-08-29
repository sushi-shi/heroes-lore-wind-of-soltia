package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: av */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:av.class */
/**
 * Abstract base for the game's multi-tile bosses (the concrete subclasses are
 * the scripted encounters such as {@link RockyBoss} and {@link GebHead}). It
 * refines {@link Enemy}: the AI tick caches the hero offset each frame
 * ({@link #heroDistX}/{@link #heroDistY}); drawing uses the wider 16-cell boss
 * sprite bank; and the damage-intake paths ({@link #takeGuardianHit},
 * {@link #takeHeroHit}) use a boss-tuned defense/dodge formula and invoke the
 * abstract {@link #onDeath} cleanup instead of dropping loot. Death simply
 * unregisters the boss from the map.
 */
public abstract class Boss extends Enemy {
    /* renamed from: f */
    /** Cached absolute tile-column distance to the hero (recomputed each tick). */
    public byte heroDistX;

    /* renamed from: g */
    /** Cached absolute tile-row distance to the hero (recomputed each tick). */
    public byte heroDistY;

    public Boss(byte tileX, byte tileY, byte halfWidth, byte halfHeight, byte layer) {
        super((short) (tileX << 4), (short) (tileY << 4), halfWidth, halfHeight);
        ((Entity) this).layer = layer;
        setOccupancy();
    }

    @Override // defpackage.al, defpackage.o
    public void update() {
        this.animFrame = (byte) (this.animFrame + 1);
        Hero hero = defpackage.GameState.hero();
        this.heroDistX = tileDistX(hero);
        this.heroDistY = tileDistY(hero);
        updateAi();
        stepIfMoving();
        animate();
    }

    /* renamed from: m */
    /** Per-boss despawn/cleanup, run when the boss dies. */
    public abstract void onDeath();

    @Override // defpackage.al, defpackage.ck
    public void paint(Graphics graphics, int originX, int originY) {
        int frameGroup;
        int screenX = originX + ((Entity) this).pixelX + ((Entity) this).halfW + ((((Entity) this).layer - 1) * 8);
        int screenY = originY + ((Entity) this).pixelY + ((Entity) this).halfH;
        switch (this.state) {
            case 2:
                frameGroup = (this.statRow * 16) + 4 + (this.moveDir - 1);
                break;
            case 3:
                frameGroup = (this.statRow * 16) + 12 + (this.moveDir - 1);
                break;
            case 4:
            default:
                frameGroup = (this.statRow * 16) + 0 + (this.moveDir - 1);
                break;
            case 5:
                frameGroup = (this.statRow * 16) + 8 + (this.moveDir - 1);
                break;
        }
        if (AssetCache.bossFrames[frameGroup] == null) {
            frameGroup = (this.statRow * 16) + 0 + (this.moveDir - 1);
        }
        GameScreen.drawFrameGroup(graphics, (byte[]) AssetCache.bossFrames[frameGroup], this.animFrame, screenX, screenY);
        drawStatusIcons(graphics, screenX, screenY - (((Enemy) this).stats.size * 3));
        drawFloaters(graphics, screenX, screenY);
    }

    /* renamed from: a */
    @Override // defpackage.al
    public final void takeGuardianHit(int rawDamage, byte guardianElement) {
        if (this.state == 6 || this.state == 5) {
            return;
        }
        if (rawDamage < 0) {
            rawDamage = 0;
        }
        int finalDamage = (rawDamage * Directions.elementDamageMultiplier[guardianElement][((Enemy) this).stats.element]) / 10;
        ((Enemy) this).hp = (short) (((Enemy) this).hp - finalDamage);
        ((Battler) this).floaters.addElement(new Floater((byte) 7, (short) 4, (short) finalDamage));
        ((Battler) this).floaters.addElement(new Floater((byte) 1));
        if (((Enemy) this).hp <= 0) {
            setState((byte) 5);
            this.animFrame = (byte) 0;
            onDeath();
        }
    }

    /* renamed from: a */
    @Override // defpackage.al
    public final void takeHeroHit(int rawDamage, boolean knockback, byte attackerDir, boolean crit, byte hitFloaterKind, byte procKind, Hero hero) {
        if (this.state == 6 || this.state == 5) {
            return;
        }
        GameLoop.gameScreen.setTarget((Enemy) this, false);
        Weapon weapon = (Weapon) hero.getEquip(0);
        byte guardianElement = hero.getActiveGuardian().element();
        int afterDefense = rawDamage - ((Enemy) this).stats.defense;
        int afterDefenseClamped = afterDefense;
        if (afterDefense < 0) {
            afterDefenseClamped = 0;
        }
        int finalDamage = (afterDefenseClamped * Directions.elementDamageMultiplier[guardianElement][((Enemy) this).stats.element]) / 10;
        if (crit) {
            finalDamage += (finalDamage * weapon.critBonus) / 10;
        }
        boolean dodged = defpackage.ByteUtil.randRange(0, 99) < ((((((Enemy) this).stats.evasion - (hero.agility + hero.agilityBonus)) - (((Equipment) weapon).refineLevel / 5)) + 10) * 2);
        boolean dodgedSfx = dodged;
        if (dodged) {
            ((Battler) this).floaters.addElement(new Floater((byte) 2));
        } else {
            switch (procKind) {
                case 3:
                    hero.addMp((finalDamage * 30) / 100);
                    break;
                case 4:
                    hero.addHp(finalDamage / 2);
                    break;
                case 8:
                    finalDamage *= 2;
                    break;
            }
            ((Battler) this).floaters.addElement(new Floater(hitFloaterKind));
            damage(finalDamage);
            if (((Enemy) this).hp <= 0) {
                onDeath();
            }
        }
        if (dodgedSfx) {
            AudioManager.playSfx((byte) 14, false);
        } else if (crit) {
            AudioManager.playSfx((byte) 15, false);
        } else {
            AudioManager.playSfx((byte) 13, false);
        }
    }

    /* renamed from: l */
    @Override // defpackage.al
    public void die() {
        defpackage.GameState.map.removeEntity(this);
    }
}
