package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: ak */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ak.class */
/**
 * Right hand of the three-part <b>Geb</b> encounter (enemy-data record 41, size
 * 1, anchored at column 13), constructed by {@link GameMap#spawnGebBoss()} and
 * owned by the {@link GebHead} core. Like the left hand it slides vertically to
 * line up on the hero, but it alternates between two different sweep attacks: a
 * long reach that connects on frame 5 ({@link #swingSide} == 1) and a shorter
 * reach that connects on frame 8 ({@link #swingSide} == 2). The side is flipped
 * each swing through the direction turn-table. Its retract ({@code state == 3})
 * plays out the attack sprite before returning to idle. It paints 64 pixels high.
 */
public final class GebHandRight extends Boss {
    /* renamed from: v */
    /** Ticks since the last landed sweep; once past 100 the attack delay is removed. */
    private byte ticksSinceHit;

    /* renamed from: w */
    /** Which of the two sweeps to run next (1 = long reach @f5, 2 = short reach @f8); toggles each swing. */
    private byte swingSide;

    public GebHandRight(GameMap map, byte tileX, byte tileY, byte kind, byte statRow) {
        super(tileX, (byte) (tileY + 4), kind, statRow, (byte) 1);
        this.swingSide = (byte) 2;
        map.occupancy[tileY + 4][tileX] = null;
        map.occupancy[tileY + 4][tileX + 1] = null;
        ((Enemy) this).stats.attackDelay = (byte) 2;
        this.ticksSinceHit = (byte) 0;
    }

    /* renamed from: d */
    @Override // defpackage.Boss, defpackage.al, defpackage.o
    public final void update() {
        this.animFrame = (byte) (this.animFrame + 1);
        this.ticksSinceHit = (byte) (this.ticksSinceHit + 1);
        if (this.ticksSinceHit > 100) {
            ((Enemy) this).stats.attackDelay = (byte) 0;
        }
        if (this.state == 3) {
            if (this.animFrame >= ((byte[]) AssetCache.bossFrames[(this.statRow * 16) + 12 + (this.moveDir - 1)])[0]) {
                enterIdle(false);
                tryAttack();
            }
        } else {
            updateAi();
        }
        if (this.state == 2) {
            switch (this.facing) {
                case 1:
                    ((Entity) this).pixelY = (short) (((Entity) this).pixelY - 8);
                    break;
                case 2:
                    ((Entity) this).pixelY = (short) (((Entity) this).pixelY + 8);
                    break;
            }
            syncTile();
        }
        animate();
    }

    /* renamed from: a */
    @Override // defpackage.Boss, defpackage.al, defpackage.ck
    public final void paint(Graphics graphics, int originX, int originY) {
        byte savedMoveDir = this.moveDir;
        if (this.state != 3) {
            this.moveDir = (byte) 1;
        }
        super.paint(graphics, originX, originY - 64);
        this.moveDir = savedMoveDir;
    }

    /* renamed from: i */
    @Override // defpackage.al
    public final void tryAttack() {
        Hero hero = defpackage.GameState.hero();
        int heroRowOffset = ((Entity) hero).tileY - ((((Entity) this).tileY - 4) + 2);
        if (this.hurtCooldown == 0 && heroRowOffset >= -1 && heroRowOffset <= 2 && ((Entity) hero).tileX >= ((Entity) this).tileX - 7) {
            beginAttack();
            this.swingSide = Directions.reverse[this.swingSide];
            setFacing(this.swingSide);
        } else if (this.attackCooldown == 0) {
            if (heroRowOffset > 2) {
                setState((byte) 2);
                setFacing((byte) 2);
            } else if (heroRowOffset < -1) {
                setState((byte) 2);
                setFacing((byte) 1);
            } else {
                setState((byte) 1);
                setFacing((byte) 2);
            }
        }
    }

    /* renamed from: j */
    @Override // defpackage.al
    public final void resolveAttack() {
        Hero hero = defpackage.GameState.hero();
        if (this.animFrame == 5 && this.swingSide == 1) {
            byte hitMinX = (byte) (((Entity) this).tileX - 7);
            byte hitMaxX = (byte) (((Entity) this).tileX - 1);
            byte hitMinY = (byte) ((((Entity) this).tileY - 4) + 1);
            byte hitMaxY = (byte) ((((Entity) this).tileY - 4) + 4);
            if (((Entity) hero).tileX < hitMinX || ((Entity) hero).tileX > hitMaxX || ((Entity) hero).tileY < hitMinY || ((Entity) hero).tileY > hitMaxY) {
                return;
            }
            hero.takeHit((Enemy) this, (byte) 3);
            return;
        }
        if (this.animFrame == 8 && this.swingSide == 2) {
            byte hitMinX = (byte) (((Entity) this).tileX - 5);
            byte hitMaxX = (byte) (((Entity) this).tileX - 1);
            byte hitMinY = (byte) ((((Entity) this).tileY - 4) + 1);
            byte hitMaxY = (byte) ((((Entity) this).tileY - 4) + 4);
            if (((Entity) hero).tileX < hitMinX || ((Entity) hero).tileX > hitMaxX || ((Entity) hero).tileY < hitMinY || ((Entity) hero).tileY > hitMaxY) {
                return;
            }
            hero.takeHit((Enemy) this, (byte) 2);
            this.ticksSinceHit = (byte) 0;
            ((Enemy) this).stats.attackDelay = (byte) 2;
        }
    }

    /* renamed from: m */
    @Override // defpackage.Boss
    public final void onDeath() {
        this.deathTimer = (byte) 0;
    }
}
