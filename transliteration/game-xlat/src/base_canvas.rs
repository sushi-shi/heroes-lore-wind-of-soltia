//! Transliterated from `java/src/main/java/defpackage/BaseCanvas.java`
//! (original `r.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The shared full-screen `Canvas` base of `TitleScreen`/`GameScreen`. This
//! increment ports the pieces the FIRST-FRAME title render path touches — the
//! cached screen geometry (`width`/`halfW` at construction, `height`/`halfH` via
//! `setViewHeight`), the per-frame `flushKey`, `requestRepaint`, and the
//! cooperative `yieldTick` — **plus** the previously-deferred animated
//! loading-screen number/label kit: the bitmap-digit helpers (`drawNumberAt`,
//! `drawNumber`, `drawFraction`, `numberWidth`), the black-box label draw
//! (`drawLabelBox`, both overloads), the phased loading screen
//! (`drawLoadingScreen`) and its `beginLoading` (re)set.
//!
//! ## Cross-owner reads (snapshot params)
//!
//! `drawNumber`/`drawFraction` draw with the `AssetCache.numberFont0..4` /
//! `AssetCache.fractionSlash` glyph sheets and `drawLoadingScreen` draws the
//! `FontManager.loadingTitle` / `loadingSubtitle` labels — all statics owned by the
//! still-PARTIAL `AssetCache` (`ce`) / `FontManager` (`bh`) banks, not modelled
//! here. Per `docs/TRANSLITERATION.md` (a cross-owner read becomes an explicit
//! snapshot param, not a second borrow), those `Image`/`char[]` statics are threaded
//! in as parameters (`glyph_sheet`, `fraction_slash`, `loading_title`,
//! `loading_subtitle`); the caller snapshots the (unported) bank. `drawNumber`'s
//! `style != 0` clip uses `GameScreen.clipToWorld` (`as.class`), inlined here from
//! the already-modelled `GameScreen.worldHeight` static (threaded as `world_height`)
//! — its whole body is `if (y + h > worldHeight) h = worldHeight - y; setClip(...)`.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `r.a:(I)V => [idiv]`
//! (setViewHeight — the one `height / 2`), `r.i:()V => []` (flushKey),
//! `r.j:()V => []` (requestRepaint), `r.k:()V => []` (yieldTick — `loadProgress++`
//! is an `iinc`, not an arithmetic opcode).

use crate::font_manager::{self, FontManagerState};
use crate::game::Game;
use crate::game_loop;
use j2me_jvm::{java_div, java_rem};

/// The reference device resolution (the v207 build's captured frames are
/// 240×320; `getWidth()`/`getHeight()` return these). A device fact, kept here as
/// the single source used by `TitleScreen`'s `Canvas`/framebuffer construction.
pub const DEVICE_WIDTH: i32 = 240;
/// See [`DEVICE_WIDTH`].
pub const DEVICE_HEIGHT: i32 = 320;

/// Java `r` / `BaseCanvas` state: the two instance fields of the shown canvas
/// (`keyDown`, `pendingKey`) plus the class `static` geometry / loading counters
/// (see `java/reconstruction/ownership.tsv`), in reviewed declaration order.
#[derive(Debug)]
pub struct BaseCanvasState {
    /// `public boolean keyDown = false;` (instance).
    pub key_down: bool,
    /// `public int pendingKey = 0;` (instance).
    pub pending_key: i32,
    /// `public static int width;`
    pub width: i32,
    /// `public static int height;`
    pub height: i32,
    /// `public static int halfW;`
    pub half_w: i32,
    /// `public static int halfH;`
    pub half_h: i32,
    /// `public static boolean keepLoadingProgress = false;`
    pub keep_loading_progress: bool,
    /// `public static int loadProgress = 0;`
    pub load_progress: i32,
    /// `public static int loadTotal = 100;`
    pub load_total: i32,
    /// `public static int loadPhase = 0;`
    pub load_phase: i32,
}

impl Default for BaseCanvasState {
    fn default() -> Self {
        // `r.<clinit>` allocates two dead local `int[]` tables and discards them
        // (no field write, no side effect) — a no-op. The class's field
        // initializers set `keepLoadingProgress=false`, `loadProgress=0`,
        // `loadTotal=100`, `loadPhase=0`; `width`/`height`/`halfW`/`halfH` are the
        // JVM default 0 until the `BaseCanvas()` constructor / `setViewHeight`.
        BaseCanvasState {
            key_down: false,
            pending_key: 0,
            width: 0,
            height: 0,
            half_w: 0,
            half_h: 0,
            keep_loading_progress: false,
            load_progress: 0,
            load_total: 100,
            load_phase: 0,
        }
    }
}

/// `public BaseCanvas()` — the constructor's observable geometry writes:
/// `setFullScreenMode(true); width = getWidth(); halfW = width / 2;`. The
/// `System.out.println("MyGameCanvas")` is dropped. Called from
/// `TitleScreen`'s constructor after its `Canvas` is materialised.
pub fn construct(g: &mut Game) {
    // setFullScreenMode(true);
    if let Some(canvas) = g.canvas.as_mut() {
        canvas.set_full_screen_mode(true);
    }
    // width = getWidth();
    g.base_canvas.width = g
        .canvas
        .as_ref()
        .map(|c| c.width())
        .expect("BaseCanvas() requires a constructed Canvas");
    // halfW = width / 2;
    g.base_canvas.half_w = java_div(g.base_canvas.width, 2).expect("width / 2");
}

/// `public final void flushKey()` (`r.i:()V => []`): clears the held-key latch and
/// delivers a pending synthetic release. `pendingKey` is 0 on the title path, so
/// no `keyReleased` fires.
pub fn flush_key(g: &mut Game) {
    // this.keyDown = false;
    g.base_canvas.key_down = false;
    // if (this.pendingKey != 0) { keyReleased(this.pendingKey); this.pendingKey = 0; }
    if g.base_canvas.pending_key != 0 {
        // keyReleased(pendingKey) — TitleScreen/BaseCanvas do not override it; the
        // Canvas default is empty. DEFERRED (not reached: pendingKey == 0 here).
        g.base_canvas.pending_key = 0;
    }
}

/// `public static final void drawNumberAt(Graphics graphics, int number, int x, int y, int anchor)`:
/// `drawNumber(graphics, number, x, y, anchor, 0)` — the default (style-0) glyph
/// set. `glyph_sheet` is the `AssetCache.numberFont0` snapshot (see the module
/// header); style 0 never reaches `clipToWorld`, so no `world_height` is needed.
pub fn draw_number_at(
    graphics: &mut j2me_me::Graphics,
    number: i32,
    x: i32,
    y: i32,
    anchor: i32,
    glyph_sheet: &j2me_me::Image,
) {
    // drawNumber(graphics, number, x, y, anchor, 0);
    //   style 0 takes the `graphics.setClip` else-branch, so `world_height` (the
    //   GameScreen.clipToWorld snapshot) is never read — pass 0.
    draw_number(graphics, number, x, y, anchor, 0, glyph_sheet, 0);
}

/// `public static final void drawNumber(Graphics graphics, int number, int x, int y, int anchor, int style)`:
/// draws `number` right-to-left in bitmap digit glyphs. `style` selects the glyph
/// metrics (and, on the real device, the `AssetCache.numberFont{style}` sheet — here
/// the `glyph_sheet` snapshot); `anchor` bit 1 centres and bit 8 right-aligns the run.
/// `world_height` is the `GameScreen.worldHeight` snapshot for the `style != 0`
/// `clipToWorld`.
#[allow(clippy::too_many_arguments)]
pub fn draw_number(
    graphics: &mut j2me_me::Graphics,
    mut number: i32,
    x: i32,
    y: i32,
    anchor: i32,
    style: i32,
    glyph_sheet: &j2me_me::Image,
    world_height: i32,
) {
    // byte[] digits = new byte[9];
    let mut digits: Vec<i8> = vec![0i8; 9];
    // byte glyphWidth = 0; byte glyphAdvance = 0; int glyphHeight = 0; Image glyphSheet = null;
    let mut glyph_width: i8 = 0;
    let mut glyph_advance: i8 = 0;
    let mut glyph_height: i32 = 0;
    // switch (style) — the metrics; `glyphSheet = AssetCache.numberFont{style}` is
    // the `glyph_sheet` snapshot param (the numberFont banks are unported).
    match style {
        0 => {
            glyph_width = 5;
            glyph_advance = 4;
            glyph_height = 7;
        }
        1 => {
            glyph_width = 7;
            glyph_advance = 6;
            glyph_height = 9;
        }
        2 => {
            glyph_width = 7;
            glyph_advance = 6;
            glyph_height = 9;
        }
        3 => {
            glyph_width = 9;
            glyph_advance = 8;
            glyph_height = 14;
        }
        4 => {
            glyph_width = 9;
            glyph_advance = 8;
            glyph_height = 14;
        }
        _ => {}
    }
    // int clipX = graphics.getClipX(); clipY; clipWidth; clipHeight;
    let clip_x: i32 = graphics.clip_x();
    let clip_y: i32 = graphics.clip_y();
    let clip_width: i32 = graphics.clip_width();
    let clip_height: i32 = graphics.clip_height();
    // byte digitCount = 0;
    let mut digit_count: i8 = 0;
    // do { digit = (byte)(number % 10); number /= 10; slot = digitCount;
    //      digitCount = (byte)(digitCount + 1); digits[slot] = digit; } while (number != 0);
    loop {
        let digit: i8 = java_rem(number, 10).expect("number % 10") as i8;
        number = java_div(number, 10).expect("number / 10");
        let slot: i8 = digit_count;
        digit_count = (digit_count as i32).wrapping_add(1) as i8;
        digits[slot as usize] = digit;
        if number == 0 {
            break;
        }
    }
    // int startX = x;
    let mut start_x: i32 = x;
    // if ((anchor | 1) == anchor) startX -= (digitCount * glyphAdvance) / 2;
    if (anchor | 1) == anchor {
        start_x = start_x.wrapping_sub(
            java_div((digit_count as i32).wrapping_mul(glyph_advance as i32), 2)
                .expect("(digitCount * glyphAdvance) / 2"),
        );
    // else if ((anchor | 8) == anchor) startX -= digitCount * glyphAdvance;
    } else if (anchor | 8) == anchor {
        start_x = start_x.wrapping_sub((digit_count as i32).wrapping_mul(glyph_advance as i32));
    }
    // for (int i = 0; i < digitCount; i++) { ... }
    let mut i: i32 = 0;
    while i < digit_count as i32 {
        let window_x: i32 = start_x.wrapping_add(i.wrapping_mul(glyph_advance as i32));
        if style != 0 {
            // GameScreen.clipToWorld(graphics, window_x, y, glyphWidth, glyphHeight);
            //   inlined (as.class): if (y + h > worldHeight) h = worldHeight - y; setClip(...)
            let mut h: i32 = glyph_height;
            if y.wrapping_add(h) > world_height {
                h = world_height.wrapping_sub(y);
            }
            graphics.set_clip(window_x, y, glyph_width as i32, h);
        } else {
            // graphics.setClip(window_x, y, glyphWidth, glyphHeight);
            graphics.set_clip(window_x, y, glyph_width as i32, glyph_height);
        }
        // graphics.drawImage(glyphSheet, window_x - (digits[(digitCount - i) - 1] * glyphWidth), y, 20);
        let digit: i8 = digits[((digit_count as i32).wrapping_sub(i).wrapping_sub(1)) as usize];
        let draw_x: i32 = window_x.wrapping_sub((digit as i32).wrapping_mul(glyph_width as i32));
        graphics
            .draw_image(glyph_sheet, draw_x, y, 20)
            .expect("drawImage(numberFont glyph)");
        i = i.wrapping_add(1);
    }
    // graphics.setClip(clipX, clipY, clipWidth, clipHeight);
    graphics.set_clip(clip_x, clip_y, clip_width, clip_height);
}

/// `public static final void drawFraction(Graphics graphics, int x, int y, int numerator, int denominator)`:
/// draws "`numerator` / `denominator`" ending at (`x`,`y`). `glyph_sheet` is the
/// `AssetCache.numberFont0` snapshot; `fraction_slash` is the `AssetCache.fractionSlash`
/// snapshot (both unported banks — see the module header).
pub fn draw_fraction(
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    numerator: i32,
    denominator: i32,
    glyph_sheet: &j2me_me::Image,
    fraction_slash: &j2me_me::Image,
) {
    // drawNumberAt(graphics, denominator, x, y, 8);
    draw_number_at(graphics, denominator, x, y, 8, glyph_sheet);
    // int denominatorWidth = numberWidth(denominator);
    let denominator_width: i32 = number_width(denominator);
    // graphics.drawImage(AssetCache.fractionSlash, x - denominatorWidth, y, 24);
    graphics
        .draw_image(fraction_slash, x.wrapping_sub(denominator_width), y, 24)
        .expect("drawImage(fractionSlash)");
    // drawNumberAt(graphics, numerator, (x - denominatorWidth) - 9, y, 8);
    draw_number_at(
        graphics,
        numerator,
        x.wrapping_sub(denominator_width).wrapping_sub(9),
        y,
        8,
        glyph_sheet,
    );
}

/// `public static final int numberWidth(int value)`: the rendered pixel width of
/// `value` as bitmap digits (1 + 4 per digit). Pure arithmetic.
pub fn number_width(mut value: i32) -> i32 {
    // int pixelWidth = 1;
    let mut pixel_width: i32 = 1;
    // do { value /= 10; pixelWidth += 4; } while (value != 0);
    loop {
        value = java_div(value, 10).expect("value / 10");
        pixel_width = pixel_width.wrapping_add(4);
        if value == 0 {
            break;
        }
    }
    // return pixelWidth;
    pixel_width
}

/// `public static final int drawLabelBox(Graphics graphics, String text, int x, int y)`:
/// the `String` overload — `drawLabelBox(graphics, text.toCharArray(), x, y)`.
pub fn draw_label_box_string(
    fm: &FontManagerState,
    graphics: &mut j2me_me::Graphics,
    text: &str,
    x: i32,
    y: i32,
) -> i32 {
    // return drawLabelBox(graphics, text.toCharArray(), x, y);
    let chars: Vec<u16> = text.encode_utf16().collect();
    draw_label_box(fm, graphics, &chars, x, y)
}

/// `public static final int drawLabelBox(Graphics graphics, char[] text, int x, int y)`:
/// draws `text` in a white-on-black box at (`x`,`y`); returns the box's right edge.
/// `fm` is the `FontManager` (`bh`) state the width/height/draw read.
pub fn draw_label_box(
    fm: &FontManagerState,
    graphics: &mut j2me_me::Graphics,
    text: &[u16],
    x: i32,
    y: i32,
) -> i32 {
    // int boxWidth = FontManager.stringWidth(text) + 2;
    let box_width: i32 = font_manager::string_width(fm, text).wrapping_add(2);
    // int boxHeight = FontManager.lineHeight() + 2;
    let box_height: i32 = font_manager::line_height(fm).wrapping_add(2);
    // graphics.setColor(0);
    graphics.set_color(0);
    // graphics.fillRect(x - 1, y - 1, boxWidth, boxHeight);
    graphics.fill_rect(x.wrapping_sub(1), y.wrapping_sub(1), box_width, box_height);
    // graphics.setColor(16777215);
    graphics.set_color(16777215);
    // FontManager.drawChars(graphics, x, y, text, 1);
    font_manager::draw_chars(fm, graphics, x, y, text, 1);
    // return x + boxWidth;
    x.wrapping_add(box_width)
}

/// `public static final void drawLoadingScreen(Graphics graphics)`: renders the
/// animated asset-loading screen for the current `loadPhase`, then advances it.
/// `loading_title` / `loading_subtitle` are the `FontManager.loadingTitle` /
/// `loadingSubtitle` snapshots (unported labels — see the module header).
pub fn draw_loading_screen(g: &mut Game, loading_title: &[u16], loading_subtitle: &[u16]) {
    // Disjoint field borrows: the framebuffer (mut, via Graphics), the fonts (read),
    // and the geometry / loading counters (read; loadPhase written at the end).
    let Game {
        screen,
        font_manager,
        base_canvas,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // if (loadPhase >= 3) { ... }
    if base_canvas.load_phase >= 3 {
        // graphics.setColor(0); graphics.fillRect(0, 0, width, height);
        graphics.set_color(0);
        graphics.fill_rect(0, 0, base_canvas.width, base_canvas.height);
        // graphics.setColor(14663551);
        graphics.set_color(14663551);
        // FontManager.drawChars(graphics, halfW - 48, halfH - 12, FontManager.loadingTitle, 0);
        font_manager::draw_chars(
            font_manager,
            &mut graphics,
            base_canvas.half_w.wrapping_sub(48),
            base_canvas.half_h.wrapping_sub(12),
            loading_title,
            0,
        );
        // graphics.drawLine(halfW - 50, halfH, halfW + 48, halfH);
        graphics.draw_line(
            base_canvas.half_w.wrapping_sub(50),
            base_canvas.half_h,
            base_canvas.half_w.wrapping_add(48),
            base_canvas.half_h,
        );
        // graphics.fillRect(halfW - 51, halfH + 1, 2, 2);
        graphics.fill_rect(
            base_canvas.half_w.wrapping_sub(51),
            base_canvas.half_h.wrapping_add(1),
            2,
            2,
        );
        // graphics.fillRect(halfW + 48, halfH + 1, 2, 2);
        graphics.fill_rect(
            base_canvas.half_w.wrapping_add(48),
            base_canvas.half_h.wrapping_add(1),
            2,
            2,
        );
        // graphics.setColor(10452799);
        graphics.set_color(10452799);
        // graphics.drawLine(halfW - 50, halfH + 5, halfW + 48, halfH + 5);
        graphics.draw_line(
            base_canvas.half_w.wrapping_sub(50),
            base_canvas.half_h.wrapping_add(5),
            base_canvas.half_w.wrapping_add(48),
            base_canvas.half_h.wrapping_add(5),
        );
        // graphics.fillRect(halfW - 51, halfH + 3, 2, 2);
        graphics.fill_rect(
            base_canvas.half_w.wrapping_sub(51),
            base_canvas.half_h.wrapping_add(3),
            2,
            2,
        );
        // graphics.fillRect(halfW + 48, halfH + 3, 2, 2);
        graphics.fill_rect(
            base_canvas.half_w.wrapping_add(48),
            base_canvas.half_h.wrapping_add(3),
            2,
            2,
        );
        // char[] subtitle = FontManager.loadingSubtitle;
        let subtitle: &[u16] = loading_subtitle;
        // FontManager.drawChars(graphics, halfW - (FontManager.stringWidth(subtitle) / 2), halfH + 50, subtitle, 0);
        let subtitle_x: i32 = base_canvas.half_w.wrapping_sub(
            java_div(font_manager::string_width(font_manager, subtitle), 2)
                .expect("stringWidth(subtitle) / 2"),
        );
        font_manager::draw_chars(
            font_manager,
            &mut graphics,
            subtitle_x,
            base_canvas.half_h.wrapping_add(50),
            subtitle,
            0,
        );
    }
    // if (loadPhase > 3) { ... } else if (loadPhase < 3) { ... }
    if base_canvas.load_phase > 3 {
        // graphics.setColor(0); graphics.fillRect(halfW + 20, halfH - 12, 18, 10);
        graphics.set_color(0);
        graphics.fill_rect(
            base_canvas.half_w.wrapping_add(20),
            base_canvas.half_h.wrapping_sub(12),
            18,
            10,
        );
        // graphics.setColor(14663551);
        graphics.set_color(14663551);
        // graphics.drawChars(graphics, halfW + 20, halfH - 12, "...".substring(0, loadPhase % 4).toCharArray(), 0);
        //   "..." is three dots; substring(0, n) with n = loadPhase % 4 in 0..3.
        let dot_count: i32 = java_rem(base_canvas.load_phase, 4).expect("loadPhase % 4");
        let dots: Vec<u16> = "..."
            .encode_utf16()
            .take(dot_count as usize)
            .collect::<Vec<u16>>();
        font_manager::draw_chars(
            font_manager,
            &mut graphics,
            base_canvas.half_w.wrapping_add(20),
            base_canvas.half_h.wrapping_sub(12),
            &dots,
            0,
        );
        // graphics.setColor(14655295);
        graphics.set_color(14655295);
        // int fill = (95 * (loadProgress < loadTotal ? loadProgress : loadTotal)) / loadTotal;
        let clamped: i32 = if base_canvas.load_progress < base_canvas.load_total {
            base_canvas.load_progress
        } else {
            base_canvas.load_total
        };
        let fill: i32 = java_div(95i32.wrapping_mul(clamped), base_canvas.load_total)
            .expect("(95 * clamp) / loadTotal");
        // graphics.fillRect(halfW - 48, halfH + 2, fill, 1);
        graphics.fill_rect(
            base_canvas.half_w.wrapping_sub(48),
            base_canvas.half_h.wrapping_add(2),
            fill,
            1,
        );
        // graphics.setColor(16777087);
        graphics.set_color(16777087);
        // graphics.fillRect(halfW - 48, halfH + 3, fill, 1);
        graphics.fill_rect(
            base_canvas.half_w.wrapping_sub(48),
            base_canvas.half_h.wrapping_add(3),
            fill,
            1,
        );
    } else if base_canvas.load_phase < 3 {
        // graphics.setColor(0);
        graphics.set_color(0);
        // int barCount = (height + 11) / 12;
        let bar_count: i32 =
            java_div(base_canvas.height.wrapping_add(11), 12).expect("(height + 11) / 12");
        // for (int i = 0; i < barCount; i++) graphics.fillRect(0, (i * 12) + (loadPhase * 4), width, 4);
        let mut i: i32 = 0;
        while i < bar_count {
            graphics.fill_rect(
                0,
                i.wrapping_mul(12)
                    .wrapping_add(base_canvas.load_phase.wrapping_mul(4)),
                base_canvas.width,
                4,
            );
            i = i.wrapping_add(1);
        }
    }
    // loadPhase++;
    base_canvas.load_phase = base_canvas.load_phase.wrapping_add(1);
}

/// `public static final void beginLoading(String label, int total)`: begins (or, when
/// `keepLoadingProgress` is set, resumes) a loading screen expecting `total` progress
/// units. The `label` argument is unused in the shipped body (dropped). Drops to the
/// 5-FPS loading frame rate via `GameLoop.instance.setLoadingFps()`.
pub fn begin_loading(g: &mut Game, _label: &str, total: i32) {
    // GameLoop.instance.setLoadingFps();
    game_loop::set_loading_fps(g);
    // if (keepLoadingProgress) { if (loadPhase < 3) loadPhase = 3; keepLoadingProgress = false; }
    if g.base_canvas.keep_loading_progress {
        if g.base_canvas.load_phase < 3 {
            g.base_canvas.load_phase = 3;
        }
        g.base_canvas.keep_loading_progress = false;
    } else {
        // else { loadTotal = total; loadProgress = 0; loadPhase = 0; }
        g.base_canvas.load_total = total;
        g.base_canvas.load_progress = 0;
        g.base_canvas.load_phase = 0;
    }
}

/// `public final void requestRepaint()` (`r.j:()V => []`): `repaint()` — schedules
/// an async repaint of the shown canvas (arms the `j2me-me` repaint latch).
pub fn request_repaint(g: &mut Game) {
    // repaint();
    if let Some(canvas) = g.canvas.as_mut() {
        canvas.request_repaint();
    }
}

/// `public final void setViewHeight(int newHeight)` (`r.a:(I)V => [idiv]`): sets the
/// playfield height (below 350 keeps `newHeight`, else the full screen height) and
/// recomputes `halfH`.
pub fn set_view_height(g: &mut Game, new_height: i32) {
    // if (newHeight < 0 || newHeight < 350) height = newHeight; else height = getHeight();
    if new_height < 0 || new_height < 350 {
        g.base_canvas.height = new_height;
    } else {
        // getHeight()
        g.base_canvas.height = g
            .canvas
            .as_ref()
            .map(|c| c.height())
            .unwrap_or(DEVICE_HEIGHT);
    }
    // halfH = height / 2;
    g.base_canvas.half_h = java_div(g.base_canvas.height, 2).expect("height / 2");
}

/// `public static final void yieldTick()` (`r.k:()V => []`): advances loading
/// progress and, every sixth call, sleeps to yield the CPU. The `Thread.sleep(50)`
/// is a no-op here (no observable state), preserving the `loadProgress`
/// increment/modulo.
pub fn yield_tick(g: &mut Game) {
    // loadProgress++;
    g.base_canvas.load_progress = g.base_canvas.load_progress.wrapping_add(1);
    // if (loadProgress % 6 == 0) { Thread.sleep(50L); }   — sleep is a no-op.
    if j2me_jvm::java_rem(g.base_canvas.load_progress, 6).expect("loadProgress % 6") == 0 {
        // Thread.sleep(50L) — no observable state; dropped.
    }
}
