package defpackage;

import javax.microedition.lcdui.Image;

/* renamed from: bc */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ScrollCaption.class */
/**
 * One line of the end-credits staff roll: a pre-rendered caption {@link #image}
 * and its current vertical position {@link #y}. {@link GameScreen} spawns these
 * into its {@code creditCaptions} vector as the ending text advances, scrolls
 * each up two pixels per frame, and drops it once it leaves the top of the
 * screen.
 */
public final class ScrollCaption {
    /* renamed from: a */
    /** Pre-rendered caption image for this credits line. */
    public Image image;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Current y position (screen pixels); decremented as the roll scrolls up. */
    public int y;

    public ScrollCaption(Image image, int y) {
        this.image = image;
        this.y = y;
    }
}
