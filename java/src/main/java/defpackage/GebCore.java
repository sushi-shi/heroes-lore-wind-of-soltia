package defpackage;

/* renamed from: bv */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bv.class */
/**
 * Final phase of the three-part <b>Geb</b> encounter: the "Geb (Hack)" core
 * (enemy-data record 42, spawned by {@link GameMap#spawnGebCore()} once the head
 * and both hands are down). It is a passive weak-point — a heavily armoured,
 * high-HP body with no attack of its own that the hero must whittle down.
 * Destroying it fires story trigger 1, which ends the encounter.
 */
public final class GebCore extends Boss {
    public GebCore(GameMap map, byte tileX, byte tileY, byte kind, byte statRow) {
        super(tileX, tileY, kind, statRow, (byte) 1);
    }

    /* renamed from: i */
    @Override // defpackage.al
    /** The core is a passive weak-point: it never initiates an attack. */
    public final void tryAttack() {
    }

    /* renamed from: l */
    @Override // defpackage.Boss, defpackage.al
    public final void die() {
        super.die();
        EventScript.fire((byte) 1);
    }

    /* renamed from: m */
    @Override // defpackage.Boss
    public final void onDeath() {
        this.deathTimer = (byte) 16;
    }
}
