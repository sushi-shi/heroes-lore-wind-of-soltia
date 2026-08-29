package defpackage;

/* renamed from: an */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:an.class */
/**
 * Minimal Adler-32 running checksum (RFC 1950), used by {@link PngMerger} to
 * write the zlib check value of the deflate stream it emits. The 32-bit state
 * packs two 16-bit sums {@code (high << 16) | low}; both are reduced modulo
 * {@code 65521} (the largest prime below 2^16) and at most {@code 5552} bytes
 * are folded before each reduction to keep the low sum from overflowing.
 */
public final class Adler32 {
    /** Packed running checksum: {@code (sumB << 16) | sumA}, seeded to 1. */
    private int sum = 1;

    /**
     * Folds {@code length} bytes of {@code data} starting at {@code offset} into
     * the running checksum.
     */
    public final void update(byte[] data, int offset, int length) {
        int sumA = this.sum & 65535;
        int sumB = (this.sum >> 16) & 65535;
        while (true) {
            if (length <= 0) {
                this.sum = (sumB << 16) | sumA;
                return;
            }
            int block = length < 5552 ? length : 5552;
            length -= block;
            while (true) {
                int remaining = block;
                block = remaining - 1;
                if (remaining <= 0) {
                    break;
                }
                int index = offset;
                offset++;
                sumA += data[index] & 255;
                sumB += sumA;
            }
            sumA %= 65521;
            sumB = sumB % 65521;
        }
    }

    /** Resets the checksum to its initial value (1). */
    public final void reset() {
        this.sum = 1;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /**
     * Returns the current checksum. Preserves the original {@code & -1L} mask,
     * which sign-extends rather than zero-extending the 32-bit state.
     */
    public final long getValue() {
        return ((long) this.sum) & (-1);
    }
}
