package defpackage;

import java.util.Random;
import javax.microedition.lcdui.Graphics;

/* renamed from: ck */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ck.class */
/**
 * Root of the on-map object hierarchy (Battler/Hero/Enemy, Effect/Projectile,
 * MapObject, Guardian all extend this). Holds a pixel position, the derived
 * tile coordinate, a collision half-size, a draw layer, and the intrusive
 * doubly-linked list pointers used by {@link EntityList}. Tiles are 16 pixels
 * (pixel {@code >> 4} yields the tile index).
 */
public abstract class Entity implements Directions {
    /** Tile column ({@code pixelX >> 4}). */
    public byte tileX;
    /** Tile row ({@code pixelY >> 4}). */
    public byte tileY;

    /* renamed from: a */
    /** True while the entity is mid-tile horizontally (low 4 pixel bits set). */
    public boolean offGridX;

    /* renamed from: b */
    /** True while the entity is mid-tile vertically (low 4 pixel bits set). */
    public boolean offGridY;

    /** Pixel X of the entity's reference point. */
    public short pixelX;
    /** Pixel Y of the entity's reference point. */
    public short pixelY;

    /* renamed from: c */
    /** Collision box half-width, in pixels. */
    public byte halfW;

    /* renamed from: d */
    /** Collision box half-height, in pixels. */
    public byte halfH;

    /* renamed from: a */
    /** Shared PRNG for all entities. */
    public static Random rng = new Random();

    /* renamed from: a */
    /** Next node in the owning {@link EntityList}. */
    public Entity next;

    /* renamed from: b */
    /** Previous node in the owning {@link EntityList}. */
    public Entity prev;

    /** Draw/sort layer (defaults to 1). */
    public byte layer = 1;

    /* renamed from: c */
    /** Set once the entity has been unlinked from its list. */
    public boolean removed = false;

    public Entity(short pixelX, short pixelY, byte halfWidth, byte halfHeight) {
        setPixelPos(pixelX, pixelY);
        syncTile();
        this.halfW = halfWidth;
        this.halfH = halfHeight;
    }

    /** Moves the entity to an absolute pixel position (tile is not re-derived). */
    public void setPixelPos(short pixelX, short pixelY) {
        this.pixelX = pixelX;
        this.pixelY = pixelY;
    }

    /** Recomputes {@link #tileX}/{@link #tileY} and the off-grid flags from pixels. */
    public final void syncTile() {
        this.tileY = (byte) (this.pixelY >> 4);
        this.tileX = (byte) (this.pixelX >> 4);
        this.offGridY = (this.pixelY & 15) != 0;
        this.offGridX = (this.pixelX & 15) != 0;
    }

    /**
     * Returns the entity occupying the tile {@code distance} steps from this one
     * in {@code direction} (1=up, 2=down, 3=left, 4=right), or {@code null} when
     * off-map or empty.
     */
    public final Entity neighbor(byte direction, byte distance) {
        GameMap map = GameState.map;
        switch (direction) {
            case 1:
                if (this.tileY - distance < 0) {
                    return null;
                }
                return map.occupancy[this.tileY - distance][this.tileX];
            case 2:
                if (this.tileY + distance >= map.heightTiles) {
                    return null;
                }
                return map.occupancy[this.tileY + distance][this.tileX];
            case 3:
                if (this.tileX - distance < 0) {
                    return null;
                }
                return map.occupancy[this.tileY][this.tileX - distance];
            case 4:
                if (this.tileX + distance >= map.widthTiles) {
                    return null;
                }
                return map.occupancy[this.tileY][this.tileX + distance];
            default:
                return null;
        }
    }

    /** Draws the entity at screen origin ({@code screenX}, {@code screenY}). */
    public abstract void paint(Graphics graphics, int screenX, int screenY);
}
