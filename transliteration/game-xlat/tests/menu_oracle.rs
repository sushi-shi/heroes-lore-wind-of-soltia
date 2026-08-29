//! EXACT-PIXEL differential oracle for the MAIN MENU (the boot → title → any-key →
//! parchment main menu sequence, and its DOWN-navigation carousel).
//!
//! This is the milestone follow-up to `first_frame_oracle.rs`: that oracle proved
//! the publisher splash (0 px) and the state-1 title (136 px). THIS oracle drives
//! the port one key past the title — `TitleScreen.keyPressed` (state 1) →
//! `enterStoryMode` → the loadPhase-2 loader → `GameLoop.showGameScreen` →
//! `GameState.buildLoadMenu` → `GameScreen.paint` (screen 9) → `MainMenu.draw` —
//! and compares each rendered menu frame against the FreeJ2ME captures the
//! `00-boot` / `01-menu` routes recorded.
//!
//! Route correspondence (`tools/oracle/routes/{00-boot,01-menu}.txt`):
//!   * `main-menu-new-game` / `menu-new-game` — title, then one SOFT1 (any key),
//!     settle → the six-item menu with NEW GAME selected;
//!   * `menu-options` / `menu-help` / `menu-about` / `menu-exit` — each a DOWN from
//!     the previous (the disabled LOAD row is skipped on the fresh install, so the
//!     selectable order is NEW GAME → OPTIONS → HELP → ABOUT → EXIT).
//!
//! Comparator: RGB only, alpha masked (`& 0x00FF_FFFF`) — matches
//! `tools/oracle/compare_frames.py::compare_exact` and `first_frame_oracle.rs`.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, font_manager, game_loop, game_midlet, title_screen, Game,
};
use j2me_me::Image;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The route's game-RNG seed (`seed 305419896`). The menu itself is RNG-free, but
/// the title frames before the key consume `ByteUtil.rng` (birds); seeding keeps
/// the pre-menu drive deterministic (as in `first_frame_oracle`).
const GAME_RNG_SEED: i64 = 305419896;

/// RGB-only mask: the FreeJ2ME reference PNGs are 24-bit, the framebuffer is ARGB.
const RGB_MASK: u32 = 0x00FF_FFFF;

/// A captured reference frame must carry at least this many distinct colours or it
/// is treated as blank/frozen and cannot be trusted (compare_frames.py blind-spot #3).
const MINIMUM_COLOURS: usize = 16;

/// State-1 title frames to paint before the any-key press (any settled title frame
/// works — the menu that follows is RNG-free).
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;

/// Frames to run after each key so the menu's `logoFrame` intro animation settles
/// (0 → 1 → 2, then it holds). The reference is captured after a 60-frame settle;
/// the settled menu is idempotent, so any count past ~4 yields the same frame.
const MENU_SETTLE: u32 = 12;

/// SOFT1 (left soft key) key code — the route's `tap SOFT1`. At the state-1 title
/// ANY key leaves to the menu, so the exact code is immaterial to the transition.
const KEY_SOFT1: i32 = -6;

/// DOWN key code (`KEY_NUM8`) — matches `moveCursorVertical`'s `case 56` and maps to
/// the DOWN game action.
const KEY_DOWN: i32 = 56;

// --------------------------------------------------------------------------
// Paths / reference decode (fail LOUD, never skip)
// --------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root is two levels above the crate manifest")
        .to_path_buf()
}

/// Reference PNG under `_reference/oracle/reference/pass-1/<rel>` (git-ignored
/// FreeJ2ME captures).
fn reference_path(rel: &str) -> PathBuf {
    repo_root().join(format!("_reference/oracle/reference/pass-1/{rel}.png"))
}

fn diag_dir() -> PathBuf {
    let dir = repo_root().join("_temp/oracle/menu");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn load_reference(rel: &str) -> Image {
    let path = reference_path(rel);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "reference frame `{rel}` not found at {} ({e}); it is the git-ignored \
             FreeJ2ME capture. The oracle FAILS (never skips) when ground truth is \
             absent (GATES.md R4).",
            path.display()
        )
    });
    let decoder = png::Decoder::new(&bytes[..]);
    let mut reader = decoder
        .read_info()
        .unwrap_or_else(|e| panic!("reference `{rel}` failed to read_info: {e}"));
    let (w, h, color, depth) = {
        let info = reader.info();
        (info.width, info.height, info.color_type, info.bit_depth)
    };
    assert_eq!((w, h), (240, 320), "reference `{rel}` is the 240x320 frame");
    assert_eq!(depth, png::BitDepth::Eight, "reference `{rel}` is 8-bit");
    let channels = match color {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => panic!("reference `{rel}` has unexpected colour type {other:?}"),
    };
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut buf)
        .unwrap_or_else(|e| panic!("reference `{rel}` failed to decode IDAT: {e}"));
    let data = &buf[..frame.buffer_size()];
    let mut pixels = Vec::with_capacity((w * h) as usize);
    for chunk in data.chunks_exact(channels) {
        let (r, g, b) = (chunk[0] as u32, chunk[1] as u32, chunk[2] as u32);
        pixels.push(0xff00_0000 | (r << 16) | (g << 8) | b);
    }
    Image::from_argb(w as i32, h as i32, pixels).expect("reference ARGB buffer")
}

// --------------------------------------------------------------------------
// Port drive: boot → title → any-key → main menu → N DOWN presses
// --------------------------------------------------------------------------

fn key_press(g: &mut Game, code: i32) {
    g.canvas.as_mut().expect("canvas").key_pressed(code);
}

/// Drives the port to the settled main menu after `down_presses` DOWN keys from
/// NEW GAME. `down_presses = 0` is the `menu-new-game` / `main-menu-new-game` frame.
fn drive_menu(down_presses: u32) -> Game {
    let mut g = Game::new();
    // seed 305419896 — the route's game-RNG seed (pre-menu title birds).
    g.byte_util = byte_util::ByteUtilState::seeded(GAME_RNG_SEED);
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    game_midlet::construct(&mut g);
    game_midlet::start_app(&mut g);
    title_screen::construct(&mut g);
    // The boot's title prerequisites (anti-bog): logo + title atlases, the six
    // fonts, and the labels (title footer/version + the main-menu item + soft-key
    // labels the loadLabels/apply boot subset fills). The main-menu asset atlas
    // (`menuFrames`) is loaded later, by the keyPressed→loadPhase-2 transition.
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
    // Run the state-10 logo animation until startTitle flips to the state-1 title.
    let mut guard = 0u32;
    loop {
        game_loop::run_one_frame(&mut g);
        guard += 1;
        if g.title_screen.state == 1 {
            break;
        }
        assert!(guard < 10_000, "state-10 never transitioned to the title");
    }
    // A few settled title frames, then the any-key press → the main menu.
    for _ in 0..TITLE_FRAMES_BEFORE_KEY {
        game_loop::run_one_frame(&mut g);
    }
    key_press(&mut g, KEY_SOFT1);
    for _ in 0..MENU_SETTLE {
        game_loop::run_one_frame(&mut g);
    }
    // DOWN navigation (each resets logoFrame; settle again between shots).
    for _ in 0..down_presses {
        key_press(&mut g, KEY_DOWN);
        for _ in 0..MENU_SETTLE {
            game_loop::run_one_frame(&mut g);
        }
    }
    g
}

fn render_menu(down_presses: u32) -> Image {
    drive_menu(down_presses)
        .screen
        .as_ref()
        .expect("framebuffer")
        .clone()
}

// --------------------------------------------------------------------------
// Comparator + diagnostics
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
        "{what} looks blank/frozen: only {n} distinct colours (< {MINIMUM_COLOURS})"
    );
}

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

/// The six menu frames: (down-presses, reference relative path, ratchet key).
const MENU_FRAMES: &[(u32, &str, &str)] = &[
    (0, "00-boot/main-menu-new-game", "main-menu-new-game"),
    (0, "01-menu/menu-new-game", "menu-new-game"),
    (1, "01-menu/menu-options", "menu-options"),
    (2, "01-menu/menu-help", "menu-help"),
    (3, "01-menu/menu-about", "menu-about"),
    (4, "01-menu/menu-exit", "menu-exit"),
];

// ==========================================================================
// Oracles
// ==========================================================================

/// Ground truth is decodable, correctly sized, and non-blank.
#[test]
fn menu_reference_frames_decode_and_are_non_blank() {
    for (_, rel, _) in MENU_FRAMES {
        let img = load_reference(rel);
        assert_eq!((img.width(), img.height()), (240, 320));
        assert_non_blank(&img, &format!("reference `{rel}`"));
    }
}

/// The two "new game" references (`00-boot/main-menu-new-game` and
/// `01-menu/menu-new-game`) are the SAME captured frame — the port renders one
/// framebuffer for both, so they must diff identically against it.
#[test]
fn the_two_new_game_references_are_the_same_frame() {
    let a = load_reference("00-boot/main-menu-new-game");
    let b = load_reference("01-menu/menu-new-game");
    assert_eq!(
        differing_pixels(&a, &b),
        0,
        "the 00-boot and 01-menu new-game captures differ — the route assumption is wrong"
    );
}

/// DIAGNOSTIC (non-asserting): prints the honest differing-pixel count + region
/// breakdown for every menu frame, and dumps the port/diff PNGs. Used to set the
/// ratchet; always passes so one run reports all six.
#[test]
fn menu_frames_diagnostic() {
    for (down, rel, key) in MENU_FRAMES {
        let g = drive_menu(*down);
        eprintln!(
            "STATE `{key}`: logo_frame={} cursor_index={} item_count={} screen={} current={:?}",
            g.main_menu.logo_frame,
            g.main_menu.base.cursor_index,
            g.main_menu.base.item_count,
            g.game_state.screen,
            g.current_screen
        );
        let port = g.screen.as_ref().expect("framebuffer").clone();
        let reference = load_reference(rel);
        let differing = differing_pixels(&port, &reference);
        eprintln!(
            "DIAG `{key}` (down={down}): differing_pixels = {differing}  (port colours: {})",
            distinct_colours(&port)
        );
        for ((cx, cy), c) in region_breakdown(&port, &reference).iter().take(4) {
            eprintln!("      region col{cx} row{cy}: {c} differing");
        }
        write_rgb_png(&diag_dir().join(format!("port_{key}.png")), &port);
        write_diff_png(
            &diag_dir().join(format!("diff_{key}.png")),
            &port,
            &reference,
        );
        // Histogram of (port_rgb -> ref_rgb) over the differing pixels — reveals a
        // uniform colour swap vs a positional mismatch.
        if *down == 0 && *key == "menu-new-game" {
            let mut hist: std::collections::HashMap<(u32, u32), usize> =
                std::collections::HashMap::new();
            for (i, (&p, &q)) in port.pixels().iter().zip(reference.pixels()).enumerate() {
                if (p & RGB_MASK) != (q & RGB_MASK) {
                    *hist.entry((p & RGB_MASK, q & RGB_MASK)).or_default() += 1;
                    let _ = i;
                }
            }
            // The residual is 100% (port_rgb -> ref_rgb) colour pairs, no positional
            // mismatch: black<->white on the highlighted big-font label and
            // white->#ff8800 (smallOrange) on the soft keys — the FreeJ2ME
            // getColor/setColor colour-latch on WHITE-fill text (the port renders the
            // MIDP-correct colours; the reference does not). Same emulator artifact as
            // the title-footer 136px residual.
            let mut pairs: Vec<_> = hist.into_iter().collect();
            pairs.sort_by_key(|e| std::cmp::Reverse(e.1));
            for ((p, q), c) in pairs.iter().take(12) {
                eprintln!("      port #{p:06x} -> ref #{q:06x} : {c}");
            }
        }
    }
}

/// THE MILESTONE. Each menu frame (NEW GAME + the four DOWN steps) is driven
/// through the real keyPressed→transition→GameScreen→MainMenu path and diffed RGB
/// against ground truth; the honest residual is pinned by the ratchet. Writes the
/// port frame + a diff visualization + region breakdown to the git-ignored sink.
#[test]
fn menu_frames_agree_to_the_ratchet() {
    for (down, rel, key) in MENU_FRAMES {
        let port = render_menu(*down);
        let reference = load_reference(rel);
        assert_non_blank(&port, &format!("port `{key}`"));
        assert_non_blank(&reference, &format!("reference `{rel}`"));

        let differing = differing_pixels(&port, &reference);
        eprintln!("port `{key}` (down={down}) vs {rel}: differing_pixels = {differing}");
        for ((cx, cy), c) in region_breakdown(&port, &reference).iter().take(3) {
            eprintln!("    region col{cx} row{cy}: {c} differing");
        }
        write_rgb_png(&diag_dir().join(format!("port_{key}.png")), &port);
        write_diff_png(
            &diag_dir().join(format!("diff_{key}.png")),
            &port,
            &reference,
        );

        let recorded = recorded_agreement(key);
        assert_eq!(
            differing, recorded,
            "`{key}` agreement moved to {differing} (ratchet has {recorded}). If the \
             render improved, update tests/first_frame_oracle_agreement.txt deliberately \
             after viewing _temp/oracle/menu/diff_{key}.png"
        );
    }
}
