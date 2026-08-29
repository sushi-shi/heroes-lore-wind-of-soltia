//! Transliterated from `java/src/main/java/defpackage/GameLoop.java`
//! (original `bs.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The single-threaded game loop and settings owner. This increment ports only
//! the boot-entry construction path — `create` (the static factory), the private
//! constructor, and the FPS helpers + `start` it reaches. The frame loop
//! (`run`), the option pack/unpack + RMS persistence, and the screen transitions
//! are DEFERRED to later increments.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `bs.<clinit>:()V => []`, `bs.a:(Ljavax/microedition/lcdui/Display;)V => []`
//! (create), `bs.<init>:(Ljavax/microedition/lcdui/Display;)V => [ior, i2b]`,
//! `bs.a:(I)V => [idiv]` (setFps), `bs.f:()V => []` (applyDifficultyFps),
//! `bs.c:()V => []` (start).

use crate::base_canvas;
use crate::game::Game;
use crate::title_screen;
use j2me_jvm::{java_div, Clock};

/// Deferred cross-class static `EventScript.skip` (obf `...:Z`), read by
/// `throttle` as its fast-forward guard. Defaults to `false` (no cutscene
/// skip active on the title path). EventScript is not ported in this increment;
/// snapshotted per the contract's cross-owner-read convention.
const EVENT_SCRIPT_SKIP: bool = false;

/// Deferred cross-class static `Debug.fullVersion` (obf `x.a:Z`), read by the
/// GameLoop constructor as `!Debug.fullVersion`. Debug's `<clinit>` sets it to
/// `false` (`iconst_0 putstatic`); the sibling `byte[]{7,9,13,16,19}` allocation
/// and `System.currentTimeMillis()` in that `<clinit>` are discarded (a dead
/// static array + a dead call — no observable effect). Debug is not ported in
/// this increment; snapshotted per the contract's "cross-owner reads become
/// explicit snapshot params, not second borrows".
const DEBUG_FULL_VERSION: bool = false;

/// Deferred cross-class static `AudioManager.maxVolume` (obf `bw.a:I`), the
/// GameLoop field initializer `volume = AudioManager.maxVolume`. AudioManager's
/// `<clinit>` sets it to `10` (`bipush 10 putstatic`). AudioManager is not ported
/// in this increment; snapshotted.
const AUDIO_MANAGER_MAX_VOLUME: i32 = 10;

/// Java `bs` / `GameLoop` state — the singleton loop's instance fields plus its
/// statics, in reviewed declaration order. Reference-typed fields are presence
/// flags (`false` while the Java reference is null); the `Display` referent lives
/// on [`Game::display`].
#[derive(Debug)]
pub struct GameLoopState {
    /// `private Display display;` — reference to [`Game::display`] (null → false).
    pub display: bool,
    /// `private BaseCanvas current;` — the shown screen; BaseCanvas not ported
    /// (null → false). Set in the deferred `run()` / `showGameScreen()` path.
    pub current: bool,
    /// `public static GameScreen gameScreen;` — the live GameScreen or null; not
    /// ported (null → false).
    pub game_screen: bool,
    /// `private int frameDelay;` — ms frame delay for the current difficulty.
    pub frame_delay: i32,
    /// `private int frameTargetMs;` — target ms per frame (`1000 / fps`).
    pub frame_target_ms: i32,
    /// `public int volume = AudioManager.maxVolume;`
    pub volume: i32,
    /// `public boolean soundEnabled;`
    pub sound_enabled: bool,
    /// `public boolean hasCreatedCharacter;`
    pub has_created_character: bool,
    /// `public boolean autoTextAdvance;`
    pub auto_text_advance: bool,
    /// `public boolean cameraFollow;`
    pub camera_follow: bool,
    /// `public byte difficulty;` — 0..3, indexes `frameDelayTable`.
    pub difficulty: i8,
    /// `public byte progressFlags;`
    pub progress_flags: i8,
    /// `public int progressData;`
    pub progress_data: i32,
    /// `private long frameStartMs;`
    pub frame_start_ms: i64,
    /// `public boolean stopped;`
    pub stopped: bool,
    /// `private boolean bootPending;`
    pub boot_pending: bool,
    /// `public static GameLoop instance;` — the singleton self-reference
    /// (null → false).
    pub instance: bool,
    /// `public static final int[] frameDelayTable = {8, 10, 14, 18};`
    pub frame_delay_table: Vec<i32>,
    /// `public static Object lock = new Object();` — the frame monitor; a presence
    /// flag (the single-threaded transliteration does not model the monitor).
    pub lock: bool,
}

/// The body of `bs.<clinit>` (`bs.<clinit>:()V => []`): the class's static-field
/// initializers in JVM order. Shared by [`GameLoopState::default`] (eager, at
/// class-load) and [`class_init`] (the lazy trigger), so the array literal has a
/// single home.
fn clinit_apply(s: &mut GameLoopState) {
    // static final int[] frameDelayTable = {8, 10, 14, 18};
    s.frame_delay_table = vec![8, 10, 14, 18];
    // static Object lock = new Object();
    s.lock = true;
    // gameScreen, instance default null (already false).
}

impl Default for GameLoopState {
    fn default() -> Self {
        // Instance fields at their JVM defaults (0 / false / null) — a GameLoop
        // instance is materialised by `create()` / [`construct`], not at class
        // load; the static block runs via `clinit_apply`.
        let mut s = GameLoopState {
            display: false,
            current: false,
            game_screen: false,
            frame_delay: 0,
            frame_target_ms: 0,
            volume: 0,
            sound_enabled: false,
            has_created_character: false,
            auto_text_advance: false,
            camera_follow: false,
            difficulty: 0,
            progress_flags: 0,
            progress_data: 0,
            frame_start_ms: 0,
            stopped: false,
            boot_pending: false,
            instance: false,
            frame_delay_table: Vec::new(),
            lock: false,
        };
        clinit_apply(&mut s);
        s
    }
}

/// `bs.<clinit>` at its JVM trigger point — the first active use of `GameLoop`,
/// which on the boot path is the static `create()` call. Idempotent (guarded by
/// [`Game::game_loop_class_initialized`]).
pub fn class_init(g: &mut Game) {
    if g.game_loop_class_initialized {
        return;
    }
    g.game_loop_class_initialized = true;
    clinit_apply(&mut g.game_loop);
}

/// `public static final void create(Display display)`
/// (`bs.a:(Ljavax/microedition/lcdui/Display;)V`) — the static factory. Being the
/// first active use of `GameLoop`, it triggers `bs.<clinit>`, then
/// `instance = new GameLoop(display)`.
pub fn create(g: &mut Game) {
    // First active use of GameLoop -> bs.<clinit>.
    class_init(g);
    // instance = new GameLoop(display);
    construct(g);
    g.game_loop.instance = true;
}

/// `private GameLoop(Display display)`
/// (`bs.<init>:(Ljavax/microedition/lcdui/Display;)V => [ior, i2b]`) — populates
/// the singleton's instance fields. `volume = AudioManager.maxVolume` is a field
/// initializer (bytecode-first); the rest is the constructor body, in bytecode
/// order.
pub fn construct(g: &mut Game) {
    // volume = AudioManager.maxVolume;   (field initializer)
    g.game_loop.volume = AUDIO_MANAGER_MAX_VOLUME;
    // this.soundEnabled = !Debug.fullVersion;
    g.game_loop.sound_enabled = !DEBUG_FULL_VERSION;
    // this.hasCreatedCharacter = false;
    g.game_loop.has_created_character = false;
    // this.autoTextAdvance = false;
    g.game_loop.auto_text_advance = false;
    // this.cameraFollow = true;
    g.game_loop.camera_follow = true;
    // this.difficulty = (byte) 2;
    g.game_loop.difficulty = 2;
    // this.progressFlags = (byte) 0;
    g.game_loop.progress_flags = 0;
    // this.progressData = 0;
    g.game_loop.progress_data = 0;
    // this.bootPending = true;
    g.game_loop.boot_pending = true;
    // this.display = display;
    g.game_loop.display = true;
    // this.frameDelay = frameDelayTable[this.difficulty];   (unguarded iaload)
    let difficulty = g.game_loop.difficulty;
    g.game_loop.frame_delay = g.game_loop.frame_delay_table[difficulty as usize];
    // applyDifficultyFps();
    apply_difficulty_fps(g);
    // this.progressFlags = (byte) (this.progressFlags | 8);   (ior, i2b)
    g.game_loop.progress_flags = ((g.game_loop.progress_flags as i32) | 8) as i8;
}

/// `public final void setFps(int fps)` (`bs.a:(I)V => [idiv]`). The `1000 / fps`
/// divide is unguarded in the original; a zero `fps` throws an uncaught
/// `ArithmeticException` (fatal), so a panic here is faithful.
pub fn set_fps(g: &mut Game, fps: i32) {
    // this.frameTargetMs = 1000 / fps;
    g.game_loop.frame_target_ms = java_div(1000, fps).expect("ArithmeticException: 1000 / fps");
}

/// `public final void applyDifficultyFps()` (`bs.f:()V => []`).
pub fn apply_difficulty_fps(g: &mut Game) {
    // setFps(this.frameDelay);
    let frame_delay = g.game_loop.frame_delay;
    set_fps(g, frame_delay);
}

/// `public final void start()` (`bs.c:()V => []`) — marks a boot and launches the
/// loop thread. The spawned thread runs `GameLoop.run()`, which constructs the
/// `TitleScreen` and drives the first frame; that run-loop + TitleScreen render is
/// DEFERRED to the next increment. The boot-entry stops here, before the first
/// real paint.
pub fn start(g: &mut Game) {
    // this.bootPending = true;
    g.game_loop.boot_pending = true;
    // new Thread(this).start();   — deferred (see above).
}

/// One iteration of `public final void run()` (`bs.run:()V => []`) — the frame
/// loop's body, as the single-frame drive the host calls (the loop-wrapper +
/// `callSerially` re-arm are the host's concern).
///
/// The `bootPending` branch of `run()` (construct `TitleScreen`,
/// `display.setCurrent`, `boot()`, `AudioManager.readySound`, `setLoadingFps`) is
/// driven by the caller in this increment; `boot()`'s font/config/language setup
/// and the async asset-loader thread that reaches state 10 are DEFERRED (see
/// [`title_screen`]). Here we model the `synchronized (lock)` critical section:
/// `markFrameStart` → `current.flushKey` → `current.requestRepaint`, then MIDP's
/// serialized dispatch of the owed repaint into one `paint`.
pub fn run_one_frame(g: &mut Game) {
    // synchronized (lock) {
    //   if (this.stopped) return;
    if g.game_loop.stopped {
        return;
    }
    //   markFrameStart();
    mark_frame_start(g);
    //   this.current.flushKey();
    base_canvas::flush_key(g);
    //   this.current.requestRepaint();
    base_canvas::request_repaint(g);
    //   this.display.callSerially(this);   — re-arm for the next frame (host concern).
    // }
    // MIDP serializes the owed repaint into a paint callback; dispatch it so one
    // run_one_frame yields one rendered frame.
    let owed = g
        .canvas
        .as_mut()
        .map(|c| c.service_repaints())
        .unwrap_or(false);
    if owed {
        title_screen::paint(g);
    }
}

/// `public final void markFrameStart()` (`bs.a:()V => []`).
pub fn mark_frame_start(g: &mut Game) {
    // this.frameStartMs = System.currentTimeMillis();
    g.game_loop.frame_start_ms = g.clock.current_time_millis();
}

/// `public final void throttle()` (`bs.b:()V`). `EventScript.skip` (snapshot,
/// false on the title path) fast-forwards; otherwise sleep to the frame's target
/// duration.
pub fn throttle(g: &mut Game) {
    // if (EventScript.skip) return;
    if EVENT_SCRIPT_SKIP {
        return;
    }
    // sleepFor(this.frameStartMs, this.frameTargetMs);
    let frame_start_ms = g.game_loop.frame_start_ms;
    let frame_target_ms = g.game_loop.frame_target_ms as i64; // int -> long (i2l)
    sleep_for(g, frame_start_ms, frame_target_ms);
}

/// `public final void sleepFor(long startMs, long targetMs)`
/// (`bs.a:(JJ)V => [lsub, lsub]`). Sleeps so `targetMs` passes since `startMs`;
/// yields if already over. `Thread.yield()` / `Thread.sleep(...)` are no-ops here
/// (no observable state); the two `long` subtractions are preserved.
pub fn sleep_for(g: &mut Game, start_ms: i64, target_ms: i64) {
    // long elapsedMs = System.currentTimeMillis() - startMs;
    let elapsed_ms = g.clock.current_time_millis().wrapping_sub(start_ms);
    // if (elapsedMs >= targetMs) { Thread.yield(); } else { Thread.sleep(targetMs - elapsedMs); }
    if elapsed_ms >= target_ms {
        // Thread.yield() — no-op.
    } else {
        let _sleep_ms = target_ms.wrapping_sub(elapsed_ms);
        // Thread.sleep(targetMs - elapsedMs) — no-op.
    }
}

/// `public final void setLoadingFps()` (`bs.g:()V => []`) — 5 FPS for loading.
pub fn set_loading_fps(g: &mut Game) {
    // setFps(5);
    set_fps(g, 5);
}
