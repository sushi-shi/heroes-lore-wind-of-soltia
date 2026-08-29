package defpackage;

/* renamed from: ag */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ag.class */
/**
 * Core body of the three-part phase-2 <b>Nord</b> encounter (enemy-data record
 * 36), spawned by {@link GameMap#spawnNordBoss(boolean)} with
 * {@code intro == false}. It links to its two companion parts — the
 * {@link NordHealer} and the {@link NordTentacle} striker — via
 * {@link #setParts(NordHealer, NordTentacle)}; killing this core is what ends
 * the whole encounter. Its attack is a five-way (plus/X) projectile volley, and
 * on death it despawns both companion parts and fires story trigger 1.
 */
public final class NordBody2 extends Boss {
    /* renamed from: a */
    /** Support part that tops up this core's (and the striker's) HP. */
    private NordHealer healer;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /* renamed from: f31a */
    /** Companion telegraphed-slam tentacle. */
    private NordTentacle striker;

    public NordBody2(byte tileX, byte tileY, byte kind, byte statRow) {
        super(tileX, tileY, kind, statRow, (byte) 3);
    }

    /* renamed from: a */
    /** Links this core to the healer and striker parts spawned alongside it. */
    public final void setParts(NordHealer healer, NordTentacle striker) {
        this.healer = healer;
        this.striker = striker;
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
        if (this.animFrame >= ((Enemy) this).stats.attackFrames) {
            setState((byte) 1);
        }
    }

    /* renamed from: i */
    @Override // defpackage.al
    public final void tryAttack() {
        if (this.hurtCooldown == 0) {
            setFacing((byte) 2);
            beginAttack();
        }
    }

    /* renamed from: j */
    @Override // defpackage.al
    public final void resolveAttack() {
        if (this.animFrame == 2) {
            defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX - 1), (byte) (((Entity) this).tileY - 1), (byte[]) AssetCache.attackEffectScripts[this.statRow], this, this.facing, (byte) 13, (byte) 2));
            defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX + 3), (byte) (((Entity) this).tileY - 1), (byte[]) AssetCache.attackEffectScripts[this.statRow], this, this.facing, (byte) 13, (byte) 2));
            defpackage.GameState.map.addEntity(new Projectile(((Entity) this).tileX, ((Entity) this).tileY, (byte[]) AssetCache.attackEffectScripts[this.statRow], this, this.facing, (byte) 13, (byte) 2));
            defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX + 2), ((Entity) this).tileY, (byte[]) AssetCache.attackEffectScripts[this.statRow], this, this.facing, (byte) 13, (byte) 2));
            defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX + 1), (byte) (((Entity) this).tileY + 1), (byte[]) AssetCache.attackEffectScripts[this.statRow], this, this.facing, (byte) 13, (byte) 2));
        }
    }

    /* renamed from: k */
    @Override // defpackage.al
    public final void stepDeathAnim() {
        if (this.deathTimer > 8) {
            defpackage.GameState.map.addEntity(new Effect((byte) (((Entity) this).tileX + defpackage.ByteUtil.randRange(-2, 2)), (byte) (((Entity) this).tileY + defpackage.ByteUtil.randRange(-2, 2)), (byte[]) AssetCache.attackEffectScripts[this.healer.statRow]));
            defpackage.GameState.map.addEntity(new Effect((byte) (((Entity) this).tileX + defpackage.ByteUtil.randRange(-2, 2)), (byte) (((Entity) this).tileY + defpackage.ByteUtil.randRange(-2, 2)), (byte[]) AssetCache.attackEffectScripts[this.healer.statRow]));
        }
    }

    /* renamed from: l */
    @Override // defpackage.Boss, defpackage.al
    public final void die() {
        super.die();
        EventScript.fire((byte) 1);
    }

    /* renamed from: m */
    @Override // defpackage.Boss
    public final void onDeath() {
        this.healer.die();
        this.striker.die();
        this.deathTimer = (byte) 24;
    }
}
