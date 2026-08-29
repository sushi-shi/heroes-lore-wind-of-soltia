package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: ba */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ba.class */
/**
 * Left hand of the three-part <b>Geb</b> encounter (enemy-data record 40, size
 * 2, anchored at column 0), constructed by {@link GameMap#spawnGebBoss()} and
 * owned by the {@link GebHead} core. It slides vertically to track the hero
 * ({@code state == 2} bobs its pixel row up or down) and, when aligned, performs
 * an area slam over a 4x6 box that also shakes the camera. It starts with a
 * two-tick attack delay that it drops once it has idled long enough
 * ({@link #ticksSinceHit} exceeds 100), then re-arms after each landed slam. It
 * paints 80 pixels high (tall sprite) and is always drawn facing down.
 */
public final class GebHandLeft extends Boss {
    /* renamed from: v */
    /** Ticks since the last landed slam; once past 100 the attack delay is removed. */
    private byte ticksSinceHit;

    public GebHandLeft(GameMap map, byte tileX, byte tileY, byte kind, byte statRow) {
        super(tileX, (byte) (tileY + 5), kind, statRow, (byte) 2);
        map.occupancy[tileY + 5][tileX] = null;
        map.occupancy[tileY + 5][tileX + 1] = null;
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
        updateAi();
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
        this.moveDir = (byte) 1;
        super.paint(graphics, originX, originY - 80);
        this.moveDir = savedMoveDir;
    }

    /* renamed from: i */
    @Override // defpackage.al
    public final void tryAttack() {
        Hero hero = defpackage.GameState.hero();
        int heroRowOffset = ((Entity) hero).tileY - ((((Entity) this).tileY - 5) + 3);
        if (this.hurtCooldown == 0 && heroRowOffset >= -2 && heroRowOffset <= 3 && ((Entity) hero).tileX <= ((Entity) this).tileX + 5) {
            beginAttack();
            return;
        }
        if (this.attackCooldown == 0) {
            if (heroRowOffset > 3) {
                setState((byte) 2);
                setFacing((byte) 2);
            } else if (heroRowOffset < -2) {
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
        if (this.animFrame == 6) {
            defpackage.GameState.map.cameraShiftX = 2;
            defpackage.GameState.map.cameraShiftY = 3;
        } else if (this.animFrame == 7) {
            defpackage.GameState.map.cameraShiftX = -3;
            defpackage.GameState.map.cameraShiftY = -1;
        } else if (this.animFrame == 8) {
            defpackage.GameState.map.cameraShiftX = 2;
            defpackage.GameState.map.cameraShiftY = -3;
        }
        if (this.animFrame == 5) {
            byte slamMinX = (byte) (((Entity) this).tileX + 2);
            byte slamMaxX = (byte) (((Entity) this).tileX + 5);
            byte slamMinY = (byte) ((((Entity) this).tileY - 5) + 1);
            byte slamMaxY = (byte) ((((Entity) this).tileY - 5) + 6);
            if (((Entity) hero).tileX < slamMinX || ((Entity) hero).tileX > slamMaxX || ((Entity) hero).tileY < slamMinY || ((Entity) hero).tileY > slamMaxY) {
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
