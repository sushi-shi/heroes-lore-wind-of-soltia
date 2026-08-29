//! Transliterated from `java/src/main/java/defpackage/TitleScreen.java`
//! (original `bg.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The title/intro `BaseCanvas`. This increment ports the FIRST-FRAME render
//! path: the constructor (materialising the `Canvas` + framebuffer + geometry),
//! `startLogo` (which arms the state-10 logo animation), and `paint`'s **state-10
//! (logo)** branch — the genuine first rendered frame after boot (captured as the
//! reference `title-logo.png`, 240×320). `startTitle` (the state-10 → state-1
//! transition) is ported too; its `AudioManager.playBgm(22)` is deferred.
//!
//! DEFERRED (anti-bog): `boot()` and the async `run()` loader (FontManager /
//! AppConfig / StringTable / TextTable / the sprite + string banks), `keyPressed`,
//! `enterStoryMode`, and `paint`'s **state-1** branch (the falling-glyph title
//! draw over `titleBgFrames`/`titleMenuFrames` + `FontManager` version/footer
//! text). On the first-frame path `state == 10`, so state 1 is not reached.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bg.<init>:()V => []`,
//! `bg.b:()V => []` (startLogo). `bg.paint:(…Graphics;)V` covers BOTH switch
//! branches; the transliterated state-10 subset is `imul,i2s` (glyph1X*2),
//! `isub` (animTick-glyph1X), `isub` (halfH-1), `iadd,i2b` (glyph1Frame+1),
//! `isub,idiv,iadd` (animTick += (halfH-animTick)/2) — a subset of that shape,
//! the state-1 remainder deferred.

use crate::base_canvas;
use crate::byte_util;
use crate::game::Game;
use crate::game_loop;
use j2me_jvm::java_div;

/// Java `bg` / `TitleScreen` state — the screen's instance fields plus its one
/// `static` (`instance`, obf `bg.a:Lbg;`), in reviewed declaration order (see
/// `java/reconstruction/ownership.tsv`). Reference-typed `instance` is a presence
/// flag (`false` while null).
#[derive(Debug, Default)]
pub struct TitleScreenState {
    /// `private byte state;` — screen state machine (0 loading, 1 title, 2 menu,
    /// 10 logo, …).
    pub state: i8,
    /// `private byte loadPhase;` — sub-phase of the state-0 async loader.
    pub load_phase: i8,
    /// `private int animTick;` — animation tick counter.
    pub anim_tick: i32,
    /// `private short glyph1X;`
    pub glyph1_x: i16,
    /// `private short glyph1Y;`
    pub glyph1_y: i16,
    /// `private byte glyph1Frame;`
    pub glyph1_frame: i8,
    /// `private short glyph2X;`
    pub glyph2_x: i16,
    /// `private short glyph2Y;`
    pub glyph2_y: i16,
    /// `private byte glyph2Frame;`
    pub glyph2_frame: i8,
    /// `private boolean skipStoryIntro = false;`
    pub skip_story_intro: bool,
    /// `public static TitleScreen instance;` — latest instance (worker Runnable);
    /// presence flag (null → false). Set in `keyPressed` (deferred), not here.
    pub instance: bool,
}

/// `public TitleScreen()` (`bg.<init>:()V => []`) — after the implicit
/// `BaseCanvas()` super-constructor. Materialises the device surface this canvas
/// paints into (the `Canvas` + its ARGB framebuffer `Image`), runs the
/// `BaseCanvas()` geometry, then the `TitleScreen` body (`new Object();` discarded,
/// `loadPhase = 0`).
pub fn construct(g: &mut Game) {
    // super BaseCanvas() needs a live Canvas to read getWidth()/getHeight() from;
    // materialise it and the framebuffer that IS the rendered frame.
    g.canvas = Some(j2me_me::Canvas::new(
        base_canvas::DEVICE_WIDTH,
        base_canvas::DEVICE_HEIGHT,
    ));
    g.screen = Some(
        j2me_me::Image::create_mutable(base_canvas::DEVICE_WIDTH, base_canvas::DEVICE_HEIGHT)
            .expect("framebuffer allocation"),
    );
    // BaseCanvas(): setFullScreenMode(true); width = getWidth(); halfW = width / 2;
    base_canvas::construct(g);
    // TitleScreen(): new Object();  (discarded)
    //                this.loadPhase = (byte) 0;
    g.title_screen.load_phase = 0;
}

/// `public final void startLogo()` (`bg.b:()V => []`) — arms the logo animation.
pub fn start_logo(g: &mut Game) {
    // GameLoop.instance.setFps(20);
    game_loop::set_fps(g, 20);
    // this.animTick = -20;
    g.title_screen.anim_tick = -20;
    // this.glyph1Frame = (byte) 0;
    g.title_screen.glyph1_frame = 0;
    // this.glyph1X = (short) 1;
    g.title_screen.glyph1_x = 1;
    // this.state = (byte) 10;
    g.title_screen.state = 10;
}

/// `public final void paint(Graphics graphics)` — the title-screen draw. This
/// increment renders the **state-10 (logo)** branch; the state-1 title draw is
/// deferred (see the module header). The `setViewHeight(getClipHeight())` prologue
/// and `GameLoop.instance.throttle()` epilogue are preserved.
pub fn paint(g: &mut Game) {
    // setViewHeight(graphics.getClipHeight());
    //   The repaint covers the whole canvas, so the paint Graphics' clip height is
    //   the full screen height (getClipHeight() == canvas height).
    let clip_height = g
        .screen
        .as_ref()
        .map(|s| s.height())
        .unwrap_or(base_canvas::DEVICE_HEIGHT);
    base_canvas::set_view_height(g, clip_height);
    // switch (this.state) { case 1: <deferred>; case 10: <logo>; }
    match g.title_screen.state {
        1 => {
            // DEFERRED: state-1 title draw (titleBgFrames/titleMenuFrames +
            // FontManager version/footer text). Not reached on the first-frame
            // (state-10) path.
        }
        10 => paint_logo(g),
        _ => {}
    }
    // GameLoop.instance.throttle();
    game_loop::throttle(g);
}

/// `paint`'s `case 10:` — the logo frame. White clear, the logo atlas frame 4
/// drawn centred (sliding down as `animTick` converges to `halfH`), and the
/// tick/frame state machine.
fn paint_logo(g: &mut Game) {
    let should_start_title = {
        // Disjoint field borrows: the framebuffer (mut, via Graphics) alongside
        // the logo bank (read), the geometry (read), and the anim fields (mut).
        let Game {
            screen,
            asset_cache,
            base_canvas,
            title_screen,
            ..
        } = &mut *g;
        let target = screen.as_mut().expect("framebuffer");
        let mut graphics = j2me_me::Graphics::new(target);

        // graphics.setColor(16777215);
        graphics.set_color(16777215);
        // graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
        graphics.fill_rect(0, 0, base_canvas.width, base_canvas.height);
        // if (this.glyph1Frame > 40) this.glyph1X = (short) (this.glyph1X * 2);
        if (title_screen.glyph1_frame as i32) > 40 {
            title_screen.glyph1_x = (title_screen.glyph1_x as i32).wrapping_mul(2) as i16;
        }
        // graphics.drawImage(AssetCache.logoFrames[4], BaseCanvas.halfW, this.animTick - this.glyph1X, 3);
        let logo = &asset_cache
            .logo_frames
            .as_ref()
            .expect("AssetCache.logoFrames null")[4];
        let dy = title_screen
            .anim_tick
            .wrapping_sub(title_screen.glyph1_x as i32);
        graphics
            .draw_image(logo, base_canvas.half_w, dy, 3)
            .expect("drawImage(logo, HCENTER|VCENTER)");
        // if (this.glyph1Frame != 0) { ... } else if (...) { ... } else { ... }
        if title_screen.glyph1_frame != 0 {
            match title_screen.glyph1_frame {
                // case 1: case 3: this.animTick = BaseCanvas.halfH - 1;
                1 | 3 => title_screen.anim_tick = base_canvas.half_h.wrapping_sub(1),
                // case 2: case 4: this.animTick = BaseCanvas.halfH;
                2 | 4 => title_screen.anim_tick = base_canvas.half_h,
                _ => {}
            }
            // this.glyph1Frame = (byte) (this.glyph1Frame + 1);
            title_screen.glyph1_frame = (title_screen.glyph1_frame as i32).wrapping_add(1) as i8;
        } else if title_screen.anim_tick < base_canvas.half_h.wrapping_sub(1) {
            // this.animTick += (BaseCanvas.halfH - this.animTick) / 2;
            let delta = java_div(base_canvas.half_h.wrapping_sub(title_screen.anim_tick), 2)
                .expect("(halfH - animTick) / 2");
            title_screen.anim_tick = title_screen.anim_tick.wrapping_add(delta);
        } else {
            // this.glyph1Frame = (byte) 1;
            title_screen.glyph1_frame = 1;
        }
        // if (this.glyph1X > BaseCanvas.height) startTitle();
        (title_screen.glyph1_x as i32) > base_canvas.height
    };
    if should_start_title {
        start_title(g);
    }
}

/// `private final void startTitle()` — the state-10 → state-1 transition. Not
/// reached on the single first-frame drive (`glyph1X` starts at 1, well under the
/// 320-px screen). `AudioManager.playBgm(22)` is DEFERRED (audio not ported).
fn start_title(g: &mut Game) {
    // GameLoop.instance.setFps(15);
    game_loop::set_fps(g, 15);
    // this.animTick = 0;
    g.title_screen.anim_tick = 0;
    // this.state = (byte) 1;
    g.title_screen.state = 1;
    // this.glyph1X = (short) ByteUtil.randRange(0, (BaseCanvas.width / 2) - 10);
    let hi1 = java_div(g.base_canvas.width, 2)
        .expect("width / 2")
        .wrapping_sub(10);
    g.title_screen.glyph1_x = byte_util::rand_range(&mut g.byte_util, 0, hi1) as i16;
    // this.glyph1Y = (short) (10 * ByteUtil.randRange(0, 4));
    g.title_screen.glyph1_y =
        10i32.wrapping_mul(byte_util::rand_range(&mut g.byte_util, 0, 4)) as i16;
    // this.glyph1Frame = (byte) ByteUtil.randRange(0, 7);
    g.title_screen.glyph1_frame = byte_util::rand_range(&mut g.byte_util, 0, 7) as i8;
    // this.glyph2X = (short) ByteUtil.randRange(BaseCanvas.width / 2, BaseCanvas.width - 10);
    let lo2 = java_div(g.base_canvas.width, 2).expect("width / 2");
    let hi2 = g.base_canvas.width.wrapping_sub(10);
    g.title_screen.glyph2_x = byte_util::rand_range(&mut g.byte_util, lo2, hi2) as i16;
    // this.glyph2Y = (short) (10 * ByteUtil.randRange(3, 7));
    g.title_screen.glyph2_y =
        10i32.wrapping_mul(byte_util::rand_range(&mut g.byte_util, 3, 7)) as i16;
    // this.glyph2Frame = (byte) ByteUtil.randRange(0, 7);
    g.title_screen.glyph2_frame = byte_util::rand_range(&mut g.byte_util, 0, 7) as i8;
    // AudioManager.playBgm(22);   — DEFERRED (audio not ported).
}
