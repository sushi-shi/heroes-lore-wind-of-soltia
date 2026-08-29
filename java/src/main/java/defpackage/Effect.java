package defpackage;

import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/* renamed from: y */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:y.class */
/**
 * A transient animated visual effect entity (hit sparks, cast flashes, summon
 * puffs, explosions). It advances one frame each time it is painted and removes
 * itself from the map once {@link #isFinished()} is reached. The effect
 * {@link #type} selects both the lifetime (from {@link #FRAME_COUNTS}) and the
 * draw routine. {@link #onFrame()} is an overridable per-frame hook (base
 * no-op; {@link Projectile} overrides it to travel and deal damage).
 */
public class Effect extends Entity {
    /* renamed from: a */
    /** Lifetime (frame count) per built-in effect type. */
    private static final short[] FRAME_COUNTS = {-1, 4, 8, 6, 10, 11, 7, 9, 6, 4, 3};

    /* renamed from: i */
    /** Sprite-script index (into {@link AssetCache#d}) per type; -1 = none. */
    private static final byte[] TYPE_SPRITE_INDEX = {-1, 0, -1, -1, 0, 0, 1, 0, 1, 1, -1};

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Number of animation frames before the effect finishes. */
    public short frameCount;

    /* renamed from: b */
    /** Current animation frame. */
    public short frame;

    /* renamed from: f */
    /** Effect variant, selecting lifetime and draw routine. */
    private byte type;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Guardian cast/summon image bank ({@link AssetCache#f191a}[12]). */
    private Image[] frameBank;

    /* renamed from: h */
    /** Sprite-cell script blitted for simple frame-based effects. */
    public byte[] spriteScript;

    public Effect(short pixelX, short pixelY, byte type) {
        super(pixelX, pixelY, (byte) 8, (byte) 9);
        this.frameCount = FRAME_COUNTS[type];
        this.frame = (short) 0;
        this.type = type;
        this.frameBank = AssetCache.spriteBanks[12];
        if (TYPE_SPRITE_INDEX[type] != -1) {
            this.spriteScript = (byte[]) AssetCache.guardianFrames[TYPE_SPRITE_INDEX[type]];
        }
    }

    public Effect(byte tileX, byte tileY, byte[] spriteScript) {
        super((short) (tileX << 4), (short) (tileY << 4), (byte) 8, (byte) 9);
        this.frameCount = spriteScript[0];
        this.frame = (short) 0;
        this.type = (byte) 100;
        this.spriteScript = spriteScript;
    }

    /* renamed from: a */
    /** Per-frame hook (base does nothing; overridden by {@link Projectile}). */
    public void onFrame() {
    }

    /* JADX INFO: renamed from: a */
    /** Whether the effect's animation has run its course. */
    public boolean isFinished() {
        return this.frame >= this.frameCount;
    }

    @Override // defpackage.ck
    public void paint(Graphics graphics, int originX, int originY) {
        GameState.map.unlinkEntity(this);
        int screenX = originX + ((Entity) this).pixelX + ((Entity) this).halfW;
        int screenY = originY + ((Entity) this).pixelY + ((Entity) this).halfH;
        switch (this.type) {
            case 1:
            case 6:
            case 8:
            case 9:
            case 100:
                drawSprite(graphics, screenX, screenY);
                break;
            case 2:
                drawHitSpark(graphics, screenX, screenY);
                break;
            case 4:
                drawRisingCast(graphics, screenX, screenY, FRAME_COUNTS[4], this.frameBank[8]);
                break;
            case 5:
                drawRisingCast(graphics, screenX, screenY, FRAME_COUNTS[5], this.frameBank[11]);
                break;
            case 7:
                drawRisingCast(graphics, screenX, screenY, FRAME_COUNTS[7], this.frameBank[11]);
                break;
            case 10:
                drawSummonPuff(graphics, screenX, screenY);
                break;
        }
        this.frame = (short) (this.frame + 1);
        if (isFinished()) {
            GameState.map.removeEntity(this);
        }
    }

    /* renamed from: c */
    /** Draws the hit-spark frames from the shared spark bank. */
    private void drawHitSpark(Graphics graphics, int x, int y) {
        if (this.frame < 0 || this.frame >= this.frameCount) {
            return;
        }
        GameScreen.drawFrameGroup(graphics, AssetCache.guardianSpriteScript, (byte) this.frame, x, y);
    }

    /* renamed from: a */
    /** Draws a cast effect that rises for the first frames then blits its sprite script. */
    private void drawRisingCast(Graphics graphics, int x, int y, int riseFrames, Image image) {
        if (this.frame >= riseFrames) {
        }
        int height = image.getHeight();
        switch (this.frame) {
            case 0:
                GameScreen.clipToWorld(graphics, x - 20, y - 50, 40, 50);
                graphics.drawImage(image, x, y + ((height * 7) / 10), 33);
                graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                break;
            case 1:
                GameScreen.clipToWorld(graphics, x - 20, y - 50, 40, 50);
                graphics.drawImage(image, x, y + ((height * 5) / 10), 33);
                graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                break;
            case 2:
                GameScreen.clipToWorld(graphics, x - 20, y - 50, 40, 50);
                graphics.drawImage(image, x, y + ((height * 3) / 10), 33);
                graphics.setClip(0, 0, GameScreen.width, GameScreen.worldHeight);
                break;
            default:
                GameScreen.drawFrameGroup(graphics, this.spriteScript, (byte) (this.frame - 3), x, y);
                break;
        }
    }

    /* renamed from: b */
    /** Blits the current frame of the effect's sprite script. */
    public final void drawSprite(Graphics graphics, int x, int y) {
        if (this.frame < 0 || this.frame >= this.frameCount) {
            return;
        }
        GameScreen.drawFrameGroup(graphics, this.spriteScript, (byte) this.frame, x, y);
    }

    /* renamed from: d */
    /** Draws the two-part guardian summon puff. */
    private void drawSummonPuff(Graphics graphics, int x, int y) {
        switch (this.frame) {
            case 0:
                graphics.drawImage(this.frameBank[0], x, y, 33);
                break;
            case 1:
                graphics.drawImage(this.frameBank[0], x, y, 33);
                graphics.drawImage(this.frameBank[1], x, y + 3, 33);
                break;
            case 2:
                graphics.drawImage(this.frameBank[1], x, y + 3, 33);
                break;
        }
    }
}
