package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: f */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:f.class */
/**
 * Abstract base of the short-lived on-map visual effects that entities carry in
 * their floater list ({@link Floater} damage/heal numbers, {@link StatusIcon}
 * buff/debuff icons, {@link GuardianCastFx} guardian-cast animations). Each
 * effect advances a {@link #frame} counter every {@link #paint} and marks itself
 * {@link #finished} once it has run for its {@link #lifetime}; the owning entity
 * then drops finished effects from its list.
 */
public abstract class Overlay implements Directions {
    /* renamed from: a */
    /** Number of frames this effect lives for (or {@code -1} for open-ended). */
    public short lifetime;
    /* renamed from: b */
    /** Frames elapsed since the effect started. */
    public short frame = 0;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Set once the effect has finished so the owner can drop it. */
    public boolean finished = false;

    public Overlay(short lifetime) {
        this.lifetime = lifetime;
    }

    public abstract void paint(Graphics graphics, int x, int y);
}
