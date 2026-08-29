//! Transliterated from `java/src/main/java/defpackage/TitleScreen.java`
//! (original `bg.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The title/intro `BaseCanvas`. This increment ports the title render path: the
//! constructor (materialising the `Canvas` + framebuffer + geometry), `startLogo`
//! (which arms the state-10 logo animation), `paint`'s **state-10 (logo)** branch
//! (the publisher splash), the `startTitle` state-10 → state-1 transition, and
//! `paint`'s **state-1 (title)** branch — the HEROES LORE title art
//! (`titleBgFrames`/`titleMenuFrames`), the version ("2.0.7") + "PRESS ANY KEY"
//! footer text, and the RNG-driven fluttering-bird animation (captured as the
//! reference `title-logo.png`, 240×320). `AudioManager.playBgm(22)` in
//! `startTitle` is deferred.
//!
//! DEFERRED (anti-bog): `boot()` and the async `run()` loader (AppConfig / TextTable
//! / the sprite + string banks; the font/label/title-asset prerequisites the
//! state-1 paint needs are driven directly by the caller — see the oracle),
//! `keyPressed`, and `enterStoryMode`.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bg.<init>:()V => []`,
//! `bg.b:()V => []` (startLogo). `bg.paint:(…Graphics;)V` covers BOTH switch
//! branches; both are transliterated below, preserving the source arithmetic
//! (`imul`/`i2s` for the glyph position updates, `idiv` for `width / 2`, `irem`
//! for `animTick % 4`, `iadd`/`i2b` for the frame counters).

use crate::asset_cache;
use crate::base_canvas;
use crate::byte_util;
use crate::font_manager;
use crate::game::Game;
use crate::game_loop;
use j2me_jvm::{java_div, java_rem};

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
    // switch (this.state) { case 1: <title>; case 10: <logo>; }
    match g.title_screen.state {
        1 => paint_title(g),
        10 => paint_logo(g),
        _ => {}
    }
    // GameLoop.instance.throttle();
    game_loop::throttle(g);
}

/// `paint`'s `case 1:` — the HEROES LORE title screen: white clear, the assembled
/// title art (`titleBgFrames[2..4]`), the orange version text ("2.0.7"), the two
/// fluttering birds (`titleMenuFrames`, ping-pong sprite + drifting position), the
/// blinking "PRESS ANY KEY" footer, and the per-frame glyph state machine (which
/// consumes `ByteUtil.randRange`). The two `drawImage` anchors are `20` (TOP|LEFT)
/// for the art and `33` (HCENTER|BOTTOM) for the birds.
fn paint_title(g: &mut Game) {
    // Disjoint field borrows: the framebuffer (mut, via Graphics), the title banks
    // (read), the geometry (read), the fonts/labels (read), the anim fields (mut),
    // and the RNG (mut).
    let Game {
        screen,
        asset_cache,
        base_canvas,
        title_screen,
        font_manager,
        byte_util,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // graphics.setColor(16777215);
    graphics.set_color(16777215);
    // graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.fill_rect(0, 0, base_canvas.width, base_canvas.height);
    // int logoTopY = BaseCanvas.halfH - 68;
    let logo_top_y: i32 = base_canvas.half_h.wrapping_sub(68);
    // int logoLeftX = BaseCanvas.halfW - 60;
    let logo_left_x: i32 = base_canvas.half_w.wrapping_sub(60);
    {
        let title_bg = asset_cache
            .title_bg_frames
            .as_ref()
            .expect("AssetCache.titleBgFrames null");
        // graphics.drawImage(titleBgFrames[2], logoLeftX + 0, logoTopY + 25, 20);
        graphics
            .draw_image(
                &title_bg[2],
                logo_left_x.wrapping_add(0),
                logo_top_y.wrapping_add(25),
                20,
            )
            .expect("drawImage(titleBgFrames[2])");
        // graphics.drawImage(titleBgFrames[3], logoLeftX + 52, logoTopY + 25, 20);
        graphics
            .draw_image(
                &title_bg[3],
                logo_left_x.wrapping_add(52),
                logo_top_y.wrapping_add(25),
                20,
            )
            .expect("drawImage(titleBgFrames[3])");
        // graphics.drawImage(titleBgFrames[4], logoLeftX + 93, logoTopY + 2, 20);
        graphics
            .draw_image(
                &title_bg[4],
                logo_left_x.wrapping_add(93),
                logo_top_y.wrapping_add(2),
                20,
            )
            .expect("drawImage(titleBgFrames[4])");
    }
    // graphics.setColor(4136767);
    graphics.set_color(4136767);
    // if (FontManager.versionText != null) FontManager.drawChars(graphics,
    //     (BaseCanvas.width - 2) - FontManager.stringWidth(versionText), BaseCanvas.height - 31, versionText, 0);
    if let Some(version_text) = font_manager.version_text.as_ref() {
        let width_px = font_manager::string_width(font_manager, version_text);
        let x = base_canvas.width.wrapping_sub(2).wrapping_sub(width_px);
        let y = base_canvas.height.wrapping_sub(31);
        font_manager::draw_chars(font_manager, &mut graphics, x, y, version_text, 0);
    }
    {
        let title_menu = asset_cache
            .title_menu_frames
            .as_ref()
            .expect("AssetCache.titleMenuFrames null");
        // graphics.drawImage(titleMenuFrames[glyph1Frame < 4 ? glyph1Frame : 8 - glyph1Frame], glyph1X, glyph1Y, 33);
        let idx1: i32 = if (title_screen.glyph1_frame as i32) < 4 {
            title_screen.glyph1_frame as i32
        } else {
            8i32.wrapping_sub(title_screen.glyph1_frame as i32)
        };
        graphics
            .draw_image(
                &title_menu[idx1 as usize],
                title_screen.glyph1_x as i32,
                title_screen.glyph1_y as i32,
                33,
            )
            .expect("drawImage(bird1)");
        // graphics.drawImage(titleMenuFrames[(glyph2Frame < 4 ? glyph2Frame : 8 - glyph2Frame) + 5], glyph2X, glyph2Y, 33);
        let idx2: i32 = (if (title_screen.glyph2_frame as i32) < 4 {
            title_screen.glyph2_frame as i32
        } else {
            8i32.wrapping_sub(title_screen.glyph2_frame as i32)
        })
        .wrapping_add(5);
        graphics
            .draw_image(
                &title_menu[idx2 as usize],
                title_screen.glyph2_x as i32,
                title_screen.glyph2_y as i32,
                33,
            )
            .expect("drawImage(bird2)");
    }
    // glyph1X = (short) (glyph1X + (10 * (glyph1Frame < 4 ? 1 : -1)));
    title_screen.glyph1_x = (title_screen.glyph1_x as i32).wrapping_add(10i32.wrapping_mul(
        if (title_screen.glyph1_frame as i32) < 4 {
            1
        } else {
            -1
        },
    )) as i16;
    // glyph1Y = (short) (glyph1Y + ByteUtil.randRange(-1, 4));
    title_screen.glyph1_y =
        (title_screen.glyph1_y as i32).wrapping_add(byte_util::rand_range(byte_util, -1, 4)) as i16;
    // glyph2X = (short) (glyph2X + (10 * (glyph2Frame < 4 ? -1 : 1)));
    title_screen.glyph2_x = (title_screen.glyph2_x as i32).wrapping_add(10i32.wrapping_mul(
        if (title_screen.glyph2_frame as i32) < 4 {
            -1
        } else {
            1
        },
    )) as i16;
    // glyph2Y = (short) (glyph2Y + ByteUtil.randRange(-1, 4));
    title_screen.glyph2_y =
        (title_screen.glyph2_y as i32).wrapping_add(byte_util::rand_range(byte_util, -1, 4)) as i16;
    // glyph1Frame = (byte) (glyph1Frame + 1);
    title_screen.glyph1_frame = (title_screen.glyph1_frame as i32).wrapping_add(1) as i8;
    // glyph2Frame = (byte) (glyph2Frame + 1);
    title_screen.glyph2_frame = (title_screen.glyph2_frame as i32).wrapping_add(1) as i8;
    // if (glyph1Frame > 7) glyph1Frame = (byte) 0;
    if (title_screen.glyph1_frame as i32) > 7 {
        title_screen.glyph1_frame = 0;
    }
    // if (glyph2Frame > 7) glyph2Frame = (byte) 0;
    if (title_screen.glyph2_frame as i32) > 7 {
        title_screen.glyph2_frame = 0;
    }
    // if (glyph1Y > BaseCanvas.height + 10) { respawn glyph1 }
    if (title_screen.glyph1_y as i32) > base_canvas.height.wrapping_add(10) {
        // glyph1X = (short) ByteUtil.randRange(10, (BaseCanvas.width / 2) - 10);
        let hi = java_div(base_canvas.width, 2)
            .expect("width / 2")
            .wrapping_sub(10);
        title_screen.glyph1_x = byte_util::rand_range(byte_util, 10, hi) as i16;
        // glyph1Y = (short) ((-10) * ByteUtil.randRange(0, 4));
        title_screen.glyph1_y =
            (-10i32).wrapping_mul(byte_util::rand_range(byte_util, 0, 4)) as i16;
        // glyph1Frame = (byte) ByteUtil.randRange(0, 7);
        title_screen.glyph1_frame = byte_util::rand_range(byte_util, 0, 7) as i8;
    }
    // if (glyph2Y > BaseCanvas.height + 10) { respawn glyph2 }
    if (title_screen.glyph2_y as i32) > base_canvas.height.wrapping_add(10) {
        // glyph2X = (short) ByteUtil.randRange((BaseCanvas.width / 2) + 10, BaseCanvas.width - 10);
        let lo = java_div(base_canvas.width, 2)
            .expect("width / 2")
            .wrapping_add(10);
        let hi = base_canvas.width.wrapping_sub(10);
        title_screen.glyph2_x = byte_util::rand_range(byte_util, lo, hi) as i16;
        // glyph2Y = (short) ((-10) * ByteUtil.randRange(3, 7));
        title_screen.glyph2_y =
            (-10i32).wrapping_mul(byte_util::rand_range(byte_util, 3, 7)) as i16;
        // glyph2Frame = (byte) ByteUtil.randRange(0, 7);
        title_screen.glyph2_frame = byte_util::rand_range(byte_util, 0, 7) as i8;
    }
    // if (this.animTick % 4 < 2) { graphics.setColor(0);
    //   FontManager.drawCharsCentered(graphics, BaseCanvas.halfW, BaseCanvas.height - 45, titleFooter, 1); }
    if java_rem(title_screen.anim_tick, 4).expect("animTick % 4") < 2 {
        graphics.set_color(0);
        let footer = font_manager
            .title_footer
            .as_ref()
            .expect("FontManager.titleFooter null");
        font_manager::draw_chars_centered(
            font_manager,
            &mut graphics,
            base_canvas.half_w,
            base_canvas.height.wrapping_sub(45),
            footer,
            1,
        );
    }
    // this.animTick++;
    title_screen.anim_tick = title_screen.anim_tick.wrapping_add(1);
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

/// `public final void keyPressed(int keyCode)` (`bg.keyPressed:(I)V => []`): the
/// title's key handler. On the state-1 title, ANY key leaves the title into
/// `enterStoryMode` (→ the main menu on a fresh install). `AudioManager.stopBgm()`
/// is DEFERRED (audio not ported).
pub fn key_pressed(g: &mut Game, key_code: i32) {
    // getGameAction(keyCode);   — the return value is discarded (no side effect).
    let _ = j2me_me::Canvas::common_game_action(key_code);
    // if (GameLoop.instance == null || GameLoop.instance.stopped) return;
    if !g.game_loop.instance || g.game_loop.stopped {
        return;
    }
    // switch (this.state)
    match g.title_screen.state {
        1 => {
            // AudioManager.stopBgm();   — DEFERRED (audio not ported).
            // AssetCache.unloadLogo();
            asset_cache::unload_logo(g);
            // AssetCache.unloadTitleScreen();
            asset_cache::unload_title_screen(g);
            // enterStoryMode(false, (byte) 1);
            enter_story_mode(g, false, 1);
        }
        2 => {
            // this.state = (byte) 3; this.animTick = 0; setLoadingFps(); instance = this; new Thread(instance).start();
            //   NOT reached on the fresh boot→menu route (enterStoryMode goes 1 → 0/loadPhase-2,
            //   never 2, because progressFlags bit 8 makes `resume` true). The async loader
            //   thread is DEFERRED.
            g.title_screen.state = 3;
            g.title_screen.anim_tick = 0;
            game_loop::set_loading_fps(g);
            g.title_screen.instance = true;
            // new Thread(instance).start();   — DEFERRED (async run() loader).
        }
        6 => {
            // GameMIDlet.instance.destroyApp(true);   — DEFERRED (lifecycle terminator).
        }
        10 => {
            // startTitle();
            start_title(g);
        }
        _ => {}
    }
}

/// `public final void enterStoryMode(boolean resume, byte mode)`: on a fresh
/// install `progressFlags` carries bit 8 (set by the `GameLoop` constructor), so
/// `resume` becomes true and control takes the loader path (state 0, loadPhase 2),
/// which loads the main-menu assets and shows the `GameScreen`. `AudioManager.stopSfx()`
/// and `BaseCanvas.beginLoading(...)` are DEFERRED.
pub fn enter_story_mode(g: &mut Game, resume: bool, mode: i8) {
    // AudioManager.stopSfx();   — DEFERRED (audio not ported).
    let mut resume = resume;
    // if (!resume) { if ((progressFlags & (mode==1 ? 8 : 2)) != 0) resume = true; }
    if !resume {
        let mask: i8 = if mode == 1 { 8 } else { 2 };
        // byte & byte promotes to int in Java; bit 8 is positive so no sign issue.
        if ((g.game_loop.progress_flags as i32) & (mask as i32)) != 0 {
            resume = true;
        }
    }
    // if (!resume || this.skipStoryIntro) { this.state = (byte) 2; return; }
    if !resume || g.title_screen.skip_story_intro {
        g.title_screen.state = 2;
        return;
    }
    // this.state = (byte) 0; this.loadPhase = (byte) 2;
    g.title_screen.state = 0;
    g.title_screen.load_phase = 2;
    // BaseCanvas.beginLoading("- STORY MODE", 52);   — DEFERRED (loading-overlay
    //   counters; the state-0 loading screen is not captured — the route settles
    //   straight to the main menu).
    // GameLoop.instance.setLoadingFps();
    game_loop::set_loading_fps(g);
    // new Thread(this).start();   — the async loader thread. Executed synchronously
    //   here (single-threaded transliteration convention; the intermediate loading
    //   frames are not captured, and this reaches the same settled main-menu state).
    run(g);
}

/// One activation of `public final void run()` (`bg.run:()V => []`) — the ported
/// state-0 / loadPhase-2 branch: load the main-menu assets, show the `GameScreen`,
/// then mark the loader done (`state = -1`, `loadPhase = 0`). `AssetLoader.loadStringTables()`
/// is DEFERRED (the menu labels are already loaded by `loadLabels` at boot;
/// `commonText` is not read by the main-menu render). The loadPhase-1 first-boot
/// loader is DEFERRED (driven by the caller — see the module header).
pub fn run(g: &mut Game) {
    // switch (this.state) { case 0: switch (this.loadPhase) { ... } }
    if g.title_screen.state == 0 {
        // switch (loadPhase)  — only `case 2` (the main-menu loader) is ported;
        // `case 1` is DEFERRED to the default arm (hence single_match).
        #[allow(clippy::single_match)]
        match g.title_screen.load_phase {
            2 => {
                // AssetLoader.loadStringTables();   — DEFERRED (see above).
                // AssetCache.loadMainMenuAssets();
                asset_cache::load_main_menu_assets(g);
                // GameLoop.instance.showGameScreen();
                game_loop::show_game_screen(g);
                // this.state = (byte) -1;
                g.title_screen.state = -1;
                // this.loadPhase = (byte) 0;
                g.title_screen.load_phase = 0;
            }
            // (DEFERRED: case 1 — the first-boot global-UI / options loader.)
            _ => {}
        }
    }
}
