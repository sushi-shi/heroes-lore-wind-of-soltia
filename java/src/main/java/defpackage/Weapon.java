package defpackage;

/* renamed from: l */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:l.class */
/**
 * A wieldable {@link Armor} (weapon slot). Adds {@link #accuracy}, which raises
 * the hero's critical-hit chance, and {@link #critBonus}, a tenths-of-damage
 * bonus applied on a critical hit. Its record layout reads the base
 * name/description/price and the equipment stats, then the two weapon bytes, and
 * finally the shared {@link Armor#attribute}, so it overrides the record parser
 * rather than chaining through {@link Armor}.
 */
public final class Weapon extends Armor {
    /* renamed from: a */
    /** Added to the hero's critical-hit chance. */
    public byte accuracy;

    /* renamed from: b */
    /** Critical-hit damage bonus, in tenths of the rolled damage. */
    public byte critBonus;

    public Weapon(byte type, byte subId) {
        super(type, subId);
    }

    @Override // defpackage.t, defpackage.e, defpackage.ad
    public final int parseRecord(boolean rollEnchants, byte[] data, int offset) {
        int afterName = offset + parseName(data, offset);
        int afterDesc = afterName + parseDescription(data, afterName);
        int afterPrice = afterDesc + parsePrice(data, afterDesc);
        int afterEquip = afterPrice + parseEquipStats(data, afterPrice, rollEnchants);
        int p = afterEquip + 1;
        this.accuracy = data[afterEquip];
        int p2 = p + 1;
        this.critBonus = data[p];
        int end = p2 + 1;
        this.attribute = data[p2];
        return end;
    }
}
