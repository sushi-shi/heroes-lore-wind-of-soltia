package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: ac */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ac.class */
/**
 * A town / quest NPC actor (a {@link Battler} that never fights). Kinds &lt; 18
 * are animated character sprites (walk/talk frame banks); kinds &gt;= 18 draw a
 * single static object image. NPCs move free-form in their facing direction and
 * never auto-path, and only register in the occupancy grid while idle.
 */
public final class Npc extends Battler {
    /* renamed from: f */
    /** NPC kind; &gt;= 18 selects a static object image instead of a character. */
    public byte kind;

    /* renamed from: g */
    /** Sprite/animation-bank index for animated NPCs. */
    public byte spriteSet;

    /* renamed from: d */
    /** Whether this NPC is currently drawn. */
    public boolean visible;

    public Npc(short pixelX, short pixelY, byte kind, byte spriteSet) {
        super(pixelX, pixelY, (byte) 8, (byte) 8);
        this.kind = kind;
        this.visible = true;
        this.spriteSet = spriteSet;
    }

    @Override // defpackage.ck
    public final void setPixelPos(short pixelX, short pixelY) {
        clearOccupancy();
        super.setPixelPos(pixelX, pixelY);
        syncTile();
        if (this.state == 1) {
            setOccupancy();
        }
    }

    /* renamed from: a */
    @Override // defpackage.o
    public final void move(int stepPixels) {
        clearOccupancy();
        ((Entity) this).pixelX = (short) (((Entity) this).pixelX + (stepPixels * Directions.dirDx[this.facing]));
        ((Entity) this).pixelY = (short) (((Entity) this).pixelY + (stepPixels * Directions.dirDy[this.facing]));
        syncTile();
        if (this.state == 1) {
            setOccupancy();
        }
    }

    /* JADX INFO: renamed from: a */
    @Override // defpackage.o
    public final boolean tryStepForward() {
        return (((Entity) this).offGridX || ((Entity) this).offGridY) ? false : false;
    }

    @Override // defpackage.ck
    public final void paint(Graphics graphics, int originX, int originY) {
        if (this.visible) {
            int screenX = originX + ((Entity) this).pixelX + ((Entity) this).halfW;
            int screenY = originY + ((Entity) this).pixelY + ((Entity) this).halfH;
            if (screenX + 16 < 0 || screenY < 0 || screenX - 16 > GameScreen.width || screenY > GameScreen.worldHeight + 32) {
                return;
            }
            graphics.drawImage(AssetCache.entityShadow, screenX, screenY - 3, 17);
            if (this.kind >= 18) {
                graphics.drawImage(AssetCache.mapNpcImages[this.kind - 18], screenX, screenY, 33);
            } else {
                GameScreen.drawFrameGroup(graphics, (byte[]) AssetCache.npcFrames[this.state == 2 ? (this.spriteSet * 12) + 4 + (this.moveDir - 1) : (this.spriteSet * 12) + 0 + (this.moveDir - 1)], this.animFrame, screenX, screenY);
                this.animFrame = (byte) (this.animFrame + 1);
                if (this.state == 1 && AssetCache.npcAnimFrames0[this.spriteSet] <= this.animFrame) {
                    this.animFrame = (byte) 0;
                } else if (this.state == 2 && AssetCache.npcAnimFrames1[this.spriteSet] <= this.animFrame) {
                    this.animFrame = (byte) 0;
                }
            }
            drawFloaters(graphics, screenX, screenY);
        }
    }
}
