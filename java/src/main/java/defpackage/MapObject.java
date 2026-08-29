package defpackage;

import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/* renamed from: aj */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:aj.class */
/**
 * A static placed-image map decoration/prop (a {@link Entity} with no AI or
 * stats). It holds one {@link #image} and precomputed cull bounds, and its only
 * behaviour is to draw itself at its world position when on screen.
 */
public final class MapObject extends Entity {
    /* renamed from: a */
    /** The prop bitmap. */
    public Image image;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Left cull bound (screen X below which the prop is off-screen). */
    private short minX;

    /* renamed from: b */
    /** Right cull bound. */
    private short maxX;

    /* renamed from: e */
    /** Bottom cull bound. */
    private short maxY;

    public MapObject(short pixelX, short pixelY, byte halfWidth, byte halfHeight, Image image) {
        super(pixelX, pixelY, halfWidth, halfHeight);
        this.image = image;
        if (image != null) {
            this.minX = (short) (-(image.getWidth() >> 1));
            this.maxX = (short) (GameScreen.width + (image.getWidth() >> 1));
            this.maxY = (short) (GameScreen.worldHeight + image.getHeight());
        }
    }

    @Override // defpackage.ck
    public final void paint(Graphics graphics, int originX, int originY) {
        int screenX = originX + ((Entity) this).pixelX + ((Entity) this).halfW;
        int screenY = originY + ((Entity) this).pixelY + ((Entity) this).halfH;
        if (screenX < this.minX || screenX > this.maxX || screenY < 0 || screenY > this.maxY) {
            return;
        }
        graphics.drawImage(this.image, screenX, screenY, 33);
    }
}
