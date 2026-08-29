package defpackage;

/* renamed from: ca */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ca.class */
/**
 * Minimal table-driven CRC-32 (the ISO 3309 / zlib variant), used by
 * {@link PngMerger} to compute PNG chunk CRCs and the IDAT check field. Uses
 * the standard reversed polynomial {@code 0xEDB88320}, an initial value of all
 * ones, and a final ones-complement, matching {@code java.util.zip.CRC32}.
 */
public final class Crc32 {
    /** Precomputed byte lookup table (256 entries) for the reversed polynomial. */
    private static final int[] TABLE = new int[256];

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Running CRC, held pre-inverted (seeded to {@code -1}). */
    private int crc = -1;

    /** Resets the running CRC to its initial value. */
    public final void reset() {
        this.crc = -1;
    }

    /**
     * Folds {@code length} bytes of {@code data} starting at {@code offset} into
     * the running CRC.
     */
    public final void update(byte[] data, int offset, int length) {
        for (int i = offset; i < length + offset; i++) {
            this.crc = ((this.crc >>> 8) & 16777215) ^ TABLE[(this.crc ^ data[i]) & 255];
        }
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the finished CRC-32 value (the ones-complement of the state). */
    public final int getValue() {
        return this.crc ^ (-1);
    }

    static {
        for (short n = 0; n < 256; n = (short) (n + 1)) {
            int c = n;
            for (byte bit = 1; bit < 9; bit = (byte) (bit + 1)) {
                c = (c & 1) == 1 ? (c >>> 1) ^ (-306674912) : c >>> 1;
            }
            TABLE[n] = c;
        }
    }
}
