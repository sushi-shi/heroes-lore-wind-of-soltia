package defpackage;

import java.io.ByteArrayInputStream;
import java.io.DataInputStream;
import java.io.IOException;

/* renamed from: cj */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:cj.class */
/**
 * The global localized-string table — the loader/decoder for
 * {@code lang/language.<suffix>} (Phase-1 spec §3.6, §1). A singleton
 * ({@link #instance}) holds the whole blob in memory and hands out individual
 * strings by numeric id via {@link #get}.
 *
 * <p>File layout: {@code [u32 BE payloadLen]} then a body of a per-id {@code u32
 * BE} offset table immediately followed by the packed records. String {@code i}
 * is located as {@code pos = (i<<2) + readInt(offsetTable[i]) + 2}, then read as
 * a {@code [u16 recLen][u16 utfLen][utf bytes]} modified-UTF-8 record — exactly
 * the {@code reset / skip(i<<2) / skip(readInt()) / skip(2) / readUTF()} sequence
 * in {@link #get}.
 *
 * <p>The five {@link #locales} suffixes are only default-index filenames; the EN
 * baseline ships a single mislabeled {@code language.fr-FR} that actually holds
 * English (§1). {@link #resolveLocale} matches {@code microedition.locale} to a
 * suffix (exact, then two-letter prefix with the {@code 0x8000} "prefix-match"
 * flag), and {@link #load} masks that flag off into {@link #localeIndex}.
 */
public final class StringTable {
    /* renamed from: a */
    /** The process-wide string table singleton. */
    public static StringTable instance = new StringTable();

    /* renamed from: a, reason: collision with other field name */
    /** Raw table body: the offset table followed by the packed UTF records (payload of the lang file). */
    public byte[] blob;

    /* renamed from: a, reason: collision with other field name */
    /** DataInputStream re-reading {@link #blob} for random access in {@link #get}. */
    public DataInputStream stream;

    /* renamed from: a, reason: collision with other field name */
    /** Index into {@link #locales} of the loaded language (the {@code 0x8000} prefix-flag masked off). */
    public byte localeIndex = 0;

    /* renamed from: a, reason: collision with other field name */
    /** Locale suffixes; only the default-index filename — carries no language meaning (§1). */
    public final String[] locales = {"en-GB", "fr-FR", "de-DE", "it-IT", "es-ES"};

    public StringTable() {
        // Leftover localized UI labels from the original class; unused at runtime
        // (constructed and discarded — the obfuscator kept them as dead locals).
        String[] selectLabelByLocale = {"Select", "Sélection.", "Wählen", "Selez.", "Elegir"};
        String[] exitLabelByLocale = {"Exit", "Quitter", "Beenden", "Esci", "Salir"};
    }

    /* renamed from: a */
    /**
     * Resolves a locale string (or {@code microedition.locale} when {@code null})
     * to a {@link #locales} index: an exact case-insensitive match returns the
     * plain index; a two-letter prefix match returns the index OR'd with
     * {@code 0x8000}; no match returns -1.
     */
    private int resolveLocale(String locale) throws IOException {
        int index = -1;
        if (locale == null) {
            try {
                locale = System.getProperty("microedition.locale");
            } catch (Exception unused) {
                locale = null;
            }
        }
        if (locale != null) {
            for (int i = 0; i < this.locales.length; i++) {
                if (this.locales[i].toLowerCase().compareTo(locale.toLowerCase()) == 0) {
                    index = i;
                    break;
                }
            }
            if (index == -1) {
                for (int i = 0; i < this.locales.length; i++) {
                    if (this.locales[i].toLowerCase().substring(0, 2).compareTo(locale.toLowerCase().substring(0, 2)) == 0) {
                        index = i | 32768;
                        break;
                    }
                }
            }
        }
        return index;
    }

    /* renamed from: a */
    /**
     * Loads {@code basePath + "." + locales[index]} into memory. When {@code index}
     * is negative it is resolved from {@code locale} via {@link #resolveLocale}
     * (falling back to 0). Reads the 4-byte payload length, slurps that many
     * bytes into {@link #blob}, and wraps it in {@link #stream}. Any
     * {@link IOException} during the read is caught and logged as
     * {@code "Couldn't load babble file."}, matching the original.
     */
    public final void load(String basePath, String locale, int index) throws IOException {
        try {
            if (index < 0) {
                index = resolveLocale(locale);
                if (index == -1) {
                    index = 0;
                }
            }
            this.localeIndex = (byte) (index & 32767);
            DataInputStream in = new DataInputStream(Runtime.getRuntime().getClass().getResourceAsStream(new StringBuffer().append(basePath).append(".").append(this.locales[this.localeIndex]).toString()));
            int payloadLen = in.readInt();
            this.blob = new byte[payloadLen];
            int filled = 0;
            int total;
            do {
                total = filled + in.read(this.blob, filled, payloadLen - filled);
                filled = total;
            } while (total < payloadLen);
        } catch (IOException e) {
            System.out.println(new StringBuffer().append("ERROR: Couldn't load babble file.").append(e).toString());
        }
        this.stream = new DataInputStream(new ByteArrayInputStream(this.blob));
    }

    /* renamed from: a */
    /**
     * Returns the localized string with the given id, decoded from {@link #blob}
     * via the offset table (see the class doc). On any failure returns a
     * diagnostic {@code "<id>.<exception>"} placeholder rather than throwing.
     */
    public final String get(int id) {
        try {
            this.stream.reset();
            this.stream.skip(id << 2);
            this.stream.skip(this.stream.readInt());
            this.stream.skip(2L);
            return this.stream.readUTF();
        } catch (Exception e) {
            return new StringBuffer().append(id).append(".").append(e.toString()).toString();
        }
    }

    static {
        // Leftover localized language names from the original static block; unused.
        String[] languageNames = {"English", "FranÇais", "Deutsch", "Italiano", "EspaÑol"};
    }
}
