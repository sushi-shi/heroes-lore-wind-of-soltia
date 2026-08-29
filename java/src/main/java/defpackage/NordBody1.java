package defpackage;

/* renamed from: ar */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ar.class */
/**
 * First-phase solo form of the <b>Nord</b> encounter (enemy-data record 35),
 * spawned by {@link GameMap#spawnNordBoss(boolean)} with {@code intro == true}.
 * It is a rooted caster: it never walks ({@link #tryStepForward} is forced
 * false), only turns to face the hero and lobs a three-way projectile volley.
 * Defeating it does not end the fight — {@link #die()} immediately spawns the
 * three-part phase-2 Nord ({@link NordBody2} core + {@link NordTentacle} +
 * {@link NordHealer}).
 */
public final class NordBody1 extends Boss {
    public NordBody1(byte tileX, byte tileY, byte kind, byte statRow) {
        super(tileX, tileY, kind, statRow, (byte) 1);
    }

    /* renamed from: a */
    @Override // defpackage.o
    /** Rooted in place: this phase-1 form never advances a step. */
    public final boolean tryStepForward() {
        return false;
    }

    /* renamed from: i */
    @Override // defpackage.al
    public final void tryAttack() {
        Hero hero = defpackage.GameState.hero();
        if (this.hurtCooldown == 0 && ((Boss) this).heroDistX <= 1) {
            setFacing((byte) 2);
            beginAttack();
            return;
        }
        if (this.attackCooldown == 0) {
            if (((Boss) this).heroDistX <= 1) {
                setState((byte) 1);
                setFacing((byte) 2);
            } else if (((Entity) hero).tileX > ((Entity) this).tileX) {
                setState((byte) 2);
                setFacing((byte) 4);
            } else if (((Entity) hero).tileX < ((Entity) this).tileX) {
                setState((byte) 2);
                setFacing((byte) 3);
            }
        }
    }

    /* renamed from: j */
    @Override // defpackage.al
    public final void resolveAttack() {
        if (this.animFrame == 2) {
            defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX - 1), ((Entity) this).tileY, (byte[]) AssetCache.attackEffectScripts[this.statRow], this, this.facing, (byte) 13, (byte) 2));
            defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX + 1), ((Entity) this).tileY, (byte[]) AssetCache.attackEffectScripts[this.statRow], this, this.facing, (byte) 13, (byte) 2));
            defpackage.GameState.map.addEntity(new Projectile(((Entity) this).tileX, (byte) (((Entity) this).tileY + 1), (byte[]) AssetCache.attackEffectScripts[this.statRow], this, this.facing, (byte) 13, (byte) 2));
        }
    }

    /* renamed from: l */
    @Override // defpackage.Boss, defpackage.al
    public final void die() {
        super.die();
        defpackage.GameState.map.spawnNordBoss(false);
    }

    /* renamed from: m */
    @Override // defpackage.Boss
    public final void onDeath() {
        this.deathTimer = (byte) 0;
    }
}
