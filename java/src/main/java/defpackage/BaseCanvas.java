package defpackage;

import javax.microedition.lcdui.Canvas;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/* renamed from: r */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:r.class */
/**
 * Shared full-screen {@link Canvas} base for {@link TitleScreen} and
 * {@link GameScreen}. Owns the cached screen dimensions, the pending-key
 * latch, the small bitmap-number/label drawing helpers, and the animated
 * asset-loading screen with its cooperative {@link #yieldTick()} throttle.
 */
public abstract class BaseCanvas extends Canvas {
    /** True while a key is held (cleared by {@link #flushKey()}). */
    public boolean keyDown = false;
    /** Key code awaiting a synthetic release, or 0 when none. */
    public int pendingKey = 0;
    /** Screen width in pixels. */
    public static int width;
    /** Screen height in pixels. */
    public static int height;
    /** Half of {@link #width}. */
    public static int halfW;
    /** Half of {@link #height}. */
    public static int halfH;

    /* renamed from: b */
    /** When set, the next {@link #beginLoading} keeps the current progress bar. */
    public static boolean keepLoadingProgress = false;
    /** Current loading progress (also advanced by {@link #yieldTick()}). */
    public static int loadProgress = 0;
    /** Loading progress denominator. */
    public static int loadTotal = 100;
    /** Loading-screen animation phase. */
    public static int loadPhase = 0;

    public BaseCanvas() {
        System.out.println("MyGameCanvas");
        setFullScreenMode(true);
        width = getWidth();
        halfW = width / 2;
    }

    /** Clears the held-key state and delivers a pending release. */
    public final void flushKey() {
        this.keyDown = false;
        if (this.pendingKey != 0) {
            keyReleased(this.pendingKey);
            this.pendingKey = 0;
        }
    }

    /** Draws {@code number} at ({@code x},{@code y}) with the default glyph set. */
    public static final void drawNumberAt(Graphics graphics, int number, int x, int y, int anchor) {
        drawNumber(graphics, number, x, y, anchor, 0);
    }

    /**
     * Draws {@code number} right-to-left using bitmap digit glyphs. {@code style}
     * selects the glyph sheet/size; {@code anchor} bit 1 centers and bit 8 right-
     * aligns the run.
     */
    public static final void drawNumber(Graphics graphics, int number, int x, int y, int anchor, int style) {
        byte[] digits = new byte[9];
        byte glyphWidth = 0;
        byte glyphAdvance = 0;
        int glyphHeight = 0;
        Image glyphSheet = null;
        switch (style) {
            case 0:
                glyphWidth = 5;
                glyphAdvance = 4;
                glyphHeight = 7;
                glyphSheet = AssetCache.numberFont0;
                break;
            case 1:
                glyphWidth = 7;
                glyphAdvance = 6;
                glyphHeight = 9;
                glyphSheet = AssetCache.numberFont1;
                break;
            case 2:
                glyphWidth = 7;
                glyphAdvance = 6;
                glyphHeight = 9;
                glyphSheet = AssetCache.numberFont2;
                break;
            case 3:
                glyphWidth = 9;
                glyphAdvance = 8;
                glyphHeight = 14;
                glyphSheet = AssetCache.numberFont3;
                break;
            case 4:
                glyphWidth = 9;
                glyphAdvance = 8;
                glyphHeight = 14;
                glyphSheet = AssetCache.numberFont4;
                break;
        }
        int clipX = graphics.getClipX();
        int clipY = graphics.getClipY();
        int clipWidth = graphics.getClipWidth();
        int clipHeight = graphics.getClipHeight();
        byte digitCount = 0;
        do {
            byte digit = (byte) (number % 10);
            number /= 10;
            byte slot = digitCount;
            digitCount = (byte) (digitCount + 1);
            digits[slot] = digit;
        } while (number != 0);
        int startX = x;
        if ((anchor | 1) == anchor) {
            startX -= (digitCount * glyphAdvance) / 2;
        } else if ((anchor | 8) == anchor) {
            startX -= digitCount * glyphAdvance;
        }
        for (int i = 0; i < digitCount; i++) {
            if (style != 0) {
                GameScreen.clipToWorld(graphics, startX + (i * glyphAdvance), y, glyphWidth, glyphHeight);
            } else {
                graphics.setClip(startX + (i * glyphAdvance), y, glyphWidth, glyphHeight);
            }
            graphics.drawImage(glyphSheet, (startX + (i * glyphAdvance)) - (digits[(digitCount - i) - 1] * glyphWidth), y, 20);
        }
        graphics.setClip(clipX, clipY, clipWidth, clipHeight);
    }

    /** Draws "{@code numerator} / {@code denominator}" ending at ({@code x},{@code y}). */
    public static final void drawFraction(Graphics graphics, int x, int y, int numerator, int denominator) {
        drawNumberAt(graphics, denominator, x, y, 8);
        int denominatorWidth = numberWidth(denominator);
        graphics.drawImage(AssetCache.fractionSlash, x - denominatorWidth, y, 24);
        drawNumberAt(graphics, numerator, (x - denominatorWidth) - 9, y, 8);
    }

    /** Returns the rendered pixel width of {@code value} as bitmap digits. */
    public static final int numberWidth(int value) {
        int pixelWidth = 1;
        do {
            value /= 10;
            pixelWidth += 4;
        } while (value != 0);
        return pixelWidth;
    }

    /** Draws {@code text} in a black box at ({@code x},{@code y}); returns right edge. */
    public static final int drawLabelBox(Graphics graphics, String text, int x, int y) {
        return drawLabelBox(graphics, text.toCharArray(), x, y);
    }

    /** Draws {@code text} in a black box at ({@code x},{@code y}); returns right edge. */
    public static final int drawLabelBox(Graphics graphics, char[] text, int x, int y) {
        int boxWidth = FontManager.stringWidth(text) + 2;
        int boxHeight = FontManager.lineHeight() + 2;
        graphics.setColor(0);
        graphics.fillRect(x - 1, y - 1, boxWidth, boxHeight);
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, x, y, text, 1);
        return x + boxWidth;
    }

    /** Renders the animated asset-loading screen for the current phase. */
    public static final void drawLoadingScreen(Graphics graphics) {
        if (loadPhase >= 3) {
            graphics.setColor(0);
            graphics.fillRect(0, 0, width, height);
            graphics.setColor(14663551);
            FontManager.drawChars(graphics, halfW - 48, halfH - 12, FontManager.loadingTitle, 0);
            graphics.drawLine(halfW - 50, halfH, halfW + 48, halfH);
            graphics.fillRect(halfW - 51, halfH + 1, 2, 2);
            graphics.fillRect(halfW + 48, halfH + 1, 2, 2);
            graphics.setColor(10452799);
            graphics.drawLine(halfW - 50, halfH + 5, halfW + 48, halfH + 5);
            graphics.fillRect(halfW - 51, halfH + 3, 2, 2);
            graphics.fillRect(halfW + 48, halfH + 3, 2, 2);
            char[] subtitle = FontManager.loadingSubtitle;
            FontManager.drawChars(graphics, halfW - (FontManager.stringWidth(subtitle) / 2), halfH + 50, subtitle, 0);
        }
        if (loadPhase > 3) {
            graphics.setColor(0);
            graphics.fillRect(halfW + 20, halfH - 12, 18, 10);
            graphics.setColor(14663551);
            FontManager.drawChars(graphics, halfW + 20, halfH - 12, "...".substring(0, loadPhase % 4).toCharArray(), 0);
            graphics.setColor(14655295);
            graphics.fillRect(halfW - 48, halfH + 2, (95 * (loadProgress < loadTotal ? loadProgress : loadTotal)) / loadTotal, 1);
            graphics.setColor(16777087);
            graphics.fillRect(halfW - 48, halfH + 3, (95 * (loadProgress < loadTotal ? loadProgress : loadTotal)) / loadTotal, 1);
        } else if (loadPhase < 3) {
            graphics.setColor(0);
            int barCount = (height + 11) / 12;
            for (int i = 0; i < barCount; i++) {
                graphics.fillRect(0, (i * 12) + (loadPhase * 4), width, 4);
            }
        }
        loadPhase++;
    }

    /** Begins (or resumes) a loading screen expecting {@code total} progress units. */
    public static final void beginLoading(String label, int total) {
        GameLoop.instance.setLoadingFps();
        if (keepLoadingProgress) {
            if (loadPhase < 3) {
                loadPhase = 3;
            }
            keepLoadingProgress = false;
        } else {
            loadTotal = total;
            loadProgress = 0;
            loadPhase = 0;
        }
    }

    /** Requests an async repaint of this canvas. */
    public final void requestRepaint() {
        repaint();
    }

    /* renamed from: a */
    /** Sets the playfield height (clamped below 350) and recomputes {@link #halfH}. */
    public final void setViewHeight(int newHeight) {
        if (newHeight < 0 || newHeight < 350) {
            height = newHeight;
        } else {
            height = getHeight();
        }
        halfH = height / 2;
    }

    /** Advances loading progress and sleeps every sixth call to yield the CPU. */
    public static final void yieldTick() {
        loadProgress++;
        if (loadProgress % 6 == 0) {
            try {
                Thread.sleep(50L);
            } catch (InterruptedException unused) {
            }
        }
    }

    static {
        int[] iArr = {5, 5, 5, 5, 5, 5, 5, 5, 3, 4, 5, 5, 7, 5, 5, 5, 5, 5, 5, 5, 5, 5, 7, 5, 5, 5};
        int[] iArr2 = {0, 4, 8, 12, 16, 20, 24, 28, 32, 34, 37, 41, 45, 51, 55, 59, 63, 67, 71, 75, 79, 83, 87, 93, 97, 101};
    }
}
