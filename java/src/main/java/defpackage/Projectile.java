package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: i */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:i.class */
/**
 * A moving {@link Effect} that travels tile-by-tile and deals damage. Each
 * frame {@link #onFrame()} chains a fresh segment one tile forward (decrementing
 * {@link #range}) and, on landing, resolves a hit by owner polarity: an
 * enemy-owned bolt strikes the hero, a hero-owned bolt strikes an enemy. The
 * two constructors distinguish enemy-fired (plain) from hero-fired shots (which
 * carry {@link #damage}, an inflicted {@link #statusKind}, and a {@link #crit}
 * flag).
 */
public final class Projectile extends Effect {
    /* renamed from: a */
    /** The battler that fired this projectile. */
    private Battler owner;

    /* renamed from: d */
    /** Piercing: keeps travelling and hitting after the first target. */
    private boolean piercing;

    /* renamed from: f */
    /** Travel direction (1..4). */
    private byte dir;

    /* renamed from: g */
    /** Tiles of range remaining (each chained segment decrements it). */
    private byte range;

    /* renamed from: h */
    /** Frame at which the projectile chains its next segment. */
    private byte chainFrame;

    /* renamed from: e */
    /** True once this segment has already applied damage. */
    private boolean hasHit;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Damage payload (hero-fired shots). */
    private int damage;

    /* renamed from: i */
    /** Status effect inflicted on hit (hero-fired shots). */
    private byte statusKind;

    /* JADX INFO: renamed from: f, reason: collision with other field name */
    /** Whether this is a critical hit (hero-fired shots). */
    private boolean crit;

    public Projectile(byte tileX, byte tileY, byte[] spriteScript, Battler owner, byte dir, byte range, byte chainFrame) {
        super(tileX, tileY, spriteScript);
        this.owner = owner;
        this.piercing = false;
        this.dir = dir;
        this.range = (byte) (range - 1);
        this.chainFrame = chainFrame;
    }

    public Projectile(byte tileX, byte tileY, byte[] spriteScript, Battler owner, boolean piercing, byte dir, byte range, byte chainFrame, int damage, byte statusKind, boolean crit) {
        super(tileX, tileY, spriteScript);
        this.owner = owner;
        this.piercing = piercing;
        this.dir = dir;
        this.range = (byte) (range - 1);
        this.chainFrame = chainFrame;
        this.damage = damage;
        this.statusKind = statusKind;
        this.crit = crit;
    }

    @Override // defpackage.y
    public final void onFrame() {
        if (((Effect) this).frame == this.chainFrame && this.range > 0 && (this.piercing || !this.hasHit)) {
            GameMap map = GameState.map;
            byte nextX = (byte) (((Entity) this).tileX + Directions.dirDx[this.dir]);
            byte nextY = (byte) (((Entity) this).tileY + Directions.dirDy[this.dir]);
            if (nextX >= 0 && nextX < map.widthTiles && nextY >= 0 && nextY < map.heightTiles) {
                if (this.owner instanceof Enemy) {
                    map.addEntity(new Projectile(nextX, nextY, super.spriteScript, this.owner, this.dir, this.range, this.chainFrame));
                } else if (this.owner instanceof Hero) {
                    map.addEntity(new Projectile(nextX, nextY, super.spriteScript, this.owner, this.piercing, this.dir, this.range, this.chainFrame, this.damage, this.statusKind, this.crit));
                }
            }
        }
        if ((this.piercing || !this.hasHit) && ((Effect) this).frame == 1) {
            Entity cell = GameState.map.occupancy[((Entity) this).tileY][((Entity) this).tileX];
            if (this.owner instanceof Enemy) {
                if (cell == null || !(cell instanceof Hero)) {
                    return;
                }
                ((Hero) cell).takeHit((Enemy) this.owner, this.dir);
                this.hasHit = true;
                return;
            }
            if ((this.owner instanceof Hero) && cell != null && (cell instanceof Enemy)) {
                ((Enemy) cell).takeHeroHit(this.damage, false, this.dir, this.crit, (byte) 1, this.statusKind, (Hero) this.owner);
                this.hasHit = true;
            }
        }
    }

    @Override // defpackage.y
    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    public final boolean isFinished() {
        if (((Effect) this).frame != ((Effect) this).frameCount) {
            return ((Effect) this).frameCount == 2 && !this.piercing && this.hasHit && ((Effect) this).frame >= 1;
        }
        return true;
    }

    @Override // defpackage.y, defpackage.ck
    public final void paint(Graphics graphics, int originX, int originY) {
        int screenX = originX + ((Entity) this).pixelX + ((Entity) this).halfW;
        int screenY = originY + ((Entity) this).pixelY + ((Entity) this).halfH;
        if (((Effect) this).frameCount == 2 && ((Effect) this).frame == 1) {
            screenX += Directions.dirDx[this.dir] * 8;
            screenY += Directions.dirDy[this.dir] * 8;
        }
        drawSprite(graphics, screenX, screenY);
        ((Effect) this).frame = (short) (((Effect) this).frame + 1);
    }
}
