package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: cf */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:cf.class */
/**
 * A single status-effect icon bobbing above a {@link Battler} (poison, stun,
 * buffs, etc.). Extends {@link Overlay}: the inherited {@code frame} counter
 * advances each {@link #tick()} and, once it reaches the per-kind duration
 * ({@link #DURATION_BY_KIND}), the overlay marks itself finished so the owner
 * reaps it. {@link Battler#applyStatus} reuses an existing icon of the same
 * {@link #kind} by calling {@link #reset()} instead of stacking a new one.
 */
public final class StatusIcon extends Overlay {
    /* renamed from: a */
    /** Status kind this icon represents (indexes {@link AssetCache#f233v}). */
    public byte kind;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Lifetime in frames per status kind. */
    private static final short[] DURATION_BY_KIND = {40, 40, 40, 40, 40, 140, 160, 80};

    public StatusIcon(byte kind) {
        super(DURATION_BY_KIND[kind]);
        this.kind = kind;
    }

    /* renamed from: a */
    /** Advances the animation one frame, finishing when the lifetime elapses. */
    public final void tick() {
        this.frame = (short) (this.frame + 1);
        if (this.frame >= ((Overlay) this).lifetime) {
            ((Overlay) this).finished = true;
        }
    }

    /* renamed from: b */
    /** Ends this icon immediately (e.g. a cleansed status). */
    public final void expire() {
        ((Overlay) this).finished = true;
    }

    @Override // defpackage.f
    public final void paint(Graphics graphics, int x, int y) {
        graphics.drawImage(AssetCache.emoticonBubble, x, y - 30, 17);
        graphics.drawImage(AssetCache.statusIcons[this.kind], x, (y - 29) + (this.frame % 2), 17);
    }

    /* renamed from: c */
    /** Restarts the icon's animation from frame 0 (refreshes a re-applied status). */
    public final void reset() {
        this.frame = (short) 0;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the current frame counter (used for periodic poison ticks). */
    public final short elapsed() {
        return this.frame;
    }
}
