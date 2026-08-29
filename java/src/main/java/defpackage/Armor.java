package defpackage;

/* renamed from: t */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:t.class */
/**
 * {@link Equipment} that carries a combat {@link #attribute} (an elemental/proc
 * index, or -1 for none). Both weapons and armor share this attribute: on a hit
 * it may proc, with {@link #PROC_CHANCE} giving the per-attribute chance and
 * {@link #PROC_STATUS} the status effect it inflicts (see {@code Hero.rollProc}).
 * {@link #attributeNames} supplies the attribute's display text.
 */
public class Armor extends Equipment implements Directions {
    /* renamed from: a */
    /** Display names for each combat {@link #attribute}. */
    public static TextTable attributeNames;

    /* renamed from: c */
    /** Combat attribute / proc index (-1 = none). */
    public byte attribute;

    /* renamed from: h */
    /** Status effect inflicted by each {@link #attribute} (-1 = none). */
    public static final byte[] PROC_STATUS = {0, 1, -1, -1, -1, 4, 3, 2, -1};

    /* renamed from: i */
    /** Proc chance (percent) for each {@link #attribute}. */
    public static final byte[] PROC_CHANCE = {20, 16, 6, 13, 13, 10, 10, 10, 10};

    public Armor(byte type, byte subId) {
        super(type, subId);
    }

    @Override // defpackage.e, defpackage.ad
    public int parseRecord(boolean rollEnchants, byte[] data, int offset) {
        int afterEquip = super.parseRecord(rollEnchants, data, offset);
        int end = afterEquip + 1;
        this.attribute = data[afterEquip];
        return end;
    }

    @Override // defpackage.e, defpackage.ad
    public final byte[] serialize() {
        byte[] out = super.serialize();
        out[9] = this.attribute;
        return out;
    }
}
