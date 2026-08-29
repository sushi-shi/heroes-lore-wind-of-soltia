package defpackage;

import java.io.DataInputStream;
import java.io.InputStream;
import java.util.Enumeration;
import java.util.Vector;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/* renamed from: az */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:az.class */
/**
 * The bitmap-font engine — the loader and renderer for the game's {@code .mf}
 * proportional fonts. A {@code .mf} file is: a 4-byte header, then five bytes
 * {@code [lineHeight][ascent][spaceWidth][kerning]} (the lineHeight byte carries
 * a {@code +100} bias when the font includes the 9 extra accent glyphs), then a
 * short[] of cumulative glyph x-offsets (so glyph {@code n}'s width is
 * {@code offset[n+1]-offset[n]}), then a small index of PNG patch positions, and
 * finally an embedded paletted PNG glyph sheet. At load time the two ink colors
 * are written straight into the PNG's palette and its chunk CRC is recomputed
 * ({@link #patchPng} / {@link #crc32}), so one glyph sheet can be drawn in any
 * color. Rendering maps Latin-1 accented letters onto base glyph + separately
 * blitted accent marks, and supports optional force-uppercase and
 * control-character hiding.
 */
public class BitmapFont {
    /* renamed from: a */
    /** When true, lowercase letters are promoted to uppercase glyphs. */
    public boolean forceUppercase;

    /* renamed from: b */
    /** When true, the markup control chars {@code $ @ |} are skipped, not drawn. */
    public boolean hideControls;

    /* renamed from: c */
    /** Ink color used for the drawn bullet dot (char 176). */
    private int bulletColor;

    /* renamed from: a, reason: collision with other field name */
    /** The decoded glyph-sheet image (palette pre-patched to the ink colors). */
    private Image glyphSheet;

    /* renamed from: a, reason: collision with other field name */
    /** Full line height in pixels (with the {@code +100} accent bias stripped). */
    public int lineHeight;

    /* renamed from: d */
    /** Ascent (cap height) in pixels; the fallback line height when no descenders. */
    private int ascent;

    /* renamed from: e */
    /** Advance width of the space character. */
    private int spaceWidth;

    /* renamed from: f */
    /** Kerning: pixels subtracted from every glyph's advance. */
    private int kerning;

    /* renamed from: a, reason: collision with other field name */
    /** Cumulative glyph x-offsets into the sheet; width of glyph n is {@code [n+1]-[n]}. */
    private short[] glyphOffsets;

    /* renamed from: a, reason: collision with other field name */
    /** CRC-32 lookup table (polynomial 0xEDB88320) for {@link #crc32}. */
    private static final int[] crcTable = new int[256];

    /* renamed from: b, reason: collision with other field name */
    /** Extra vertical gap between wrapped lines. */
    public int lineSpacing = 2;

    /* renamed from: c, reason: collision with other field name */
    /** True when the font carries the 9 extra accent glyphs (lineHeight byte was biased +100). */
    private boolean hasAccents = true;

    public BitmapFont(String name, int primaryColor, int secondaryColor, boolean forceUppercase) {
        this.forceUppercase = forceUppercase;
        load(getClass().getResourceAsStream(new StringBuffer().append("/").append(name).append(".mf").toString()), primaryColor, secondaryColor);
    }

    /* renamed from: a */
    /**
     * Parses a {@code .mf} stream: reads the metrics and glyph-offset table,
     * patches the two ink colors into the embedded PNG palette (fixing its CRC),
     * and decodes the glyph sheet. Throws a {@code RuntimeException} tagged
     * "MFont:" on any failure.
     */
    public final void load(InputStream stream, int primaryColor, int secondaryColor) {
        this.bulletColor = primaryColor;
        try {
            stream.read();
            stream.read();
            stream.read();
            stream.read();
            this.lineHeight = stream.read();
            this.hasAccents = false;
            if (this.lineHeight - 100 > 0) {
                this.lineHeight -= 100;
                this.hasAccents = true;
            }
            this.ascent = stream.read();
            this.spaceWidth = stream.read();
            this.kerning = stream.read();
            this.glyphOffsets = new short[95 + (this.hasAccents ? 9 : 0)];
            int g = 0;
            while (true) {
                if (g >= 95 + (this.hasAccents ? 9 : 0)) {
                    break;
                }
                this.glyphOffsets[g] = (short) (((stream.read() & 255) << 8) | (stream.read() & 255));
                g++;
            }
            int pngLength = ((stream.read() & 255) << 8) | (stream.read() & 255);
            int chunkOffset = ((stream.read() & 255) << 8) | (stream.read() & 255);
            int hi = stream.read();
            int lo = stream.read();
            int primaryColorOffset = (hi == 255 && lo == 255) ? -1 : ((hi & 255) << 8) | (lo & 255);
            int secondaryColorOffset = ((stream.read() & 255) << 8) | (stream.read() & 255);
            DataInputStream in = new DataInputStream(stream);
            byte[] png = new byte[pngLength];
            in.readFully(png);
            in.close();
            if (primaryColorOffset > 0) {
                patchPng(png, chunkOffset, primaryColorOffset, primaryColor, secondaryColorOffset, secondaryColor);
            }
            this.glyphSheet = Image.createImage(png, 0, pngLength);
        } catch (Exception e) {
            throw new RuntimeException(new StringBuffer().append("MFont: ").append(e).toString());
        }
    }

    /* renamed from: a */
    /** Pixel width of a whole string (null-safe). */
    public final int stringWidth(String str) {
        if (str == null) {
            return 0;
        }
        char[] chars = str.toCharArray();
        return stringWidth(chars, 0, chars.length);
    }

    /* renamed from: a, reason: collision with other method in class */
    /** Pixel width of {@code count} chars from {@code chars} at {@code start} (skips control chars). */
    public final int stringWidth(char[] chars, int start, int count) {
        int width = 0;
        int end = start + count;
        for (int i = start; i < end; i++) {
            if (!isControl(chars[i])) {
                width += charWidth(chars[i]) - this.kerning;
            }
        }
        return width;
    }

    /* renamed from: a */
    /**
     * Advance width of a single character, resolving accented Latin-1 letters and
     * special glyphs (œ, ¡, ¿, ß, °...) to their sheet glyph and honoring the
     * force-uppercase / accented-uppercase rules.
     */
    public final int charWidth(char c) {
        if (c == 339) {
            return 7;
        }
        if (c <= ' ' || c >= 256) {
            if (c == ' ') {
                return this.spaceWidth;
            }
            return 0;
        }
        boolean z = 192 <= c && 223 > c;
        boolean z2 = z;
        if (z) {
            c = (char) (c + ' ');
        }
        if (c > 127) {
            switch (c) {
                case 161:
                    c = 129;
                    break;
                case 162:
                case 163:
                case 164:
                case 165:
                case 166:
                case 167:
                case 168:
                case 169:
                case 170:
                case 171:
                case 172:
                case 173:
                case 174:
                case 175:
                case 177:
                case 178:
                case 179:
                case 180:
                case 181:
                case 182:
                case 183:
                case 184:
                case 185:
                case 186:
                case 187:
                case 188:
                case 189:
                case 190:
                case 192:
                case 193:
                case 194:
                case 195:
                case 196:
                case 197:
                case 198:
                case 199:
                case 200:
                case 201:
                case 202:
                case 203:
                case 204:
                case 205:
                case 206:
                case 207:
                case 208:
                case 209:
                case 210:
                case 211:
                case 212:
                case 213:
                case 214:
                case 215:
                case 216:
                case 217:
                case 218:
                case 219:
                case 220:
                case 221:
                case 222:
                case 240:
                case 247:
                case 248:
                default:
                    c = '.';
                    break;
                case 176:
                    break;
                case 191:
                    c = 130;
                    break;
                case 223:
                    c = 127;
                    break;
                case 224:
                case 225:
                case 226:
                case 227:
                case 228:
                case 229:
                    c = 'a';
                    break;
                case 230:
                    c = z2 ? (char) 136 : (char) 138;
                    break;
                case 231:
                    c = z2 ? (char) 128 : (char) 137;
                    break;
                case 232:
                case 233:
                case 234:
                case 235:
                    c = 'e';
                    break;
                case 236:
                case 237:
                case 238:
                case 239:
                    c = 'i';
                    break;
                case 241:
                    c = 'n';
                    break;
                case 242:
                case 243:
                case 244:
                case 245:
                case 246:
                    c = 'o';
                    break;
                case 249:
                case 250:
                case 251:
                case 252:
                    c = 'u';
                    break;
            }
        }
        if (c >= 136) {
            switch (c) {
                case 136:
                    return 8;
                case 137:
                    return 5;
                case 138:
                    return 7;
                case 176:
                    return 4;
            }
        }
        int width = this.glyphOffsets[(c - '!') + 1] - this.glyphOffsets[c - '!'];
        if ((this.forceUppercase || z2) && c >= 'a' && c <= 'z') {
            char upper = (char) (c - ' ');
            width = this.glyphOffsets[(upper - '!') + 1] - this.glyphOffsets[upper - '!'];
        }
        return width;
    }

    /* renamed from: b */
    /** Line height for the given text: full {@link #lineHeight} if it has descenders, else {@link #ascent}+1. */
    public final int lineHeightOf(char[] chars, int start, int count) {
        int end = start + count;
        for (int i = start; i < end; i++) {
            if ("gjpqy,;_|ç¡¿".indexOf(chars[i]) != -1) {
                return this.lineHeight;
            }
        }
        return this.ascent + 1;
    }

    /* renamed from: a */
    /** Total pixel height of {@code lines} lines stacked with {@link #lineSpacing}. */
    public final int blockHeight(Vector lines) {
        return (this.lineHeight + this.lineSpacing) * lines.size();
    }

    /* renamed from: a */
    /** Draws a whole string at (x,y) with the given anchor. */
    public final int drawString(Graphics graphics, String str, int x, int y, int anchor) {
        return drawString(graphics, str, 0, str.length(), x, y, anchor);
    }

    /* renamed from: a */
    /** Draws a whole char array at (x,y) with the given anchor. */
    public final int drawChars(Graphics graphics, char[] chars, int x, int y, int anchor) {
        return drawChars(graphics, chars, 0, chars.length, x, y, anchor);
    }

    /* renamed from: a */
    /**
     * Draws a vector of pre-wrapped string lines starting at (x,y), advancing by
     * {@link #lineHeight}+{@link #lineSpacing} per line and clipping to
     * {@code bottomY}. Returns the total height drawn.
     */
    public final int drawLines(Graphics graphics, Vector lines, int x, int y, int bottomY, int anchor) {
        int lineY = y;
        int step = this.lineHeight + this.lineSpacing;
        Enumeration e = lines.elements();
        while (e.hasMoreElements()) {
            String line = (String) e.nextElement();
            if (lineY + step >= graphics.getClipY() && lineY < bottomY) {
                drawChars(graphics, line.toCharArray(), 0, line.length(), x, lineY, anchor);
            }
            lineY += step;
        }
        return lineY - y;
    }

    /* renamed from: a */
    /** Draws the substring {@code [start,end)} of {@code str} at (x,y) with anchor. */
    public final int drawString(Graphics graphics, String str, int start, int end, int x, int y, int anchor) {
        return drawChars(graphics, str.substring(start, end).toCharArray(), 0, end - start, x, y, anchor);
    }

    /* renamed from: a */
    /**
     * Core glyph blitter. Applies horizontal (center/right) and vertical
     * (bottom/baseline) anchoring, then for each char resolves it to a base glyph
     * (plus an optional accent mark blitted above) and draws it clipped from the
     * glyph sheet, advancing by the glyph width minus {@link #kerning}. Returns
     * the total advance.
     */
    public final int drawChars(Graphics graphics, char[] chars, int start, int count, int x, int y, int anchor) {
        int clipX = graphics.getClipX();
        int clipY = graphics.getClipY();
        int clipWidth = graphics.getClipWidth();
        int clipHeight = graphics.getClipHeight();
        if ((anchor & 1) != 0) {
            x -= stringWidth(chars, start, count) / 2;
        } else if ((anchor & 8) != 0) {
            x -= stringWidth(chars, start, count);
        }
        if ((anchor & 32) != 0) {
            y -= lineHeightOf(chars, start, count);
        } else if ((anchor & 64) != 0) {
            y -= this.ascent;
        }
        int advance = 0;
        int end = start + count;
        for (int idx = start; idx < end; idx++) {
            if (x > clipX + clipWidth) {
                graphics.setClip(clipX, clipY, clipWidth, clipHeight);
                return advance;
            }
            char c = chars[idx];
            if (!isControl(c)) {
                int accentIndex = -1;
                int accentDx = 0;
                if (c == ' ') {
                    x += this.spaceWidth;
                    advance += this.spaceWidth;
                } else {
                    boolean z = false;
                    if (c == 339) {
                        c = this.forceUppercase ? (char) 139 : (char) 140;
                    } else if (c > ' ' && c < 256) {
                        boolean z2 = 192 <= c && 223 > c;
                        z = z2;
                        if (z2) {
                            c = (char) (c + ' ');
                        }
                        if (c > 127) {
                            switch (c) {
                                case 161:
                                    c = 129;
                                    break;
                                case 162:
                                case 163:
                                case 164:
                                case 165:
                                case 166:
                                case 167:
                                case 168:
                                case 169:
                                case 170:
                                case 171:
                                case 172:
                                case 173:
                                case 174:
                                case 175:
                                case 177:
                                case 178:
                                case 179:
                                case 180:
                                case 181:
                                case 182:
                                case 183:
                                case 184:
                                case 185:
                                case 186:
                                case 187:
                                case 188:
                                case 189:
                                case 190:
                                case 192:
                                case 193:
                                case 194:
                                case 195:
                                case 196:
                                case 197:
                                case 198:
                                case 199:
                                case 200:
                                case 201:
                                case 202:
                                case 203:
                                case 204:
                                case 205:
                                case 206:
                                case 207:
                                case 208:
                                case 209:
                                case 210:
                                case 211:
                                case 212:
                                case 213:
                                case 214:
                                case 215:
                                case 216:
                                case 217:
                                case 218:
                                case 219:
                                case 220:
                                case 221:
                                case 222:
                                case 240:
                                case 247:
                                case 248:
                                default:
                                    c = '.';
                                    break;
                                case 176:
                                    break;
                                case 191:
                                    c = 130;
                                    break;
                                case 223:
                                    c = 127;
                                    break;
                                case 224:
                                    c = 'a';
                                    accentIndex = 0;
                                    accentDx = 1;
                                    break;
                                case 225:
                                    c = 'a';
                                    accentIndex = 1;
                                    accentDx = 1;
                                    break;
                                case 226:
                                    c = 'a';
                                    accentIndex = 2;
                                    accentDx = 1;
                                    break;
                                case 227:
                                    c = 'a';
                                    accentIndex = 3;
                                    accentDx = 1;
                                    break;
                                case 228:
                                    c = 'a';
                                    accentIndex = 4;
                                    accentDx = 1;
                                    break;
                                case 229:
                                    c = 'a';
                                    break;
                                case 230:
                                    c = (this.forceUppercase || z) ? (char) 136 : (char) 138;
                                    break;
                                case 231:
                                    c = (this.forceUppercase || z) ? (char) 128 : (char) 137;
                                    break;
                                case 232:
                                    c = 'e';
                                    accentIndex = 0;
                                    accentDx = 1;
                                    break;
                                case 233:
                                    c = 'e';
                                    accentIndex = 1;
                                    accentDx = 1;
                                    break;
                                case 234:
                                    c = 'e';
                                    accentIndex = 2;
                                    accentDx = 1;
                                    break;
                                case 235:
                                    c = 'e';
                                    accentIndex = 3;
                                    accentDx = 1;
                                    break;
                                case 236:
                                    c = 'i';
                                    accentIndex = 0;
                                    accentDx = -1;
                                    break;
                                case 237:
                                    c = 'i';
                                    accentIndex = 1;
                                    accentDx = 0;
                                    break;
                                case 238:
                                    c = 'i';
                                    accentIndex = 2;
                                    accentDx = -1;
                                    break;
                                case 239:
                                    c = 'i';
                                    accentIndex = 4;
                                    accentDx = -1;
                                    break;
                                case 241:
                                    c = 'n';
                                    accentIndex = 3;
                                    accentDx = 1;
                                    break;
                                case 242:
                                    c = 'o';
                                    accentIndex = 0;
                                    accentDx = 1;
                                    break;
                                case 243:
                                    c = 'o';
                                    accentIndex = 1;
                                    accentDx = 1;
                                    break;
                                case 244:
                                    c = 'o';
                                    accentIndex = 2;
                                    accentDx = 1;
                                    break;
                                case 245:
                                    c = 'o';
                                    accentIndex = 3;
                                    accentDx = 1;
                                    break;
                                case 246:
                                    c = 'o';
                                    accentIndex = 4;
                                    accentDx = 1;
                                    break;
                                case 249:
                                    c = 'u';
                                    accentIndex = 0;
                                    accentDx = 1;
                                    break;
                                case 250:
                                    c = 'u';
                                    accentIndex = 1;
                                    accentDx = 1;
                                    break;
                                case 251:
                                    c = 'u';
                                    accentIndex = 2;
                                    accentDx = 1;
                                    break;
                                case 252:
                                    c = 'u';
                                    accentIndex = 4;
                                    accentDx = 1;
                                    break;
                            }
                        }
                        if ((this.forceUppercase || z) && c >= 'a' && c <= 'z') {
                            c = (char) (c - ' ');
                        }
                    }
                    short glyphX = 0;
                    int glyphWidth = 0;
                    switch (c) {
                        case 136:
                            glyphX = this.glyphOffsets[this.glyphOffsets.length - 1];
                            glyphWidth = 8;
                            break;
                        case 137:
                            glyphX = (short) (this.glyphOffsets[this.glyphOffsets.length - 1] + 8);
                            glyphWidth = 5;
                            break;
                        case 138:
                            glyphX = (short) (this.glyphOffsets[this.glyphOffsets.length - 1] + 8 + 5);
                            glyphWidth = 7;
                            break;
                        case 139:
                            glyphX = (short) (this.glyphOffsets[this.glyphOffsets.length - 1] + 8 + 5 + 7);
                            glyphWidth = 7;
                            break;
                        case 140:
                            glyphX = (short) (this.glyphOffsets[this.glyphOffsets.length - 1] + 8 + 5 + 7 + 7);
                            glyphWidth = 7;
                            break;
                        case 176:
                            graphics.setColor(this.bulletColor);
                            graphics.setClip(x, y, 4, 4);
                            graphics.drawArc(x, y, 2, 2, 0, 360);
                            x += 4;
                            advance += 4;
                            continue;
                        default:
                            int glyphIndex = c - '!';
                            if (glyphIndex >= 0 && glyphIndex < this.glyphOffsets.length - 1) {
                                glyphX = this.glyphOffsets[glyphIndex];
                                glyphWidth = this.glyphOffsets[glyphIndex + 1] - glyphX;
                            }
                            break;
                    }
                    short accentX = this.hasAccents ? this.glyphOffsets[98 + accentIndex] : (short) 0;
                    int accentWidth = this.hasAccents ? this.glyphOffsets[(98 + accentIndex) + 1] - accentX : 0;
                    if ((x + glyphWidth) - this.kerning < clipX) {
                        x += glyphWidth - this.kerning;
                        advance += glyphWidth - this.kerning;
                    } else {
                        graphics.setClip(clipX, clipY, clipWidth, clipHeight);
                        if (c != 'i' || accentIndex < 0) {
                            graphics.clipRect(x, y, glyphWidth, this.lineHeight);
                        } else {
                            graphics.clipRect(x, y + 1, glyphWidth, this.lineHeight);
                        }
                        graphics.drawImage(this.glyphSheet, x - glyphX, y, 20);
                        if (accentIndex >= 0) {
                            graphics.setClip(clipX, clipY, clipWidth, clipHeight);
                            int markWidth = accentIndex != 4 ? accentWidth : accentWidth + 1;
                            int raise = (this.forceUppercase || z) ? 2 : 0;
                            if (c != 'i' || accentIndex < 0) {
                                graphics.clipRect(x, (y - 1) - raise, markWidth + accentDx, this.lineHeight + raise);
                            } else {
                                graphics.clipRect(x - 1, (y - 1) - raise, markWidth + accentDx, this.lineHeight + raise);
                            }
                            graphics.drawImage(this.glyphSheet, (x - accentX) + accentDx, (y - 1) - raise, 20);
                        }
                        x += glyphWidth - this.kerning;
                        advance += glyphWidth - this.kerning;
                    }
                }
            }
        }
        graphics.setClip(clipX, clipY, clipWidth, clipHeight);
        return advance;
    }

    /* renamed from: a, reason: collision with other method in class */
    /** True for the markup control chars {@code $ @ |} while {@link #hideControls} is set. */
    private boolean isControl(char c) {
        if (!this.hideControls) {
            return false;
        }
        switch (c) {
            case '$':
            case '@':
            case '|':
                return true;
            default:
                return false;
        }
    }

    /* renamed from: a, reason: collision with other method in class */
    /**
     * Writes the two ink colors into the embedded PNG's palette chunk and
     * recomputes that chunk's CRC in place. {@code chunkOffset} points at the
     * chunk's 4-byte length field; the primary color is written at
     * {@code primaryColorOffset} and (when {@code secondaryColorOffset > 0}) the
     * secondary color at its offset; the CRC over {chunk-type + data} is written
     * after the data.
     */
    private void patchPng(byte[] png, int chunkOffset, int primaryColorOffset, int primaryColor, int secondaryColorOffset, int secondaryColor) {
        int chunkLen = ((png[chunkOffset] & 255) << 24) | ((png[chunkOffset + 1] & 255) << 16) | ((png[chunkOffset + 2] & 255) << 8) | (png[chunkOffset + 3] & 255);
        png[primaryColorOffset] = (byte) (primaryColor >> 16);
        png[primaryColorOffset + 1] = (byte) (primaryColor >> 8);
        png[primaryColorOffset + 2] = (byte) primaryColor;
        if (secondaryColorOffset > 0 && secondaryColor >= 0) {
            png[secondaryColorOffset] = (byte) (secondaryColor >> 16);
            png[secondaryColorOffset + 1] = (byte) (secondaryColor >> 8);
            png[secondaryColorOffset + 2] = (byte) secondaryColor;
        }
        int crc = crc32(png, chunkOffset + 4, chunkLen + 4);
        int crcPos = chunkOffset + 8 + chunkLen;
        png[crcPos] = (byte) (crc >> 24);
        png[crcPos + 1] = (byte) (crc >> 16);
        png[crcPos + 2] = (byte) (crc >> 8);
        png[crcPos + 3] = (byte) crc;
    }

    /* renamed from: a, reason: collision with other method in class */
    /** Standard CRC-32 (poly 0xEDB88320) over {@code length} bytes of {@code data} from {@code offset}. */
    private int crc32(byte[] data, int offset, int length) {
        int crc = -1;
        while (true) {
            int prev = crc;
            length--;
            if (length < 0) {
                return prev ^ (-1);
            }
            int pos = offset;
            offset++;
            crc = crcTable[(prev ^ data[pos]) & 255] ^ (prev >>> 8);
        }
    }

    static {
        for (int i = 0; i < 256; i++) {
            int c = i;
            for (int bit = 0; bit < 8; bit++) {
                c = (c & 1) != 0 ? (-306674912) ^ (c >>> 1) : c >>> 1;
                crcTable[i] = c;
            }
        }
    }
}
