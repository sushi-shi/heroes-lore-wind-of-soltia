package defpackage;

/* renamed from: bd */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bd.class */
/**
 * Striker tentacle of the second-phase <b>Nord</b> encounter (enemy-data record
 * 37, attack 100). One of the three linked parts spawned together by
 * {@link GameMap#spawnNordBoss(boolean)} alongside the {@link NordBody2} core
 * and the {@link NordHealer}. Its attack is telegraphed: at frame 4 it marks the
 * hero's current tile (and plants a warning effect there), and at frame 7 it
 * lands the blow only if the hero is still standing on that marked tile.
 */
public final class NordTentacle extends Boss {
    /* renamed from: v */
    /** Hero tile-column recorded at the telegraph frame; re-checked on the hit frame. */
    private byte markedTileX;

    /* renamed from: w */
    /** Hero tile-row recorded at the telegraph frame; re-checked on the hit frame. */
    private byte markedTileY;

    public NordTentacle(byte tileX, byte tileY, byte kind, byte statRow) {
        super(tileX, tileY, kind, statRow, (byte) 2);
    }

    /* renamed from: d */
    @Override // defpackage.Boss, defpackage.al, defpackage.o
    public final void update() {
        this.animFrame = (byte) (this.animFrame + 1);
        updateAi();
        animate();
    }

    /* renamed from: h */
    @Override // defpackage.al
    public final void chase() {
        if (this.animFrame >= ((Enemy) this).stats.walkFrames) {
            setState((byte) 1);
        }
    }

    /* renamed from: i */
    @Override // defpackage.al
    public final void tryAttack() {
        if (this.hurtCooldown == 0) {
            beginAttack();
        }
    }

    /* renamed from: j */
    @Override // defpackage.al
    public final void resolveAttack() {
        Hero hero = defpackage.GameState.hero();
        if (this.animFrame == 4) {
            spawnEffectAt(((Entity) hero).tileX, ((Entity) hero).tileY);
            this.markedTileX = ((Entity) hero).tileX;
            this.markedTileY = ((Entity) hero).tileY;
        }
        if (this.animFrame == 7
                && this.markedTileX == ((Entity) hero).tileX
                && this.markedTileY == ((Entity) hero).tileY) {
            hero.takeHit((Enemy) this, this.facing);
        }
    }

    /* renamed from: m */
    @Override // defpackage.Boss
    public final void onDeath() {
        this.deathTimer = (byte) 0;
    }
}
