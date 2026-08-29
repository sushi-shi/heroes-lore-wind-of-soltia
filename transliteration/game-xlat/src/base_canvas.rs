//! Transliterated from `java/src/main/java/defpackage/BaseCanvas.java`
//! (original `r.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The shared full-screen `Canvas` base of `TitleScreen`/`GameScreen`. This
//! increment ports the pieces the FIRST-FRAME title render path touches — the
//! cached screen geometry (`width`/`halfW` at construction, `height`/`halfH` via
//! `setViewHeight`), the per-frame `flushKey`, `requestRepaint`, and the
//! cooperative `yieldTick`. The bitmap-number/label drawing helpers and the
//! animated loading screen (`drawNumber`, `drawLoadingScreen`, `beginLoading`, …)
//! reach the not-yet-ported `FontManager`/`AssetCache` UI banks and are DEFERRED.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `r.a:(I)V => [idiv]`
//! (setViewHeight — the one `height / 2`), `r.i:()V => []` (flushKey),
//! `r.j:()V => []` (requestRepaint), `r.k:()V => []` (yieldTick — `loadProgress++`
//! is an `iinc`, not an arithmetic opcode).

use crate::game::Game;
use j2me_jvm::java_div;

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
