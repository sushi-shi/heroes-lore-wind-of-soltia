package defpackage;

import java.io.IOException;
import java.util.Vector;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;
import rpg.GameMIDlet;

/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:FontManager.class */
/* renamed from: bh */
/**
 * The game's text / label / font hub.
 *
 * <p>Responsibilities:</p>
 * <ul>
 *   <li><b>Fonts.</b> Owns the six {@link WrapFont}/{@link BitmapFont} instances,
 *       keyed by size (small / big) and pen colour (black / white / orange), plus
 *       the {@link #currentFont} pointer that {@link #setBigFont(boolean)} swings
 *       between the small and big families.</li>
 *   <li><b>UI labels.</b> Caches the localized soft-key and menu label text as
 *       {@code char[]} / {@code String} fields. {@link #loadLabels(StringTable)}
 *       fills them once from {@link StringTable} (lang ids 3902..3950); a few
 *       (loading title/subtitle, version) are filled later by their owners.</li>
 *   <li><b>String resolution.</b> {@link #getString(int)} looks up a lang id and
 *       turns embedded {@code ';'} separators into newlines; {@link #getStringChars(String)}
 *       does the same for an ASCII-decimal id embedded in binary data (item names,
 *       enemy names, dialogue), returning glyph {@code char[]}.</li>
 *   <li><b>Layout &amp; drawing.</b> Wraps, measures and draws text blocks through
 *       {@link #currentFont}, caching the most recent wrap in {@link #wrapCache}.</li>
 *   <li><b>Locale images.</b> {@link #loadLocaleImage(String)} loads a per-locale
 *       image resource.</li>
 * </ul>
 */
public final class FontManager {
    /** Prefix shown before pickup amounts, e.g. {@code "Gain "} (lang 3902). */
    /* renamed from: a */
    public static String goldGainPrefix;

    /** {@code "Progress:"} label (lang 3903). */
    /* renamed from: b */
    public static String progressLabel;

    /** {@code "Are You Sure?"} confirmation prompt (lang 3904). */
    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /* renamed from: a */
    public static char[] confirmPrompt;

    /** Marketing URL {@code "www.HandsOn.com/Heroes"} (lang 3906). */
    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /* renamed from: b */
    public static char[] websiteText;

    /** {@code "Exit"} soft-key label (lang 3907). */
    /* renamed from: c */
    public static char[] labelExit;

    /** {@code "Ok"} soft-key label (lang 3908). */
    /* renamed from: d */
    public static char[] labelOk;

    /** {@code "Back"} soft-key label (lang 3909); the most common right label. */
    /* renamed from: e */
    public static char[] labelBack;

    /** {@code "Skip"} soft-key label (lang 3910). */
    /* renamed from: f */
    public static char[] labelSkip;

    /** {@code "Next"} soft-key label (lang 3911). */
    /* renamed from: g */
    public static char[] labelNext;

    /** {@code "Sell"} soft-key label (lang 3912). */
    /* renamed from: h */
    public static char[] labelSell;

    /** {@code "Select"} soft-key label (lang 3913). */
    /* renamed from: i */
    public static char[] labelSelect;

    /** {@code "Buy"} soft-key label (lang 3914). */
    /* renamed from: j */
    public static char[] labelBuy;

    /** {@code "Yes"} soft-key label (lang 3915); default left label. */
    /* renamed from: k */
    public static char[] labelYes;

    /** {@code "No"} soft-key label (lang 3916); default right label. */
    /* renamed from: l */
    public static char[] labelNo;

    /** {@code "LEVEL "} prefix (lang 3932). */
    /* renamed from: s */
    public static char[] levelPrefix;

    /** Current font: the small or big family selected by {@link #setBigFont(boolean)}. */
    /* JADX INFO: renamed from: g, reason: collision with other field name */
    /* renamed from: g */
    private static WrapFont currentFont;

    /** Small font, black pen (fonts/small, colour 0). */
    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /* renamed from: a */
    public static WrapFont smallBlack;

    /** Small font, white pen (fonts/small, colour 0xFFFFFF). */
    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /* renamed from: b */
    public static WrapFont smallWhite;

    /** Small font, orange pen (fonts/small, colour 0xFF8000). */
    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /* renamed from: c */
    public static WrapFont smallOrange;

    /** Big font, black fill with white outline (fonts/big). */
    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /* renamed from: d */
    public static WrapFont bigBlack;

    /** Big font, white fill with black outline (fonts/big). */
    /* JADX INFO: renamed from: e, reason: collision with other field name */
    /* renamed from: e */
    public static WrapFont bigWhite;

    /** Big font, default pen; an alias of {@link #bigBlack}. */
    /* JADX INFO: renamed from: f, reason: collision with other field name */
    /* renamed from: f */
    public static WrapFont bigDefault;

    /** Main-menu item labels: New Game / Load Game / Options / Help / About / (trial buy) / Exit (lang 3920..3926). */
    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /* renamed from: a */
    public static char[][] mainMenuLabels = new char[7][];

    /** A blank spacer label (15 spaces), used where a soft key has no text. */
    /* renamed from: m */
    public static char[] blankLabel = "               ".toCharArray();

    /** {@code "Paused"} label (lang 3946). */
    /* renamed from: n */
    public static char[] pausedLabel = null;

    /** Loading-screen subtitle; filled at runtime from {@code AssetCache.commonText[38]}. */
    /* renamed from: o */
    public static char[] loadingSubtitle = null;

    /** Loading-screen title; filled at runtime from {@code AssetCache.commonText[37]}. */
    /* renamed from: p */
    public static char[] loadingTitle = null;

    /** {@code "LV"} level abbreviation used in labelled stat boxes (lang 3949). */
    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /* renamed from: c */
    public static String levelAbbrev = null;

    /** Title-screen footer line (lang 3950). */
    /* renamed from: q */
    public static char[] titleFooter = null;

    /** Version text line ("v.x.y"); filled at runtime by {@code AppConfig}. */
    /* renamed from: r */
    public static char[] versionText = null;

    /** {@code "no space"} inventory-full message (lang 3947). */
    /* renamed from: t */
    public static char[] noSpaceLabel = null;

    /** {@code "G"} gold abbreviation appended to pickup amounts (lang 3948). */
    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /* renamed from: d */
    public static String goldAbbrev = null;

    /** True while the big font is active (see {@link #setBigFont(boolean)}). */
    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /* renamed from: a */
    public static boolean bigFontActive = false;

    /** Scratch cache holding the lines of the most recent {@link #wrapLines(String, int)} call. */
    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /* renamed from: a */
    public static Vector wrapCache = new Vector();

    /** Cache key guarding {@link #wrapCache} (kept as the empty string). */
    /* JADX INFO: renamed from: e, reason: collision with other field name */
    /* renamed from: e */
    public static String wrapCacheKey = "";

    /**
     * Draws the bottom soft-key command bar: a black box + white label in the
     * bottom-left corner for {@code leftLabel} and in the bottom-right for
     * {@code rightLabel}. Either may be {@code null} to omit that corner.
     *
     * @param graphics   target surface
     * @param leftLabel  left soft-key text, or {@code null}
     * @param rightLabel right soft-key text, or {@code null}
     */
    /* renamed from: a */
    public static final void drawSoftKeys(Graphics graphics, char[] leftLabel, char[] rightLabel) {
        graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        int barHeight = lineHeight() + 5;
        if (leftLabel != null) {
            int boxWidth = stringWidth(leftLabel) + 2;
            int boxY = (defpackage.BaseCanvas.height - barHeight) + 3;
            graphics.setColor(0);
            graphics.fillRect(0, boxY, boxWidth, barHeight);
            graphics.setColor(16777215);
            drawChars(graphics, 1, boxY + 1, leftLabel, 1);
        }
        if (rightLabel != null) {
            int boxWidth = stringWidth(rightLabel) + 2;
            int boxX = defpackage.BaseCanvas.width - boxWidth;
            int boxY = (defpackage.BaseCanvas.height - barHeight) + 3;
            graphics.setColor(0);
            graphics.fillRect(boxX, boxY, boxWidth, barHeight);
            graphics.setColor(16777215);
            drawChars(graphics, boxX + 1, boxY + 1, rightLabel, 1);
        }
    }

    /**
     * Clears the whole canvas to black.
     *
     * @param graphics target surface
     */
    /* renamed from: a */
    public static final void clearScreen(Graphics graphics) {
        graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        graphics.setColor(0);
        graphics.fillRect(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
    }

    /**
     * Opens the "buy the full version" URL through a platform request and then
     * quits the MIDlet. The argument is ignored; the URL comes from
     * {@code AppConfig.buyUrl}.
     *
     * @param ignored unused
     */
    /* JADX WARN: Type inference failed for: r0v3, types: [boolean, java.lang.Throwable] */
    /* renamed from: a */
    public static final void requestBuyAndExit(String ignored) {
        try {
            GameMIDlet.instance.platformRequest(AppConfig.buyUrl);
        } catch (Exception e2) {
            e2.printStackTrace();
        }
        GameMIDlet.instance.exit();
    }

    /**
     * Draws one centred main-menu item in the big font. The item is
     * {@code mainMenuLabels[itemState >> 1]}; an even {@code itemState} renders it
     * white (highlighted), an odd one renders it black.
     *
     * @param graphics  target surface
     * @param itemState item index, doubled, with the low bit as the highlight flag
     * @param unused    unused
     * @param y         baseline y
     */
    /* renamed from: a */
    public static final void drawMenuItem(Graphics graphics, int itemState, int unused, int y) {
        int centerX = defpackage.BaseCanvas.width >> 1;
        setBigFont(true);
        int index = itemState >> 1;
        if (itemState % 2 == 0) {
            graphics.setColor(16777215);
        } else {
            graphics.setColor(0);
        }
        drawCharsCentered(graphics, centerX, y + 4, mainMenuLabels[index], 1);
        setBigFont(false);
    }

    /**
     * Selects the small or big font family as {@link #currentFont}.
     *
     * @param active {@code true} to activate the big font, {@code false} the small
     */
    /* renamed from: a */
    public static final void setBigFont(boolean active) {
        bigFontActive = active;
        if (bigFontActive) {
            currentFont = bigBlack;
        } else {
            currentFont = smallBlack;
        }
    }

    /**
     * Returns {@code value} scaled to {@code percent} percent, i.e. {@code value * percent / 100}.
     */
    /* renamed from: a */
    public static final int percentOf(int value, int percent) {
        return (value * percent) / 100;
    }

    /**
     * Loads every static UI label from {@link StringTable} (lang ids 3902..3950).
     * Called once after the string table is available.
     *
     * @param table the active string table (unused; {@link StringTable#instance} is read directly)
     */
    /* renamed from: a */
    public static final void loadLabels(StringTable table) {
        goldGainPrefix = new StringBuffer().append(getString(3902)).append(" ").toString();
        progressLabel = getString(3903);
        confirmPrompt = getString(3904).toCharArray();
        websiteText = getString(3906).toCharArray();
        labelExit = getString(3907).toCharArray();
        labelOk = getString(3908).toCharArray();
        labelBack = getString(3909).toCharArray();
        labelSkip = getString(3910).toCharArray();
        labelNext = getString(3911).toCharArray();
        labelSell = getString(3912).toCharArray();
        labelSelect = getString(3913).toCharArray();
        labelBuy = getString(3914).toCharArray();
        labelYes = getString(3915).toCharArray();
        labelNo = getString(3916).toCharArray();
        mainMenuLabels[0] = getString(3920).toCharArray();
        mainMenuLabels[1] = getString(3921).toCharArray();
        mainMenuLabels[2] = getString(3922).toCharArray();
        mainMenuLabels[3] = getString(3923).toCharArray();
        mainMenuLabels[4] = getString(3924).toCharArray();
        mainMenuLabels[5] = getString(3924).toCharArray();
        mainMenuLabels[6] = getString(3926).toCharArray();
        levelPrefix = getString(3932).toCharArray();
        pausedLabel = getString(3946).toCharArray();
        noSpaceLabel = getString(3947).toCharArray();
        goldAbbrev = getString(3948);
        levelAbbrev = getString(3949);
        titleFooter = getString(3950).toCharArray();
    }

    /**
     * Resolves a lang id to its text, replacing embedded {@code ';'} separators
     * with newlines.
     *
     * @param id lang string id
     * @return the resolved, newline-normalized string
     */
    /* renamed from: a */
    public static final String getString(int id) {
        return StringTable.instance.get(id).replace(';', '\n');
    }

    /**
     * Resolves an ASCII-decimal lang id (as extracted from binary asset data —
     * item names/descriptions, enemy names, dialogue) to glyph {@code char[]},
     * replacing {@code ';'} with newlines. On any parse/lookup failure returns a
     * {@code "2."}-prefixed diagnostic string instead of throwing.
     *
     * @param id ASCII-decimal id, possibly padded with whitespace
     * @return the resolved glyph characters (or a diagnostic on failure)
     */
    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /* renamed from: a */
    public static final char[] getStringChars(String id) {
        try {
            return StringTable.instance.get(Integer.parseInt(id.trim())).replace(';', '\n').toCharArray();
        } catch (Exception e2) {
            return new StringBuffer().append("2.").append(e2.toString()).toString().toCharArray();
        }
    }

    /**
     * Builds the six font instances (small black/white/orange, big black/white,
     * plus the big-default alias), disables their control glyphs, and selects the
     * small-black font as {@link #currentFont}.
     */
    /* renamed from: a */
    public static final void initFonts() {
        smallBlack = (WrapFont) defpackage.WrapFont.create("fonts/small", 0, false);
        smallWhite = (WrapFont) defpackage.WrapFont.create("fonts/small", 16777215, false);
        smallOrange = (WrapFont) defpackage.WrapFont.create("fonts/small", 16746496, false);
        bigBlack = (WrapFont) defpackage.WrapFont.create("fonts/big", 0, 16777215, true);
        bigWhite = (WrapFont) defpackage.WrapFont.create("fonts/big", 16777215, 0, true);
        bigDefault = bigBlack;
        ((BitmapFont) smallBlack).hideControls = true;
        ((BitmapFont) smallWhite).hideControls = true;
        ((BitmapFont) smallOrange).hideControls = true;
        ((BitmapFont) bigBlack).hideControls = true;
        ((BitmapFont) bigWhite).hideControls = true;
        ((BitmapFont) bigDefault).hideControls = true;
        currentFont = smallBlack;
    }

    /**
     * @return the line height of {@link #currentFont} in pixels
     */
    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /* renamed from: a */
    public static final int lineHeight() {
        return ((BitmapFont) currentFont).lineHeight;
    }

    /**
     * @return the rendered pixel width of {@code chars} in {@link #currentFont}
     */
    /* renamed from: a */
    public static final int stringWidth(char[] chars) {
        return currentFont.stringWidth(charsToString(chars));
    }

    /**
     * @return {@code true} if {@code c} is the {@code ';'} line separator
     */
    /* renamed from: a */
    private static final boolean isSeparator(char c) {
        return c == ';';
    }

    /**
     * Advances a typewriter reveal by one word: starting at {@code start + shown},
     * skips separators, then returns the character count (relative to {@code start})
     * up to and including the next word after the first, or the remaining length if
     * the text ends first. Used by the dialogue textbox to reveal text word by word.
     *
     * @param text  the full text
     * @param start offset of the current line within {@code text}
     * @param shown characters already revealed on the current line
     * @return the new revealed-character count
     */
    /* renamed from: a */
    public static final int advanceWord(char[] text, int start, int shown) {
        int i = start + shown;
        boolean seenWord = false;
        while (i < text.length) {
            if (isSeparator(text[i])) {
                i++;
            } else {
                if (seenWord) {
                    return (i + 1) - start;
                }
                seenWord = true;
                i++;
            }
        }
        return text.length - start;
    }

    /**
     * @return the number of characters from {@code start} that fit on one line of
     *         width {@code widthPx} in {@link #currentFont}
     */
    /* renamed from: a */
    public static final int charsInLine(char[] text, int start, int widthPx, int flags) {
        return currentFont.wrap(new String(text, start, text.length - start), widthPx, flags);
    }

    /**
     * @return the number of wrapped lines {@code chars} occupies at {@code widthPx}
     */
    /* renamed from: a */
    public static final int lineCount(char[] chars, int widthPx) {
        return wrapLines(charsToString(chars), widthPx).size();
    }

    /**
     * @return the pixel height of {@code chars} wrapped to {@code widthPx}
     *         (only {@code widthPx} and {@code chars} are used)
     */
    /* renamed from: a */
    public static final int measureBlockHeight(int widthPx, int arg2, char[] chars, int arg4, int arg5, int arg6) {
        return blockHeightPx(new String(chars), 0, 0, widthPx);
    }

    /**
     * @return the line spacing of {@link #currentFont} in pixels
     */
    /* renamed from: b */
    private static int lineSpacing() {
        return ((BitmapFont) currentFont).lineSpacing;
    }

    /**
     * @return the pixel height of {@code text} wrapped to {@code widthPx}, minus one
     *         line's spacing (only {@code text} and {@code widthPx} are used)
     */
    /* renamed from: a */
    private static int blockHeightPx(String text, int arg2, int arg3, int widthPx) {
        return currentFont.blockHeight(wrapLines(text, widthPx)) - lineSpacing();
    }

    /**
     * Wraps {@code text} to {@code widthPx} into {@link #wrapCache} and returns it.
     * Guarded by {@link #wrapCacheKey} (which stays empty, so any non-empty text
     * re-wraps).
     *
     * @return {@link #wrapCache}, holding the wrapped lines
     */
    /* renamed from: a */
    private static Vector wrapLines(String text, int widthPx) {
        if (!text.equals(wrapCacheKey)) {
            wrapCache.setSize(0);
            currentFont.wrapInto(wrapCache, text, widthPx);
        }
        return wrapCache;
    }

    /**
     * Draws {@code chars} anchored top-left at ({@code x},{@code y}) in the font
     * matching the current pen colour.
     *
     * @return the advance width returned by the underlying font
     */
    /* renamed from: a */
    public static final int drawChars(Graphics graphics, int x, int y, char[] chars, int flags) {
        return fontForColor(graphics.getColor()).drawChars(graphics, chars, x, y, 20);
    }

    /**
     * Picks the {@link BitmapFont} matching the current pen colour and font size:
     * for the big font, colour 0 -&gt; {@link #bigBlack}, 0xFFFFFF -&gt; {@link #bigWhite},
     * else {@link #bigDefault}; for the small font, 0 -&gt; {@link #smallBlack},
     * 0xFFFFFF -&gt; {@link #smallWhite}, else {@link #smallOrange}.
     *
     * @param color the current pen colour
     * @return the font to draw with
     */
    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /* renamed from: a */
    private static BitmapFont fontForColor(int color) {
        if (bigFontActive) {
            if (color == 0) {
                return bigBlack;
            }
            return color == 16777215 ? bigWhite : bigDefault;
        }
        if (color == 0) {
            return smallBlack;
        }
        return color == 16777215 ? smallWhite : smallOrange;
    }

    /**
     * Draws {@code chars} horizontally centred on {@code x} at {@code y} in the font
     * matching the current pen colour.
     */
    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /* renamed from: a */
    public static final void drawCharsCentered(Graphics graphics, int x, int y, char[] chars, int flags) {
        fontForColor(graphics.getColor()).drawChars(graphics, chars, x, y, 17);
    }

    /**
     * Draws a wrapped block of {@code count} characters (from {@code offset}) of
     * {@code chars}, wrapped to {@code widthPx}, anchored top-left at ({@code x},{@code y}).
     */
    /* renamed from: a */
    public static final void drawWrappedBlock(Graphics graphics, int x, int y, int widthPx, int arg5, char[] chars, int offset, int arg7, int count) {
        graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        BitmapFont font = fontForColor(graphics.getColor());
        if (offset + count > chars.length) {
            count = chars.length - offset;
        }
        font.drawLines(graphics, wrapLines(new String(chars, offset, count), widthPx), x, y, defpackage.BaseCanvas.height, 20);
    }

    /**
     * Like {@link #drawWrappedBlock} but horizontally centred (anchor 17).
     */
    /* renamed from: b */
    public static final void drawWrappedBlockCentered(Graphics graphics, int x, int y, int widthPx, int arg5, char[] chars, int offset, int arg7, int count) {
        graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        BitmapFont font = fontForColor(graphics.getColor());
        if (offset + count > chars.length) {
            count = chars.length - offset;
        }
        font.drawLines(graphics, wrapLines(new String(chars, offset, count), widthPx), x, y, defpackage.BaseCanvas.height, 17);
    }

    /**
     * Draws up to the first three wrapped lines of {@code chars} with a typewriter
     * reveal: {@code shown} counts the characters still to reveal, so the line that
     * runs out is clipped mid-string and drawing stops there.
     *
     * @param shown number of characters to reveal across the visible lines
     */
    /* renamed from: c */
    public static final void drawWrappedBlockPartial(Graphics graphics, int x, int y, int widthPx, int arg5, char[] chars, int offset, int arg7, int shown) {
        graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        BitmapFont font = fontForColor(graphics.getColor());
        Vector lines = wrapLines(new String(chars, offset, chars.length - offset), widthPx);
        int lineMax = Math.min(lines.size(), 3);
        for (int li = 0; li < lineMax; li++) {
            String line = (String) lines.elementAt(li);
            if (shown <= line.length()) {
                font.drawString(graphics, line, 0, shown, x, y, 20);
                return;
            }
            font.drawString(graphics, line, x, y, 20);
            shown -= line.length() + 1;
            y += ((BitmapFont) currentFont).lineHeight + 2;
        }
        graphics.setColor(16777215);
    }

    /**
     * Draws all wrapped lines of {@code chars} (wrapped to {@code widthPx}) at
     * ({@code x},{@code y}) with the given anchor.
     *
     * @return the value returned by the underlying font's line drawing
     */
    /* renamed from: a */
    public static final int drawWrappedText(Graphics graphics, int x, int y, int widthPx, int arg5, char[] chars, int anchor) {
        graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        return fontForColor(graphics.getColor()).drawLines(graphics, wrapLines(charsToString(chars), widthPx), x, y, defpackage.BaseCanvas.height, anchor);
    }

    /**
     * Convenience overload of {@link #drawWrappedText} with a top-left anchor (20).
     */
    /* renamed from: a */
    public static final int drawWrappedText(Graphics graphics, int x, int y, int widthPx, int arg5, char[] chars) {
        return drawWrappedText(graphics, x, y, widthPx, arg5, chars, 20);
    }

    /**
     * @return {@code chars} as a {@link String}
     */
    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /* renamed from: a */
    public static final String charsToString(char[] chars) {
        return new String(chars);
    }

    /**
     * Replaces every occurrence of {@code find} in {@code source} with {@code replacement}.
     */
    /* renamed from: a */
    public static final String replaceAll(String source, String find, String replacement) {
        while (true) {
            int index = source.indexOf(find);
            if (index < 0) {
                return source;
            }
            String head = source.substring(0, index);
            source = new StringBuffer().append(head).append(replacement).append(source.substring(index + find.length())).toString();
        }
    }

    /**
     * Loads an image resource from the active locale's folder,
     * i.e. {@code "/<locale>/<path>"}.
     *
     * @param path image path relative to the locale folder
     * @return the loaded image
     */
    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /* renamed from: a */
    public static final Image loadLocaleImage(String path) throws IOException {
        return Image.createImage(new StringBuffer().append("/").append(StringTable.instance.locales[StringTable.instance.localeIndex]).append("/").append(path).toString());
    }
}
