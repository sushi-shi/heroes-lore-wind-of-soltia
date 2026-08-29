package defpackage;

/* renamed from: cg */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:cg.class */
/**
 * Main body ("head") of the three-part <b>Geb</b> encounter (enemy-data record
 * 39, size 2), constructed by {@link GameMap#spawnGebBoss()} together with its
 * {@link GebHandLeft} and {@link GebHandRight}. Its attack marches its two-cell
 * occupancy footprint down the arena columns 6..9 one row per frame
 * ({@link #crushRow}), shoving and hitting the hero if caught underneath; it
 * pauses ({@link #hurtCooldown}) after every three strikes. When it dies it
 * seals columns 6..9 as impassable ({@link #die()}, guarded by
 * {@link #collisionSealed}) and despawns both hands ({@link #onDeath()}).
 */
public final class GebHead extends Boss {
    /* renamed from: a */
    /** The left hand this head owns (despawned when the head dies). */
    private GebHandLeft leftHand;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /* renamed from: f238a */
    /** The right hand this head owns (despawned when the head dies). */
    private GebHandRight rightHand;

    /* renamed from: g */
    /** Guards the one-shot collision seal in {@link #die()} against re-running. */
    private boolean collisionSealed;

    /* renamed from: v */
    /** Strikes landed in the current burst; a fourth forces a long cooldown. */
    private byte attackBurstCount;

    public GebHead(byte tileX, byte tileY, byte kind, byte statRow, GebHandLeft leftHand, GebHandRight rightHand) {
        super(tileX, tileY, kind, statRow, (byte) 2);
        this.leftHand = leftHand;
        this.rightHand = rightHand;
        this.collisionSealed = false;
        this.attackBurstCount = (byte) 0;
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
        if (this.attackBurstCount >= 3) {
            this.hurtCooldown = (byte) 40;
            this.attackBurstCount = (byte) 0;
        }
        if (this.hurtCooldown == 0) {
            setFacing((byte) 2);
            beginAttack();
            this.attackBurstCount = (byte) (this.attackBurstCount + 1);
        }
    }

    /* renamed from: j */
    @Override // defpackage.al
    public final void resolveAttack() {
        Hero hero = defpackage.GameState.hero();
        GameMap map = defpackage.GameState.map;
        switch (this.animFrame) {
            case 6:
                crushRow(hero, map, ((Entity) this).tileY);
                break;
            case 7:
                crushRow(hero, map, (byte) (((Entity) this).tileY + 1));
                break;
            case 8:
                crushRow(hero, map, (byte) (((Entity) this).tileY + 2));
                break;
            case 9:
                crushRow(hero, map, (byte) (((Entity) this).tileY + 3));
                break;
            case 11:
                crushRow(hero, map, (byte) (((Entity) this).tileY + 4));
                break;
            case 12:
                // Release the last crushed row, then land the tail-end hit if the
                // hero is standing just below the head's footprint.
                for (byte col = 6; col <= 9; col++) {
                    if (map.occupancy[((Entity) this).tileY + 4][col] == this) {
                        map.occupancy[((Entity) this).tileY + 4][col] = null;
                    }
                }
                if (((Entity) hero).tileX >= 6 && ((Entity) hero).tileX <= 9
                        && ((Entity) hero).tileY >= ((Entity) this).tileY + 5
                        && ((Entity) hero).tileY <= ((Entity) this).tileY + 8) {
                    hero.takeHit(this, (short) (((Enemy) this).stats.attack / 2), (byte) 2);
                }
                break;
        }
    }

    /* renamed from: k */
    @Override // defpackage.al
    public final void stepDeathAnim() {
        if (this.animFrame >= ((Enemy) this).stats.dieFrames) {
            this.animFrame = (byte) (((Enemy) this).stats.dieFrames - 1);
        }
    }

    /* renamed from: a */
    /**
     * Advances the head's crushing footprint onto tile-row {@code row} (columns
     * 6..9): shoves and hits the hero if it is caught on that row, vacates the
     * previously occupied row, then claims the new row.
     */
    private void crushRow(Hero hero, GameMap map, byte row) {
        for (byte col = 6; col <= 9; col++) {
            if (map.occupancy[row][col] == hero) {
                hero.slide((byte) 2, (byte) 16);
                hero.takeHit((Enemy) this, (byte) 2);
                break;
            }
        }
        for (byte col = 6; col <= 9; col++) {
            if (map.occupancy[row - 1][col] == this) {
                map.occupancy[row - 1][col] = null;
            }
        }
        for (byte col = 6; col <= 9; col++) {
            Debug.assertTrue(map.occupancy[row][col] != hero);
            map.occupancy[row][col] = this;
        }
        setOccupancy();
    }

    /* renamed from: l */
    @Override // defpackage.Boss, defpackage.al
    public final void die() {
        if (this.collisionSealed) {
            return;
        }
        GameMap map = defpackage.GameState.map;
        for (byte col = 6; col <= 9; col++) {
            for (byte row = ((Entity) this).tileY; row <= ((Entity) this).tileY + 2; row++) {
                map.collisionGrid[row][col] = 1;
            }
        }
        this.collisionSealed = true;
    }

    /* renamed from: m */
    @Override // defpackage.Boss
    public final void onDeath() {
        this.deathTimer = (byte) 12;
        GameMap map = defpackage.GameState.map;
        for (byte col = 6; col <= 9; col++) {
            for (int row = ((Entity) this).tileY + 1; row <= ((Entity) this).tileY + 5; row++) {
                if (map.occupancy[row][col] == this) {
                    map.occupancy[row][col] = null;
                }
            }
        }
        defpackage.GameState.map.removeEntity(this.leftHand);
        defpackage.GameState.map.removeEntity(this.rightHand);
    }
}
