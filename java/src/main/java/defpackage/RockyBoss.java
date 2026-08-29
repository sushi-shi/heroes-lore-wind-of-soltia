package defpackage;

/* renamed from: cc */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:cc.class */
/**
 * The solo <b>Rocky Firebird</b> boss (enemy-data record 32, fire element),
 * spawned by {@link GameMap#spawnRockyBoss()}. It cycles through three attack
 * patterns on a fixed schedule ({@link #patternSequence} =
 * {@code {1,1,2,3,2,3}}): pattern 1 is a melee lunge that first teleports the
 * boss onto an occupiable tile next to the hero, pattern 2 is a four-way
 * projectile volley, and pattern 3 is a short-range double-damage slam. Between
 * patterns it hops to a random walkable tile near the hero. Each schedule step
 * carries its own cooldown ({@link #patternCooldowns}), and defeating it fires
 * story trigger 1.
 */
public final class RockyBoss extends Boss {
    /* renamed from: h */
    /** Ordered schedule of attack-pattern ids the boss rotates through. */
    private static final byte[] patternSequence = {1, 1, 2, 3, 2, 3};

    /* renamed from: i */
    /** Cooldown (in ticks) applied when entering each corresponding schedule step. */
    private static final byte[] patternCooldowns = {2, 2, 12, 24, 12, 24};

    /* renamed from: v */
    /** Cursor into {@link #patternSequence}. */
    private byte patternIndex;

    /* renamed from: w */
    /** Attack pattern currently selected (1 = melee, 2 = volley, 3 = slam). */
    private byte attackPattern;

    /* renamed from: x */
    /** Frame count of the currently selected attack animation. */
    private byte attackFrameCount;

    public RockyBoss(byte tileX, byte tileY, byte kind, byte statRow) {
        super(tileX, tileY, kind, statRow, (byte) 1);
        this.patternIndex = (byte) 0;
        selectAttackPattern(patternSequence[this.patternIndex]);
    }

    /* renamed from: d */
    @Override // defpackage.Boss, defpackage.al, defpackage.o
    public final void update() {
        this.animFrame = (byte) (this.animFrame + 1);
        Hero hero = defpackage.GameState.hero();
        ((Boss) this).heroDistX = tileDistX(hero);
        this.heroDistY = tileDistY(hero);
        updateAi();
        animate();
    }

    /* renamed from: n */
    @Override // defpackage.al
    public final void updateAi() {
        switch (((Battler) this).state) {
            case 1:
                tryAttack();
                break;
            case 2:
                if (this.animFrame >= ((Enemy) this).stats.attackFrames) {
                    enterIdle(false);
                    tryAttack();
                }
                break;
            case 3:
                if (this.animFrame >= this.attackFrameCount) {
                    enterIdle(false);
                    this.patternIndex = (byte) (this.patternIndex + 1);
                    if (this.patternIndex >= patternSequence.length) {
                        this.patternIndex = (byte) 0;
                    }
                    selectAttackPattern(patternSequence[this.patternIndex]);
                    this.hurtCooldown = patternCooldowns[this.patternIndex];
                    this.attackCooldown = patternCooldowns[this.patternIndex];
                    tryAttack();
                }
                break;
            case 4:
                if (this.knockbackTimer < 1) {
                    setState((byte) 1);
                }
                this.knockbackTimer = (byte) (this.knockbackTimer - 1);
                break;
            case 5:
                if (this.deathTimer < 1) {
                    die();
                }
                this.deathTimer = (byte) (this.deathTimer - 1);
                break;
        }
    }

    /* renamed from: i */
    @Override // defpackage.al
    public final void tryAttack() {
        if (this.hurtCooldown == 0) {
            switch (this.attackPattern) {
                case 1:
                    if (((Boss) this).heroDistX < 4 && this.heroDistY < 4) {
                        beginAttack();
                    }
                    break;
                case 2:
                    if (((Boss) this).heroDistX * this.heroDistY == 0 && ((Boss) this).heroDistX < 4 && this.heroDistY < 4) {
                        beginAttack();
                        return;
                    }
                    break;
                case 3:
                    beginAttack();
                    return;
            }
        }
        if (this.attackCooldown == 0) {
            switch (this.attackPattern) {
                case 1:
                    if (((Boss) this).heroDistX >= 4 || this.heroDistY >= 4) {
                        setState((byte) 2);
                        this.animFrame = (byte) 0;
                    }
                    break;
                case 2:
                    if (((Boss) this).heroDistX * this.heroDistY != 0 || ((Boss) this).heroDistX >= 4 || this.heroDistY >= 4) {
                        setState((byte) 2);
                        this.animFrame = (byte) 0;
                    }
                    break;
                case 3:
                    if (((Boss) this).heroDistX >= 3 || this.heroDistY >= 3) {
                        setState((byte) 2);
                        this.animFrame = (byte) 0;
                    }
                    break;
            }
        }
    }

    /* renamed from: o */
    @Override // defpackage.al
    public final void animate() {
        switch (((Battler) this).state) {
            case 2:
                if (this.animFrame == 5) {
                    // Hop onto a random walkable tile adjacent to the hero (up to 3 tries).
                    GameMap map = defpackage.GameState.map;
                    Hero hero = defpackage.GameState.hero();
                    byte hopTileX = -1;
                    byte hopTileY = -1;
                    byte triesLeft = 3;
                    while (triesLeft > 0 && !map.isWalkable(hopTileX, hopTileY)) {
                        hopTileX = ((Entity) hero).tileX > ((Entity) this).tileX
                                ? (byte) defpackage.ByteUtil.randRange(((Entity) this).tileX, ((Entity) hero).tileX + 1)
                                : (byte) defpackage.ByteUtil.randRange(((Entity) hero).tileX - 1, ((Entity) this).tileX);
                        hopTileY = ((Entity) hero).tileY > ((Entity) this).tileY
                                ? (byte) defpackage.ByteUtil.randRange(((Entity) this).tileY, ((Entity) hero).tileY + 1)
                                : (byte) defpackage.ByteUtil.randRange(((Entity) hero).tileY - 1, ((Entity) this).tileY);
                        triesLeft = (byte) (triesLeft - 1);
                    }
                    if (triesLeft > 0) {
                        setPixelPos((short) (hopTileX << 4), (short) (hopTileY << 4));
                    }
                }
                break;
            case 3:
                resolveAttack();
                break;
            case 4:
            default:
                if (this.animFrame >= ((Enemy) this).stats.walkFrames) {
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
    @Override // defpackage.al
    public final void resolveAttack() {
        Hero hero = defpackage.GameState.hero();
        switch (this.attackPattern) {
            case 1:
                if (this.animFrame == 7) {
                    // Reposition onto the first occupiable tile next to the hero.
                    GameMap map = defpackage.GameState.map;
                    for (byte dir = 1; dir <= 4; dir++) {
                        if (map.canOccupy(this, ((Entity) hero).tileX + Directions.dirDx[dir], ((Entity) hero).tileY + Directions.dirDy[dir])) {
                            setPixelPos((short) ((((Entity) hero).tileX + Directions.dirDx[dir]) << 4), (short) ((((Entity) hero).tileY + Directions.dirDy[dir]) << 4));
                            break;
                        }
                    }
                }
                if (this.animFrame == 11 && ((Boss) this).heroDistX + this.heroDistY <= 1) {
                    hero.takeHit((Enemy) this, ((Battler) this).facing);
                }
                break;
            case 2:
                if (this.animFrame == 7) {
                    for (byte dir = 1; dir <= 4; dir++) {
                        defpackage.GameState.map.addEntity(new Projectile((byte) (((Entity) this).tileX + Directions.dirDx[dir]), (byte) (((Entity) this).tileY + Directions.dirDy[dir]), (byte[]) AssetCache.attackEffectScripts[this.statRow], this, dir, (byte) 3, (byte) 2));
                    }
                }
                break;
            case 3:
                if (this.animFrame == 4 && ((Boss) this).heroDistX <= 2 && this.heroDistY <= 2) {
                    hero.takeHit((Enemy) this, (short) (((Enemy) this).stats.attack * 2), ((Battler) this).facing);
                }
                break;
        }
    }

    /* renamed from: k */
    @Override // defpackage.al
    public final void stepDeathAnim() {
        if (this.deathTimer > 8) {
            defpackage.GameState.map.addEntity(new Effect((byte) (((Entity) this).tileX + defpackage.ByteUtil.randRange(-1, 1)), (byte) (((Entity) this).tileY + defpackage.ByteUtil.randRange(0, 3)), (byte[]) AssetCache.attackEffectScripts[this.statRow]));
        }
    }

    /* renamed from: d */
    /**
     * Switches the active attack pattern: copies pattern {@code pattern}'s four
     * attack frame-groups into this boss's attack sprite slots and caches the
     * new animation's frame count.
     */
    private final void selectAttackPattern(byte pattern) {
        this.attackPattern = pattern;
        int sourceOffset = (pattern - 1) * 4;
        for (int i = 0; i < 4; i++) {
            AssetCache.bossFrames[(this.statRow * 16) + 12 + i] = AssetCache.bossExtraFrames[sourceOffset + i];
        }
        this.attackFrameCount = ((byte[]) AssetCache.bossFrames[(this.statRow * 16) + 12])[0];
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
        this.deathTimer = (byte) 24;
    }
}
