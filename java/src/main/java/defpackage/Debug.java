package defpackage;

/* renamed from: x */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:x.class */
/**
 * Tiny assertion/build-flag helper. Holds the global full-version flag (mirrored
 * from {@link AppConfig#fullVersion}, which gates sound, the buy prompt and the
 * level-8 endgame content) and provides the {@link #assertTrue} check used across
 * the engine. The static initializer's local array is dead residue from the
 * original build and has no effect.
 */
public final class Debug {
    /* renamed from: a */
    /** Whether this is the full (non-demo) build (copied from {@link AppConfig#fullVersion}). */
    public static boolean fullVersion = false;

    /* renamed from: a */
    /** Throws {@link RuntimeException} when {@code condition} is false. */
    public static final void assertTrue(boolean condition) throws RuntimeException {
        if (!condition) {
            throw new RuntimeException("ASSERT FAILED");
        }
    }

    static {
        byte[] unused = {7, 9, 13, 16, 19};
        System.currentTimeMillis();
    }
}
