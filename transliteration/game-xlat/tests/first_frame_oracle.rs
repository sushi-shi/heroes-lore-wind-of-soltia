//! EXACT-PIXEL differential oracle for the port's first rendered frame.
//!
//! This is the follow-up the `first_frame.rs` header promised: the pixel-richness
//! gate proved a REAL frame was rendered; THIS oracle proves *which* frame — by
//! counting the pixels that differ from ground truth (a FreeJ2ME-Plus capture of
//! the real v2.0.7 JAR). `differing_pixels == 0` proves the transliteration
//! renders identically to the real game.
//!
//! Comparator semantics (match `tools/oracle/compare_frames.py::compare_exact`):
//! compare RGB only — mask the alpha byte (`& 0x00FF_FFFF`), since the FreeJ2ME
//! PNGs are 24-bit and the port framebuffer is ARGB. Exact-0 is the clean state.
//!
//! FRAME ALIGNMENT (the crux — see the module tests for the measured evidence).
//! The port's state-10 logo animation (`startLogo` → `TitleScreen.paint` case 10)
//! draws `AssetCache.logoFrames[4]` sliding down until `animTick` converges to
//! `halfH`, then holds. Empirically the frame is IDENTICAL across the whole
//! settled window (driven frames 12..=49): `animTick == 160`, `glyph1X == 1`, so
//! the image sits centred at y = 159 every frame. It is only after `glyph1Frame`
//! passes 40 (frame ~50) that `glyph1X` starts doubling, slides the logo off, and
//! `startTitle` flips to the deferred state-1 screen (blank in the port). So any
//! frame in 12..=49 is the aligned "seated logo" frame; this oracle drives to
//! [`ALIGNED_FRAME`].
//!
//! THE FRAME THE PORT ACTUALLY RENDERS. `logoFrames[4]` is the **HANDS-ON MOBILE
//! publisher splash** (the green-disc logo), NOT the title art. So the port's
//! first rendered frame corresponds to the reference **`publisher-splash.png`**,
//! against which it is PIXEL-EXACT (0 differing pixels). The reference
//! **`title-logo.png`** is a later, distinct screen — the state-1 HEROES LORE
//! title (title art + fluttering birds + "PRESS ANY KEY" + "2.0.7") — whose paint
//! branch this increment DEFERS. Both comparisons are wired here honestly: the
//! publisher-splash match is asserted exact (the proof); the title-logo gap is
//! recorded in a ratchet and diagnosed as screen-misalignment (see the report).

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, font_manager, game_loop, game_midlet, title_screen, Game,
};
use j2me_me::Image;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The RNG seed the FreeJ2ME capture route (`tools/oracle/routes/00-boot.txt`)
/// applies with `seed 305419896` before the title animation. See
/// [`drive_title_frame`] for why seeding once at the start reproduces the captured
/// `title-logo` frame.
const GAME_RNG_SEED: i64 = 305419896;

/// Post-transition state-1 paint count at which the port's fluttering birds match
/// the `title-logo` capture. Derived from the RNG simulation (the reference is the
/// 42nd state-1 paint, sim index k=41; see the sweep diagnostic below) and
/// confirmed by [`title_sweep_locates_the_matching_state1_frame`].
const TITLE_STATE1_FRAMES: u32 = 42;

/// A driven frame inside the settled window (12..=49) — the seated logo. Any
/// value in that window yields the identical framebuffer (see [`alignment`]).
const ALIGNED_FRAME: u32 = 20;

/// RGB-only mask: the FreeJ2ME reference PNGs are 24-bit, the framebuffer is ARGB.
const RGB_MASK: u32 = 0x00FF_FFFF;

/// A captured reference frame must carry at least this many distinct colours or
/// it is treated as blank/frozen and cannot be trusted as ground truth (the
/// compare_frames.py blind-spot #3 countermeasure, `MINIMUM_COLOURS`).
const MINIMUM_COLOURS: usize = 16;

// --------------------------------------------------------------------------
// Paths
// --------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root is two levels above the crate manifest")
        .to_path_buf()
}

/// Reference label PNG. The two capture passes are byte-identical (deterministic
/// capture, verified by sha256); pass-1 is canonical.
fn reference_path(label: &str) -> PathBuf {
    repo_root().join(format!(
        "_reference/oracle/reference/pass-1/00-boot/{label}.png"
    ))
}

/// Git-ignored diagnostics sink (`_temp` is in `.gitignore`).
fn diag_dir() -> PathBuf {
    let dir = repo_root().join("_temp/oracle/first_frame");
    std::fs::create_dir_all(&dir).ok();
    dir
}

// --------------------------------------------------------------------------
// Reference decode — fail LOUD (never skip) if ground truth is absent
// --------------------------------------------------------------------------

fn load_reference(label: &str) -> Image {
    let path = reference_path(label);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "reference frame `{label}` not found at {} ({e}); it is the git-ignored \
             FreeJ2ME capture — regenerate with tools/oracle/capture_reference.sh. \
             The oracle FAILS (never skips) when ground truth is absent (GATES.md R4).",
            path.display()
        )
    });
    let decoder = png::Decoder::new(&bytes[..]);
    let mut reader = decoder
        .read_info()
        .unwrap_or_else(|e| panic!("reference `{label}` failed to read_info: {e}"));
    let (w, h, color, depth) = {
        let info = reader.info();
        (info.width, info.height, info.color_type, info.bit_depth)
    };
    assert_eq!(
        (w, h),
        (240, 320),
        "reference `{label}` is the 240×320 frame"
    );
    assert_eq!(depth, png::BitDepth::Eight, "reference `{label}` is 8-bit");
    let channels = match color {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => panic!("reference `{label}` has unexpected colour type {other:?}"),
    };
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut buf)
        .unwrap_or_else(|e| panic!("reference `{label}` failed to decode IDAT: {e}"));
    let data = &buf[..frame.buffer_size()];
    let mut pixels = Vec::with_capacity((w * h) as usize);
    for chunk in data.chunks_exact(channels) {
        let (r, g, b) = (chunk[0] as u32, chunk[1] as u32, chunk[2] as u32);
        pixels.push(0xff00_0000 | (r << 16) | (g << 8) | b);
    }
    Image::from_argb(w as i32, h as i32, pixels).expect("reference ARGB buffer")
}

// --------------------------------------------------------------------------
// Port render path (mirrors first_frame.rs::drive_first_frame)
// --------------------------------------------------------------------------

fn drive_first_frame(frames: u32) -> Game {
    let mut g = Game::new();
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    game_midlet::construct(&mut g);
    game_midlet::start_app(&mut g);
    title_screen::construct(&mut g);
    // On the real device both loadLogo and loadTitleScreen (plus the fonts/labels)
    // run before any frame; load them all so a drive that slides past state-10 into
    // state-1 can render. This leaves the state-10 publisher frame unchanged (its
    // paint reads only logoFrames), and does not touch the game RNG.
    asset_cache::load_logo(&mut g);
    asset_cache::load_title_screen(&mut g);
    font_manager::init_fonts(&mut g);
    font_manager::load_title_labels(&mut g);
    title_screen::start_logo(&mut g);
    {
        let Game {
            display, canvas, ..
        } = &mut g;
        display.set_current(None, canvas.as_mut().expect("TitleScreen canvas"));
    }
    for _ in 0..frames {
        game_loop::run_one_frame(&mut g);
    }
    g
}

fn render_port_frame(frames: u32) -> Image {
    drive_first_frame(frames)
        .screen
        .as_ref()
        .expect("framebuffer")
        .clone()
}

/// Drive the port to a **state-1 title** frame, mirroring the FreeJ2ME capture
/// route (`tools/oracle/routes/00-boot.txt`).
///
/// RNG DETERMINISM (task requirement #4). The route does `seed 305419896` before
/// the title shot. Mechanically that reseed lands *after* the last paint of the
/// preceding `wait` and the `shot` does not repaint, so the captured `title-logo`
/// frame is fixed by the route's FIRST `seed 305419896` (at route start) run
/// through the whole boot→logo→title sequence — nothing consumes the game RNG
/// (`ByteUtil.rng` / `h.a`) until `TitleScreen.startTitle`, so seeding once here,
/// before any frame, reproduces it. We seed `Game::byte_util` to the same value.
///
/// The drive then loads the title assets + fonts + labels (the deferred boot's
/// prerequisites, at the anti-bog boundary), arms the state-10 logo, runs frames
/// until the state-10 → state-1 transition (`startTitle`), then runs
/// `post_transition` more state-1 paints. State-1 paint index k (sim) is drawn by
/// the (k+1)-th post-transition frame.
fn drive_title_frame(post_transition: u32) -> Game {
    let mut g = Game::new();
    // seed 305419896  — the route's game-RNG seed (see the doc comment).
    g.byte_util = byte_util::ByteUtilState::seeded(GAME_RNG_SEED);
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    game_midlet::construct(&mut g);
    game_midlet::start_app(&mut g);
    title_screen::construct(&mut g);
    // The boot/run loader's title prerequisites (anti-bog): logo + title atlases,
    // the six fonts, and the two labels the state-1 paint reads.
    asset_cache::load_logo(&mut g);
    asset_cache::load_title_screen(&mut g);
    font_manager::init_fonts(&mut g);
    font_manager::load_title_labels(&mut g);
    title_screen::start_logo(&mut g);
    {
        let Game {
            display, canvas, ..
        } = &mut g;
        display.set_current(None, canvas.as_mut().expect("TitleScreen canvas"));
    }
    // Run the state-10 logo animation until startTitle flips state to 1.
    let mut guard = 0u32;
    loop {
        game_loop::run_one_frame(&mut g);
        guard += 1;
        if g.title_screen.state == 1 {
            break;
        }
        assert!(
            guard < 10_000,
            "state-10 never transitioned to state-1 (startTitle not reached)"
        );
    }
    // Then `post_transition` state-1 paints (k = 0 .. post_transition-1).
    for _ in 0..post_transition {
        game_loop::run_one_frame(&mut g);
    }
    g
}

fn render_title_frame(post_transition: u32) -> Image {
    drive_title_frame(post_transition)
        .screen
        .as_ref()
        .expect("framebuffer")
        .clone()
}

// --------------------------------------------------------------------------
// Comparator (RGB only, alpha masked) — matches compare_frames.py
// --------------------------------------------------------------------------

fn differing_pixels(a: &Image, b: &Image) -> usize {
    assert_eq!(
        (a.width(), a.height()),
        (b.width(), b.height()),
        "frames must share the device geometry"
    );
    a.pixels()
        .iter()
        .zip(b.pixels())
        .filter(|(&p, &q)| (p & RGB_MASK) != (q & RGB_MASK))
        .count()
}

fn distinct_colours(img: &Image) -> usize {
    img.pixels()
        .iter()
        .map(|&p| p & RGB_MASK)
        .collect::<HashSet<_>>()
        .len()
}

fn assert_non_blank(img: &Image, what: &str) {
    let n = distinct_colours(img);
    assert!(
        n >= MINIMUM_COLOURS,
        "{what} looks blank/frozen: only {n} distinct colours (< {MINIMUM_COLOURS}) — \
         a vacuous oracle would pass on this; refusing (non-vacuity)"
    );
}

// --------------------------------------------------------------------------
// Local ratchet (`tests/first_frame_oracle_agreement.txt`, read at runtime)
// --------------------------------------------------------------------------

fn recorded_agreement(label: &str) -> usize {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/first_frame_oracle_agreement.txt");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("ratchet not found at {} ({e})", path.display()));
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == label {
                return v
                    .trim()
                    .parse()
                    .unwrap_or_else(|e| panic!("ratchet `{label}` not an integer: {e}"));
            }
        }
    }
    panic!("ratchet has no entry for `{label}`");
}

// --------------------------------------------------------------------------
// Diagnostics — dump port / reference / diff PNGs to the git-ignored sink
// --------------------------------------------------------------------------

fn write_rgb_png(path: &Path, img: &Image) {
    let mut rgb = Vec::with_capacity((img.width() * img.height()) as usize * 3);
    for &p in img.pixels() {
        rgb.push(((p >> 16) & 0xff) as u8);
        rgb.push(((p >> 8) & 0xff) as u8);
        rgb.push((p & 0xff) as u8);
    }
    let file = std::fs::File::create(path).expect("create diag png");
    let mut enc = png::Encoder::new(
        std::io::BufWriter::new(file),
        img.width() as u32,
        img.height() as u32,
    );
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png header")
        .write_image_data(&rgb)
        .expect("png data");
}

/// Magenta where RGB differs; dimmed-grey reference where they agree — so the eye
/// lands on the disagreeing regions.
fn write_diff_png(path: &Path, port: &Image, reference: &Image) {
    let (w, h) = (port.width(), port.height());
    let mut rgb = Vec::with_capacity((w * h) as usize * 3);
    for (&p, &q) in port.pixels().iter().zip(reference.pixels()) {
        if (p & RGB_MASK) != (q & RGB_MASK) {
            rgb.extend_from_slice(&[0xff, 0x00, 0xff]);
        } else {
            let (r, g, b) = ((q >> 16) & 0xff, (q >> 8) & 0xff, q & 0xff);
            let dim = (((r + g + b) / 3) / 2 + 96) as u8;
            rgb.extend_from_slice(&[dim, dim, dim]);
        }
    }
    let file = std::fs::File::create(path).expect("create diff png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png header")
        .write_image_data(&rgb)
        .expect("png data");
}

/// Differing-pixel counts over a 4×5 grid (60×64 cells), sorted most-different
/// first — locates the region a diagnosis should name.
fn region_breakdown(port: &Image, reference: &Image) -> Vec<((usize, usize), usize)> {
    let (w, h) = (port.width() as usize, port.height() as usize);
    let (cols, rows) = (4usize, 5usize);
    let (cw, ch) = (w / cols, h / rows);
    let mut counts = vec![0usize; cols * rows];
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            if (port.pixels()[idx] & RGB_MASK) != (reference.pixels()[idx] & RGB_MASK) {
                let cx = (x / cw).min(cols - 1);
                let cy = (y / ch).min(rows - 1);
                counts[cy * cols + cx] += 1;
            }
        }
    }
    let mut out: Vec<((usize, usize), usize)> = (0..rows)
        .flat_map(|cy| (0..cols).map(move |cx| (cx, cy)))
        .map(|(cx, cy)| ((cx, cy), counts[cy * cols + cx]))
        .collect();
    out.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    out
}

// ==========================================================================
// Oracles
// ==========================================================================

/// Ground truth is decodable, correctly-sized, and NON-BLANK — the port never
/// compares against a frozen/vacuous reference (compare_frames.py blind-spot #3).
/// Fails LOUD (never skips) if a reference PNG is absent.
#[test]
fn reference_frames_decode_and_are_non_blank() {
    for label in ["publisher-splash", "title-logo"] {
        let img = load_reference(label);
        assert_eq!((img.width(), img.height()), (240, 320));
        assert_non_blank(&img, &format!("reference `{label}`"));
        eprintln!(
            "reference `{label}`: {} distinct colours",
            distinct_colours(&img)
        );
    }
}

/// THE PROOF. The port's first rendered frame (the seated state-10 logo) is
/// PIXEL-EXACT to the real v2.0.7 game's captured publisher splash: RGB
/// `differing_pixels == 0`. Both frames are asserted non-blank (non-vacuity), and
/// the exact-0 is cross-checked against the recorded ratchet.
#[test]
fn first_rendered_frame_is_pixel_exact_to_the_publisher_splash() {
    let port = render_port_frame(ALIGNED_FRAME);
    let reference = load_reference("publisher-splash");
    assert_non_blank(&port, "port frame");
    assert_non_blank(&reference, "reference `publisher-splash`");

    let differing = differing_pixels(&port, &reference);
    eprintln!("port@{ALIGNED_FRAME} vs publisher-splash: differing_pixels = {differing}");

    // Diagnostics for the record even on the clean path.
    write_rgb_png(&diag_dir().join("port_seated.png"), &port);
    write_rgb_png(&diag_dir().join("publisher_splash.png"), &reference);

    assert_eq!(
        differing, 0,
        "the port's first rendered frame is NOT pixel-exact to the publisher splash \
         ({differing} px differ) — a real rasteriser/asset regression; see \
         _temp/oracle/first_frame/"
    );
    assert_eq!(
        differing,
        recorded_agreement("publisher-splash"),
        "ratchet drift: update tests/first_frame_oracle_agreement.txt deliberately"
    );
}

/// The frame-alignment evidence, asserted (not just narrated): the seated-logo
/// window is stable — every driven frame in 12..=49 produces the IDENTICAL
/// framebuffer — so [`ALIGNED_FRAME`] is well-defined and the exact frame index
/// within the window does not matter.
#[test]
fn alignment_the_seated_logo_window_is_stable() {
    let anchor = render_port_frame(ALIGNED_FRAME);
    for n in [12u32, 14, 30, 41, 49] {
        let f = render_port_frame(n);
        assert_eq!(
            differing_pixels(&f, &anchor),
            0,
            "frame {n} differs from the seated anchor@{ALIGNED_FRAME} — the settled \
             window assumption is wrong; re-derive ALIGNED_FRAME"
        );
    }
    // And the window really does end: frame 60 has slid to the (blank) state-1
    // screen, so it must NOT equal the seated logo.
    let after = render_port_frame(60);
    assert_ne!(
        differing_pixels(&after, &anchor),
        0,
        "frame 60 still equals the seated logo — startTitle/slide-off did not happen"
    );
}

// ==========================================================================
// State-1 TITLE screen oracle (the milestone target)
// ==========================================================================

/// Diagnostic sweep: render the state-1 title across a window of post-transition
/// paint counts and report the differing-pixel count of each against the
/// `title-logo` reference, naming the minimum. The RNG simulation predicts the
/// capture is the 42nd state-1 paint (k=41 ⇒ [`TITLE_STATE1_FRAMES`]); this proves
/// that count is the pixel-diff minimum (the bird animation phase alignment).
#[test]
fn title_sweep_locates_the_matching_state1_frame() {
    let reference = load_reference("title-logo");
    assert_non_blank(&reference, "reference `title-logo`");
    let mut best = (u32::MAX, usize::MAX);
    for k in 36u32..=48 {
        let port = render_title_frame(k);
        let d = differing_pixels(&port, &reference);
        eprintln!("title post-transition={k}: differing_pixels = {d}");
        if d < best.1 {
            best = (k, d);
        }
    }
    eprintln!(
        "MIN title diff at post-transition={} : {} px (TITLE_STATE1_FRAMES = {})",
        best.0, best.1, TITLE_STATE1_FRAMES
    );
    assert_eq!(
        best.0, TITLE_STATE1_FRAMES,
        "the pixel-diff-minimum title frame moved to post-transition={} ({} px); \
         re-derive TITLE_STATE1_FRAMES and re-check the RNG/frame alignment",
        best.0, best.1
    );
}

/// THE MILESTONE. Drive the port to the state-1 HEROES LORE title (the animation
/// moment the `title-logo` reference captured) and diff RGB against ground truth.
/// The title art, version text, birds and footer position all reproduce; the
/// recorded ratchet is the honest residual (see the ratchet file's diagnosis).
/// Writes the port frame + a diff visualization + region breakdown to the
/// git-ignored sink. Non-vacuity: both frames asserted non-blank.
#[test]
fn title_screen_state1_frame_agrees_to_the_ratchet() {
    let port = render_title_frame(TITLE_STATE1_FRAMES);
    let reference = load_reference("title-logo");
    assert_non_blank(&port, "port title frame");
    assert_non_blank(&reference, "reference `title-logo`");

    let differing = differing_pixels(&port, &reference);
    eprintln!("port title@{TITLE_STATE1_FRAMES} vs title-logo: differing_pixels = {differing}");

    write_rgb_png(&diag_dir().join("port_title.png"), &port);
    write_rgb_png(&diag_dir().join("title_logo.png"), &reference);
    write_diff_png(&diag_dir().join("diff_title_logo.png"), &port, &reference);
    let regions = region_breakdown(&port, &reference);
    for ((cx, cy), c) in regions.iter().take(4) {
        eprintln!("  region col{cx} row{cy}: {c} differing");
    }

    let recorded = recorded_agreement("title-logo");
    if recorded == 0 {
        assert_eq!(
            differing, 0,
            "ratchet asserts pixel-exact title, but {differing} px differ — a \
             regression; see _temp/oracle/first_frame/diff_title_logo.png"
        );
    } else {
        assert_eq!(
            differing, recorded,
            "title-logo agreement moved to {differing} (ratchet has {recorded}). If the \
             render improved, update tests/first_frame_oracle_agreement.txt deliberately \
             after viewing _temp/oracle/first_frame/diff_title_logo.png"
        );
    }
}
