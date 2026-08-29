package defpackage;

import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/* renamed from: aw */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:aw.class */
/**
 * A short-lived visual "floater" overlaid on a {@link Battler}: damage/heal
 * numbers, hit sparks, level-up and pickup icons, status flashes, etc. Extends
 * {@link Overlay}; {@link #kind} selects both which sprite source to bind in
 * {@link #loadSprites()} and how {@link #paint} renders and offsets it each
 * frame. The inherited {@code frame} counter advances in {@link #paint}, and the
 * overlay finishes once it reaches its lifetime (a lifetime of {@code -1} loops
 * forever). {@link #value} doubles as the number to draw (kind 7) or a sprite
 * index (kinds 9/10).
 */
public final class Floater extends Overlay {
    /** Default lifetime in frames per {@link #kind} ({@code -1} = none/looping). */
    public static final byte[] DEFAULT_LIFETIME = {-1, 3, 4, 11, 9, 3, 3, -1, 8, -1, -1};

    /* renamed from: a */
    /** Sprite frames for image-based kinds (bound from {@link AssetCache}). */
    private Image[] frames;

    /* renamed from: i */
    /** Frame-group script for kinds 4/9 (drawn via {@link GameScreen#drawFrameGroup}). */
    private byte[] spriteScript;

    /* renamed from: a */
    /** Which floater kind this is (selects sprite source and paint behaviour). */
    private byte kind;

    /* renamed from: c */
    /** Number to display (kind 7) or sprite index (kinds 9/10). */
    private short value;

    public Floater(byte kind) {
        this(kind, DEFAULT_LIFETIME[kind], (short) 0);
    }

    public Floater(byte kind, short lifetime, short value) {
        super(lifetime);
        this.kind = kind;
        this.value = value;
        loadSprites();
    }

    /* renamed from: a */
    /** Binds the sprite frames / script for {@link #kind} from {@link AssetCache}. */
    private final void loadSprites() {
        switch (this.kind) {
            case 1:
                this.frames = AssetCache.attackFx1;
                break;
            case 4:
                this.spriteScript = AssetCache.levelUpScript;
                break;
            case 5:
                this.frames = AssetCache.attackFx2;
                break;
            case 6:
                this.frames = AssetCache.attackFx3;
                break;
            case 9:
                this.spriteScript = (byte[]) AssetCache.attackEffectScripts[this.value];
                ((Overlay) this).lifetime = this.spriteScript[0];
                break;
            case 10:
                this.frames = AssetCache.emoticons;
                break;
        }
    }

    @Override // defpackage.f
    public final void paint(Graphics graphics, int x, int y) {
        switch (this.kind) {
            case 1:
                if (this.frame == 0) {
                    y -= 10;
                    x -= 3;
                } else if (this.frame == 1) {
                    y -= 8;
                }
                graphics.drawImage(this.frames[this.frame], x, y + 3, 33);
                break;
            case 2:
                graphics.drawImage(AssetCache.floaterIcon2, x, (y - 30) - (this.frame * 4), 17);
                break;
            case 3:
                if (this.frame % 4 < 3) {
                    graphics.drawImage(AssetCache.floaterIcon3, x, y + 5, 33);
                }
                break;
            case 4:
            case 9:
                GameScreen.drawFrameGroup(graphics, this.spriteScript, (byte) this.frame, x, y);
                break;
            case 5:
                if (this.frame == 2) {
                    y -= 5;
                }
                graphics.drawImage(this.frames[this.frame], x, y + 3, 33);
                break;
            case 6:
                if (this.frame == 1) {
                    y -= 2;
                } else if (this.frame == 2) {
                    y -= 6;
                }
                graphics.drawImage(this.frames[this.frame], x, y + 3, 33);
                break;
            case 7:
                BaseCanvas.drawNumber(graphics, this.value < 0 ? -this.value : this.value, x + 1, (y - 30) - (this.frame * 4), 1, this.frame < 2 ? this.value < 0 ? 4 : 3 : this.value < 0 ? 2 : 1);
                break;
            case 8:
            default:
                graphics.drawImage(this.frames[this.frame], x, y + 3, 33);
                break;
            case 10:
                if (this.value != 8 && this.value != 9) {
                    graphics.drawImage(AssetCache.emoticonBubble, x, y - 40, 17);
                }
                graphics.drawImage(this.frames[this.value], x, (y - 39) + (this.frame % 2), 17);
                break;
        }
        this.frame = (short) (this.frame + 1);
        if (this.frame < ((Overlay) this).lifetime || ((Overlay) this).lifetime == -1) {
            return;
        }
        ((Overlay) this).finished = true;
    }
}
