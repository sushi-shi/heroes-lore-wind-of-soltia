package defpackage;

import java.io.IOException;

/* renamed from: z */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:z.class */
/**
 * A parsed {@code .tdf} table-def (Phase-1 spec §3.7): a compact list that maps
 * a small local index to a global {@link StringTable} string id. The file is
 * {@code [u8 count]} then {@code count} records of {@code [u16 BE len][len ASCII
 * bytes]}, where the ASCII bytes are the decimal text of the string id. The
 * table therefore holds no text itself — {@link #get} resolves an entry through
 * the loaded {@link StringTable} at read time. UI screens (shop, blacksmith,
 * help, hero/guardian panels, endings...) each load their own {@code .tdf} and
 * index it by button/line number.
 */
public final class TextTable {
    /* renamed from: a */
    /** Global {@link StringTable} id for each local entry (parsed decimal from the {@code .tdf}). */
    private int[] stringIds;

    /* renamed from: a, reason: collision with other field name */
    /** Number of entries (the leading {@code u8} count byte of the {@code .tdf}). */
    public short count;

    public TextTable(String basePath) throws IOException {
        byte[] data = AssetCache.readResource(new StringBuffer().append(basePath).append(".tdf").toString());
        int pos = 0 + 1;
        this.count = (short) (data[0] & 255);
        this.stringIds = new int[this.count];
        for (int i = 0; i < this.count; i++) {
            int lenHi = pos;
            int lenLo = pos + 1;
            int textStart = lenLo + 1;
            int asciiLen = ((data[lenHi] & 255) << 8) + (data[lenLo] & 255);
            this.stringIds[i] = Integer.parseInt(new String(data, textStart, asciiLen).trim());
            pos = textStart + asciiLen;
        }
    }

    /* renamed from: a */
    /**
     * Resolves local entry {@code index} to its localized string, converting the
     * {@code ';'} record separator to a newline and returning it as a char array
     * (the form the {@link FontManager} renderer consumes).
     */
    public final char[] get(int index) {
        return StringTable.instance.get(this.stringIds[index]).replace(';', '\n').toCharArray();
    }
}
