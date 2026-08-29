package defpackage;

import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/* renamed from: bj */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:GuardianCastFx.class */
/**
 * On-map animation for a guardian summon/cast, carried in an entity's floater
 * list as an {@link Overlay}. After an initial {@link #startDelay} it advances
 * the inherited {@link Overlay#frame} counter each paint, choosing one of three
 * looks by ({@link #guardianType}, {@link #skillSlot}):
 * <ul>
 *   <li>both zero — the base summon pose drawn straight from the element atlas
 *       ({@link #elementSprites} slots 7-9);</li>
 *   <li>guardian 0 or 1 with skill 2 — the two-frame descending beam
 *       ({@link #beamFrames}) cycled every four frames;</li>
 *   <li>otherwise — the guardian's pose frame-group script
 *       ({@link #guardianFrames}{@code [skillSlot]}).</li>
 * </ul>
 * The effect marks itself {@link Overlay#finished} once {@link Overlay#frame}
 * reaches its {@link Overlay#lifetime}.
 */
public final class GuardianCastFx extends Overlay {
    /* renamed from: a */
    /** Guardian skill slot; also indexes {@link #guardianFrames}. */
    private byte skillSlot;
    /* renamed from: b */
    /** Guardian type (selects the beam special-case). */
    private byte guardianType;
    /* renamed from: c */
    /** Frames to wait before the animation starts playing. */
    private short startDelay;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** The two descending-beam frames (element atlas slots 0-1). */
    private Image[] beamFrames;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** The guardian element atlas ({@code spriteBanks[12]}); slots 7-9 are the base pose. */
    private Image[] elementSprites;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Guardian pose frame-group scripts (a copy of {@link AssetCache#guardianFrames}). */
    private Object[] guardianFrames;

    public GuardianCastFx(short startDelay, short lifetime, byte guardianType, byte skillSlot) {
        super(lifetime);
        this.startDelay = startDelay;
        this.guardianType = guardianType;
        this.skillSlot = skillSlot;
        this.beamFrames = new Image[2];
        Image[] elementAtlas = AssetCache.spriteBanks[12];
        this.beamFrames[0] = elementAtlas[0];
        this.beamFrames[1] = elementAtlas[1];
        this.elementSprites = AssetCache.spriteBanks[12];
        this.guardianFrames = AssetCache.guardianFrames;
    }

    @Override // defpackage.f
    public final void paint(Graphics graphics, int x, int y) {
        if (this.startDelay > 0) {
            this.startDelay = (short) (this.startDelay - 1);
            return;
        }
        if (this.guardianType != 0 || this.skillSlot != 0) {
            if ((this.guardianType == 0 && this.skillSlot == 2) || (this.guardianType == 1 && this.skillSlot == 2)) {
                switch (((Overlay) this).frame % 4) {
                    case 1:
                        graphics.drawImage(this.beamFrames[0], x, y + 9, 33);
                        break;
                    case 2:
                        graphics.drawImage(this.beamFrames[0], x, y + 9, 33);
                        graphics.drawImage(this.beamFrames[1], x, y + 12, 33);
                        break;
                    case 3:
                        graphics.drawImage(this.beamFrames[1], x, y + 12, 33);
                        break;
                }
            } else {
                GameScreen.drawFrameGroup(graphics, (byte[]) this.guardianFrames[this.skillSlot], (byte) ((Overlay) this).frame, x, y);
            }
        } else {
            switch (((Overlay) this).frame) {
                case 0:
                    graphics.drawImage(this.elementSprites[7], x, y, 33);
                    break;
                case 1:
                    graphics.drawImage(this.elementSprites[8], x, y - 1, 33);
                    break;
                case 2:
                    graphics.drawImage(this.elementSprites[7], x, y - 2, 33);
                    break;
                case 3:
                    graphics.drawImage(this.elementSprites[8], x, y - 3, 33);
                    break;
                case 4:
                case 5:
                    graphics.drawImage(this.elementSprites[9], x, y - 4, 33);
                    break;
            }
        }
        ((Overlay) this).frame = (short) (((Overlay) this).frame + 1);
        if (((Overlay) this).frame >= ((Overlay) this).lifetime) {
            ((Overlay) this).finished = true;
        }
    }
}
