package defpackage;

import java.util.Random;

/* renamed from: h */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:h.class */
/**
 * Big-endian byte helpers plus the shared RNG used across the engine. All of the
 * binary asset formats (see the Phase-1 spec) are big-endian, and every integer
 * read/written from a save record or asset blob funnels through {@link #readU16},
 * {@link #readS32} and {@link #writeI32} here. {@link #randRange} is the single
 * uniform random source for combat rolls, drops, screen shake and idle jitter.
 */
public final class ByteUtil {
    /* renamed from: a */
    /** Shared RNG backing {@link #randRange}. */
    public static Random rng = new Random();

    /* renamed from: a */
    /**
     * Uniform random integer in the inclusive range {@code [low, high]}. Asserts
     * {@code low <= high}; returns 0 when the range is empty.
     */
    public static final int randRange(int low, int high) {
        Debug.assertTrue(low <= high);
        int span = (high - low) + 1;
        if (span == 0) {
            return 0;
        }
        return low + (Math.abs(rng.nextInt()) % span);
    }

    /* renamed from: a */
    /** Reads a big-endian unsigned 16-bit value at {@code offset} (returned as a short). */
    public static final short readU16(byte[] buffer, int offset) {
        if (buffer.length - 2 < offset) {
            throw new ArrayIndexOutOfBoundsException();
        }
        return (short) (((short) (0 | ((buffer[offset] & 255) << 8))) | (buffer[offset + 1] & 255));
    }

    /* renamed from: a */
    /** Returns a new array holding {@code first} followed by {@code second}. */
    public static final char[] concat(char[] first, char[] second) {
        char[] result = new char[first.length + second.length];
        System.arraycopy(first, 0, result, 0, first.length);
        System.arraycopy(second, 0, result, first.length, second.length);
        return result;
    }

    /* renamed from: a, reason: collision with other method in class */
    /** Reads a big-endian signed 32-bit value at {@code offset}, or -1 if out of range. */
    public static final int readS32(byte[] buffer, int offset) {
        if (buffer.length < offset + 4) {
            return -1;
        }
        return ((buffer[offset] & 255) << 24) | ((buffer[offset + 1] & 255) << 16) | ((buffer[offset + 2] & 255) << 8) | (buffer[offset + 3] & 255);
    }

    /* renamed from: a */
    /** Writes {@code value} as a big-endian 32-bit integer into {@code buffer} at {@code offset}. */
    public static final void writeI32(int value, byte[] buffer, int offset) {
        byte[] tmp = {0, 0, 0, 0};
        int v = value & (-1);
        tmp[0] = (byte) ((v >> 24) & 255);
        tmp[1] = (byte) ((v >> 16) & 255);
        tmp[2] = (byte) ((v >> 8) & 255);
        tmp[3] = (byte) (v & 255);
        System.arraycopy(tmp, 0, buffer, offset, 4);
    }
}
