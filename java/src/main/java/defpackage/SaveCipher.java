package defpackage;

/* renamed from: bq */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bq.class */
/**
 * The obfuscation cipher applied to every RMS save blob. It is a rolling XOR
 * against a repeating key with a trailing one-byte additive checksum:
 * <ul>
 *   <li>the key index is advanced <em>before</em> each byte (so the first
 *       plaintext byte is XORed with {@code key[1]}, not {@code key[0]}) and
 *       wraps to 0 whenever it reaches {@code key.length};</li>
 *   <li>the checksum accumulates {@code key[keyIndex] & 0xFF} for every byte and
 *       is appended as the final output byte.</li>
 * </ul>
 * {@link #encrypt} returns {@code plaintext.length + 1} bytes (ciphertext then
 * checksum); {@link #decrypt} reproduces the same keystream over all but the
 * last byte, recomputes the checksum and returns the plaintext only if it
 * matches the stored checksum, otherwise {@code null}. It is scrambling, not
 * real cryptography — the goal is to keep casual editors out of the save file.
 */
public final class SaveCipher {
    /* renamed from: a */
    /**
     * Encrypts {@code plaintext} under {@code key}. Output is
     * {@code plaintext.length + 1} bytes: the rolling-XOR ciphertext followed by
     * the additive checksum byte that {@link #decrypt} verifies.
     */
    public static final byte[] encrypt(byte[] plaintext, byte[] key) {
        byte[] out = new byte[plaintext.length + 1];
        int checksum = 0;
        int keyIndex = 0;
        for (int i = 0; i < plaintext.length; i++) {
            byte plain = plaintext[i];
            keyIndex++;
            if (keyIndex == key.length) {
                keyIndex = 0;
            }
            int cipher = plain ^ key[keyIndex];
            checksum += key[keyIndex] & 255;
            out[i] = (byte) cipher;
        }
        out[plaintext.length] = (byte) checksum;
        return out;
    }

    /* renamed from: b */
    /**
     * Decrypts a blob produced by {@link #encrypt}. Recreates the keystream over
     * {@code cipher[0 .. length-2]}, accumulates the checksum, and returns the
     * plaintext only if the recomputed checksum matches the trailing byte;
     * returns {@code null} on mismatch, on an empty input, or on any exception.
     */
    public static final byte[] decrypt(byte[] cipher, byte[] key) {
        byte[] out = new byte[cipher.length + 1];
        int checksum = 0;
        if (cipher.length < 1) {
            return null;
        }
        int keyIndex = 0;
        int i = 0;
        while (i < cipher.length - 1) {
            try {
                byte enc = cipher[i];
                keyIndex++;
                if (keyIndex == key.length) {
                    keyIndex = 0;
                }
                int plain = enc ^ key[keyIndex];
                checksum += key[keyIndex] & 255;
                out[i] = (byte) plain;
                i++;
            } catch (Exception unused) {
                return null;
            }
        }
        if ((checksum & 255) != (cipher[i] & 255)) {
            return null;
        }
        return out;
    }
}
