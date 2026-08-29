package defpackage;

import java.util.Vector;

/* renamed from: b */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:b.class */
/**
 * A {@link BitmapFont} that adds greedy, space-aware word wrapping. On top of the
 * base glyph engine it provides {@link #wrap} (how many characters of a string
 * fit in a pixel width / line budget) and {@link #wrapInto} (split a string into
 * a {@link Vector} of lines for a width), breaking at spaces where possible and
 * mid-word only when a single word overflows, and honoring embedded {@code '\n'}.
 * {@link FontManager} builds all its concrete fonts through the {@link #create}
 * factories.
 */
public final class WrapFont extends BitmapFont {
    private WrapFont(String name, int primaryColor, int secondaryColor, boolean forceUppercase) {
        super(name, primaryColor, secondaryColor, forceUppercase);
    }

    /* renamed from: a */
    /**
     * Returns how many characters of {@code str} fit within pixel {@code width}
     * over at most {@code maxLines} lines (a fast path returns the whole length
     * for short single-line strings). Breaks at the last space before an overflow,
     * or mid-word if a word is itself too wide.
     */
    public final int wrap(String str, int width, int maxLines) {
        int length = str == null ? 0 : str.length();
        int total = length;
        if (length == 0) {
            return 0;
        }
        char[] chars = str.toCharArray();
        if (total < 64 && str.indexOf(10) == -1 && stringWidth(chars, 0, total) <= width) {
            return str.length();
        }
        int lineStart = 0;
        int pos = 0;
        while (maxLines > 0) {
            int rowStart = lineStart;
            int rowWidth = 0;
            int span = 1;
            while (maxLines > 0) {
                int probe = (rowStart + span) - 1;
                pos = probe;
                if (probe < total) {
                    char c = chars[pos];
                    rowWidth += charWidth(c);
                    if (c != '\n') {
                        if (rowWidth >= width) {
                            while (pos > rowStart && chars[pos] != ' ') {
                                pos--;
                            }
                            if (chars[pos] != ' ') {
                                lineStart = (lineStart + span) - 1;
                                maxLines--;
                                break;
                            }
                            lineStart = pos + 1;
                            maxLines--;
                            break;
                        }
                        span++;
                    } else {
                        lineStart = pos + 1;
                        while (lineStart < total && chars[lineStart] == ' ') {
                            lineStart++;
                        }
                        maxLines--;
                        break;
                    }
                } else {
                    return pos + 1;
                }
            }
        }
        return pos + 1;
    }

    /* renamed from: a */
    /**
     * Splits {@code str} into wrapped lines for pixel {@code width}, appending each
     * line to {@code out} (which it first clears) and returning it. Breaks at
     * spaces, mid-word on overflow, and at embedded newlines.
     */
    public final Vector wrapInto(Vector out, String str, int width) {
        int lineStart;
        out.removeAllElements();
        int length = str == null ? 0 : str.length();
        int total = length;
        if (length == 0) {
            return out;
        }
        char[] chars = str.toCharArray();
        if (total < 64 && str.indexOf(10) == -1 && stringWidth(chars, 0, total) <= width) {
            out.addElement(str);
            return out;
        }
        int cursor = 0;
        loop0: while (true) {
            lineStart = cursor;
            int rowWidth = 0;
            int span = 1;
            while (true) {
                int probe = (lineStart + span) - 1;
                int pos = probe;
                if (probe < total) {
                    char c = chars[pos];
                    rowWidth += charWidth(c);
                    if (c != '\n') {
                        if (rowWidth >= width) {
                            while (pos > lineStart && chars[pos] != ' ') {
                                pos--;
                            }
                            if (chars[pos] != ' ') {
                                cursor = (cursor + span) - 1;
                                out.addElement(new String(chars, lineStart, cursor - lineStart));
                                break;
                            }
                            cursor = pos + 1;
                            out.addElement(new String(chars, lineStart, pos - lineStart));
                            break;
                        }
                        span++;
                    } else {
                        cursor = pos + 1;
                        while (cursor < total && chars[cursor] == ' ') {
                            cursor++;
                        }
                        out.addElement(new String(chars, lineStart, pos - lineStart));
                        break;
                    }
                } else {
                    break loop0;
                }
            }
        }
        if (lineStart < total) {
            out.addElement(new String(chars, lineStart, total - lineStart));
        }
        return out;
    }

    /* renamed from: a */
    /** Factory: a wrapping font with explicit primary/secondary ink colors. */
    public static final BitmapFont create(String name, int primaryColor, int secondaryColor, boolean forceUppercase) {
        return new WrapFont(name, primaryColor, secondaryColor, forceUppercase);
    }

    /* renamed from: a, reason: collision with other method in class */
    /** Factory: a wrapping font with a single ink color (no secondary color). */
    public static final BitmapFont create(String name, int color, boolean forceUppercase) {
        return create(name, color, -1, forceUppercase);
    }
}
