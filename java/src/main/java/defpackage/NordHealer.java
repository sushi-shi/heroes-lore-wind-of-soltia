package defpackage;

/* renamed from: cd */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:cd.class */
/**
 * Support part of the three-part phase-2 <b>Nord</b> encounter (enemy-data
 * record 38, attack 1 — it barely hits, it heals). Spawned together with the
 * {@link NordBody2} core and the {@link NordTentacle} striker by
 * {@link GameMap#spawnNordBoss(boolean)}, and linked to them via
 * {@link #setParts(NordBody2, NordTentacle)}. On each cast it first triage-heals
 * whichever part has dropped below half HP (core, then striker, then itself),
 * otherwise it round-robins a smaller top-up across the three. Every heal restores
 * one tenth of the <i>core's</i> maximum HP.
 */
public final class NordHealer extends Boss {
    /* renamed from: a */
    /** The {@link NordBody2} core this part keeps alive. */
    private NordBody2 body;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /* renamed from: f187a */
    /** The {@link NordTentacle} striker this part keeps alive. */
    private NordTentacle striker;

    /* renamed from: v */
    /** Round-robin cursor (0=core, 1=striker, 2=self) for the top-up heal. */
    private byte healRotation;

    public NordHealer(byte tileX, byte tileY, byte kind, byte statRow) {
        super(tileX, tileY, kind, statRow, (byte) 2);
        this.healRotation = (byte) 0;
    }

    /* renamed from: a */
    /** Links this healer to the core and striker parts spawned alongside it. */
    public final void setParts(NordBody2 body, NordTentacle striker) {
        this.body = body;
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
        if (this.animFrame == 5) {
            // Emergency triage: heal any part that has fallen below half HP.
            if (this.body.state != 6 && this.body.state != 5 && ((Enemy) this.body).hp < ((Enemy) this.body).stats.maxHp / 2) {
                healTarget((Enemy) this.body);
            }
            if (this.striker.state != 6 && this.striker.state != 5 && ((Enemy) this.striker).hp < ((Enemy) this.striker).stats.maxHp / 2) {
                healTarget((Enemy) this.striker);
                return;
            }
            if (this.state != 6 && this.state != 5 && ((Enemy) this).hp < ((Enemy) this).stats.maxHp / 2) {
                healTarget((Enemy) this);
                return;
            }
            // Otherwise a routine round-robin top-up across core/striker/self.
            switch (this.healRotation) {
                case 0:
                    if (((Enemy) this.body).hp < ((Enemy) this.body).stats.maxHp) {
                        healTarget((Enemy) this.body);
                    }
                    this.healRotation = (byte) 1;
                    break;
                case 1:
                    if (((Enemy) this.striker).hp < ((Enemy) this.striker).stats.maxHp) {
                        healTarget((Enemy) this.striker);
                    }
                    this.healRotation = (byte) 2;
                    break;
                case 2:
                    if (((Enemy) this).hp < ((Enemy) this).stats.maxHp) {
                        healTarget((Enemy) this);
                    }
                    this.healRotation = (byte) 0;
                    break;
            }
        }
    }

    /* renamed from: a */
    /**
     * Heals one part by a tenth of the core's max HP, showing a green heal
     * floater. The heal amount is always keyed to the core, not to {@code target}.
     */
    private void healTarget(Enemy target) {
        target.addFloater(new Floater((byte) 9, (short) -1, this.statRow));
        target.heal(((Enemy) this.body).stats.maxHp / 10);
        target.addFloater(new Floater((byte) 7, (short) 4, (short) (-(((Enemy) this.body).stats.maxHp / 10))));
    }

    /* renamed from: m */
    @Override // defpackage.Boss
    public final void onDeath() {
        this.deathTimer = (byte) 0;
    }
}
