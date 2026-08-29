package defpackage;

/* renamed from: e */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:e.class */
/**
 * An {@link Item} that can be equipped (accessories, and via subclasses armor and
 * weapons). Adds a buy/sell {@link #value}, a {@link #levelReq}, the identify and
 * enchant mechanics: {@link #identified} governs whether magic stats are revealed,
 * {@link #refineLevel} is the upgrade level, and {@link #enchant} holds four rolled
 * enchant values. Unidentified equipment ({@link #needsIdentify}) rolls its enchant
 * values from min/max ranges stored in the item record.
 */
public class Equipment extends Item {
    /* renamed from: a */
    /** Buy/sell value. */
    public short value;

    /* renamed from: d */
    /** Minimum hero level required to equip. */
    public byte levelReq;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Whether this piece must be identified before use (has hidden enchants). */
    public boolean needsIdentify;

    /* renamed from: b */
    /** Whether the enchant values are revealed (identified). */
    public boolean identified;

    /* renamed from: e */
    /** Refine/upgrade level. */
    public byte refineLevel;

    /* renamed from: j */
    /** Four rolled enchant stat values. */
    public byte[] enchant;

    public Equipment(byte type, byte subId) {
        super(type, subId);
        this.enchant = new byte[4];
    }

    @Override // defpackage.ad
    public int parseRecord(boolean rollEnchants, byte[] data, int offset) {
        int afterBase = super.parseRecord(rollEnchants, data, offset);
        return afterBase + parseEquipStats(data, afterBase, rollEnchants);
    }

    /* renamed from: a */
    /**
     * Decodes the equipment-specific stat block (value, level requirement, the
     * identify flag and the enchant min/max ranges). When {@code rollEnchants} and
     * any enchant range is non-zero, rolls the actual enchant values.
     */
    public final int parseEquipStats(byte[] data, int offset, boolean rollEnchants) {
        int p = offset + 1;
        this.value = (short) (data[offset] & 255);
        int p2 = p + 1;
        this.levelReq = data[p];
        int rangeBase = p2 + 1;
        this.needsIdentify = data[p2] != 0;
        boolean hasEnchant = false;
        for (int i = 1; i <= 8; i++) {
            if (data[rangeBase + i] != 0) {
                hasEnchant = true;
                break;
            }
        }
        if (!hasEnchant) {
            this.identified = true;
        }
        if (!rollEnchants || !hasEnchant) {
            return 12;
        }
        rollEnchant(data[rangeBase], new byte[]{data[rangeBase + 1], data[rangeBase + 3], data[rangeBase + 5], data[rangeBase + 7]}, new byte[]{data[rangeBase + 2], data[rangeBase + 4], data[rangeBase + 6], data[rangeBase + 8]}, (byte) 0);
        return 12;
    }

    @Override // defpackage.ad
    public byte[] serialize() {
        byte[] out = super.serialize();
        out[3] = this.identified ? (byte) 1 : (byte) 0;
        out[4] = this.refineLevel;
        System.arraycopy(this.enchant, 0, out, 5, 4);
        return out;
    }

    /* renamed from: a */
    /**
     * Rolls {@code count} enchant values into random empty slots whose min/max
     * range is non-zero, then records the refine level.
     */
    public final void rollEnchant(byte count, byte[] mins, byte[] maxs, byte refine) {
        int slot;
        for (int i = 0; i < count; i++) {
            while (true) {
                slot = defpackage.ByteUtil.randRange(0, 3);
                if (this.enchant[slot] == 0 && (mins[slot] != 0 || maxs[slot] != 0)) {
                    break;
                }
            }
            this.enchant[slot] = (byte) defpackage.ByteUtil.randRange(mins[slot], maxs[slot]);
        }
        this.refineLevel = refine;
    }

    /* renamed from: a */
    /** Sets the four enchant values directly (used when loading a save). */
    public final void setEnchant(byte e0, byte e1, byte e2, byte e3) {
        this.enchant[0] = e0;
        this.enchant[1] = e1;
        this.enchant[2] = e2;
        this.enchant[3] = e3;
    }
}
