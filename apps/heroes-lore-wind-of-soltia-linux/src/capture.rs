//! `--script` / `--out` — drive one SHARED route script into the transliteration
//! ([`GameHost`]) and write one PNG per `shot`, in the layout the comparison tool
//! (`tools/oracle/compare_frames.py`) pairs by label.
//!
//! The routes in `tools/oracle/routes/` are the *same files* that drive
//! FreeJ2ME-Plus through `tools/oracle/HeadlessCapture.java` /
//! `capture_reference.sh`. Feeding the same keystrokes into both runtimes is what
//! makes the comparison a comparison of behaviour, and it exercises the real input
//! path — the R9 serial queue. `capture_port_frames.sh` loops the routes over this
//! binary and writes the provenance manifest, mirroring the reference-side script.
//!
//! ## The route format (a strict superset-tolerant subset of the emulator driver's)
//!
//! One command per line; `#` starts a comment. Every command may carry trailing
//! `key=value` tokens, and any this consumer does not know is ignored. The port's
//! frame budget is read from `frames=` if present, else the emulator's `java_frames=`
//! (these WoS routes carry only the `java_*` tokens), so the SAME route drives both
//! runtimes to the same frame counts with no port-specific edits:
//!
//!   fps <n>                              emulator gated-stepping; no-op here
//!   seed <n>                             reseed the shared RNG in place (ByteUtil.rng)
//!   wait <ms> [java_]frames=<F> [java_ms=<MS>]        advance F frames
//!   tap <KEY> <holdms> <settlems> [java_]frames=<F> [java_]settle_frames=<S>
//!                                        press KEY, hold F frames, release, settle S
//!   hold/down <KEY> ... frames=<F>       press and hold across F frames
//!   release/up <KEY> ... frames=<F>      release and advance F frames
//!   shot <label> [layer=..] [same-as=..] write <route>/<label>.png
//!   echo ... / expect ...                emulator-only; no-ops here
//!
//! ## Boot alignment with the emulator
//!
//! [`GameHost`] boots straight to the state-10 publisher splash; the real MIDlet
//! shows an async loader and (with sound) a prompt before it. The reference route
//! marks that emulator-only preamble with `port=skip`, which this consumer skips so
//! both runtimes arrive at the same splash by the first shared `shot`. This is the
//! one declared boot-path deviation.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::host::{GameHost, InputEvent, CLOCK_START_MS, H, RNG_SEED, W};

/// One parsed route command.
struct Command {
    verb: String,
    positional: Vec<String>,
    named: HashMap<String, String>,
}

impl Command {
    fn parse(line: &str) -> Command {
        let mut positional = Vec::new();
        let mut named = HashMap::new();
        let mut tokens = line.split_whitespace();
        let verb = tokens.next().unwrap_or("").to_string();
        for token in tokens {
            if let Some((k, v)) = token.split_once('=') {
                named.insert(k.to_string(), v.to_string());
            } else {
                positional.push(token.to_string());
            }
        }
        Command {
            verb,
            positional,
            named,
        }
    }

    /// The port's frame budget for this command: the port token (`frames=` /
    /// `settle_frames=`) if present, else the emulator token (`java_frames=` /
    /// `java_settle_frames=`) these routes actually carry, else `default`.
    fn frames(&self, port_key: &str, java_key: &str, default: u64) -> u64 {
        self.named
            .get(port_key)
            .and_then(|v| v.parse().ok())
            .or_else(|| self.named.get(java_key).and_then(|v| v.parse().ok()))
            .unwrap_or(default)
    }

    /// The game-time budget (ms) this command advances: `java_ms=` wins, else the
    /// named `millis_key`, else the positional at `index`, else `fallback`. Advancing
    /// the injected clock by this keeps the port's game-time tracking the reference's
    /// deterministic clock (which advances by the same budget under frame stepping).
    fn millis(&self, millis_key: &str, index: usize, fallback: i64) -> i64 {
        if let Some(v) = self.named.get("java_ms").and_then(|v| v.parse().ok()) {
            return v;
        }
        if let Some(v) = self.named.get(millis_key).and_then(|v| v.parse().ok()) {
            return v;
        }
        self.positional
            .get(index)
            .and_then(|v| v.parse().ok())
            .unwrap_or(fallback)
    }
}

/// Advance `frames` host frames, spreading `budget_ms` of game-time across them
/// (every frame gets `budget/frames`, the last gets the remainder), delivering
/// `event` on the FIRST frame.
fn step(host: &mut GameHost, frames: u64, budget_ms: i64, event: Option<InputEvent>) {
    let frames = frames.max(1);
    let share = budget_ms / frames as i64;
    for i in 0..frames {
        let this = if i + 1 == frames {
            budget_ms - share * (frames as i64 - 1)
        } else {
            share
        };
        host.advance_clock(this);
        match (i == 0).then_some(()).and(event) {
            Some(ev) => host.tick(&[ev]),
            None => host.tick(&[]),
        }
    }
}

/// Map a route key name to the raw Nokia code the game's `keyPressed(int)` sees —
/// the exact codes `HeadlessCapture.java` and the shared routes use.
fn key_code(name: &str) -> Result<i32, String> {
    Ok(match name.to_ascii_uppercase().as_str() {
        "UP" => -1,
        "DOWN" => -2,
        "LEFT" => -3,
        "RIGHT" => -4,
        "FIRE" => -5,
        "SOFT1" => -6,
        "SOFT2" => -7,
        "SEND" => -10,
        "END" => -11,
        "STAR" => 42,
        "POUND" => 35,
        other if other.starts_with("NUM") => {
            let d: i32 = other[3..]
                .parse()
                .map_err(|_| format!("bad NUM key: {name}"))?;
            if !(0..=9).contains(&d) {
                return Err(format!("NUM key out of range: {name}"));
            }
            48 + d
        }
        _ => return Err(format!("unknown key {name}")),
    })
}

/// Run one route file against a fresh [`GameHost`], writing a PNG per `shot` into
/// `out_dir/<route_stem>/`. Returns the number of shots written.
pub fn run_route(jar: &Path, route_file: &Path, out_dir: &Path) -> Result<usize, String> {
    let stem = route_file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("route file has no stem")?
        .to_string();
    let route_out = out_dir.join(&stem);
    fs::create_dir_all(&route_out).map_err(|e| format!("mkdir {}: {e}", route_out.display()))?;

    let text = fs::read_to_string(route_file)
        .map_err(|e| format!("read {}: {e}", route_file.display()))?;

    let mut host = GameHost::with_clock_start(jar, CLOCK_START_MS, RNG_SEED)
        .map_err(|e| format!("construct game: {e}"))?;
    let mut shots = 0usize;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cmd = Command::parse(line);
        // `port=skip` marks the emulator-only boot preamble the port never shows.
        if cmd.named.get("port").map(String::as_str) == Some("skip") {
            continue;
        }
        match cmd.verb.as_str() {
            // Emulator-only directives: accepted, no-op on the port.
            "echo" | "fps" | "expect" => {}

            // Reseed the shared RNG in place — the reference driver reseeds every
            // game Random to pin RNG-driven animation just before a shot.
            "seed" => {
                let n: i64 = cmd
                    .positional
                    .first()
                    .ok_or("seed needs a value")?
                    .parse()
                    .map_err(|_| "bad seed value".to_string())?;
                host.reseed_rng(n);
            }

            "wait" => {
                step(
                    &mut host,
                    cmd.frames("frames", "java_frames", 1),
                    cmd.millis("ms", 0, 0),
                    None,
                );
            }

            "tap" => {
                let code = key_code(cmd.positional.first().ok_or("tap needs a key")?)?;
                let held = cmd.frames("frames", "java_frames", 1).max(1);
                let settle = cmd.frames("settle_frames", "java_settle_frames", 1);
                step(
                    &mut host,
                    held,
                    cmd.millis("ms", 1, 60),
                    Some(InputEvent::Press(code)),
                );
                step(
                    &mut host,
                    settle,
                    cmd.millis("settle", 2, 200),
                    Some(InputEvent::Release(code)),
                );
            }

            "hold" | "down" => {
                let code = key_code(cmd.positional.first().ok_or("hold needs a key")?)?;
                step(
                    &mut host,
                    cmd.frames("frames", "java_frames", 1).max(1),
                    cmd.millis("ms", 1, 0),
                    Some(InputEvent::Press(code)),
                );
            }

            "release" | "up" => {
                let code = key_code(cmd.positional.first().ok_or("release needs a key")?)?;
                step(
                    &mut host,
                    cmd.frames("frames", "java_frames", 1).max(1),
                    cmd.millis("ms", 1, 0),
                    Some(InputEvent::Release(code)),
                );
            }

            "shot" => {
                let label = cmd.positional.first().ok_or("shot needs a label")?;
                let path = route_out.join(format!("{label}.png"));
                write_frame_png(host.frame().pixels(), W, H, &path)?;
                shots += 1;
            }

            other => return Err(format!("bad script cmd: {other} (line: {line})")),
        }
    }

    Ok(shots)
}

/// Write an ARGB framebuffer as an 8-bit RGB PNG (alpha dropped — both runtimes
/// composite into one opaque LCDUI surface, and the comparator reads RGB). The
/// channel order matches the emulator's `TYPE_INT_RGB` output.
fn write_frame_png(pixels: &[u32], w: i32, h: i32, path: &Path) -> Result<(), String> {
    let (w, h) = (w as usize, h as usize);
    if pixels.len() != w * h {
        return Err(format!(
            "framebuffer is {} px, expected {w}x{h}={}",
            pixels.len(),
            w * h
        ));
    }
    let mut rgb = Vec::with_capacity(w * h * 3);
    for &p in pixels {
        rgb.push(((p >> 16) & 0xFF) as u8);
        rgb.push(((p >> 8) & 0xFF) as u8);
        rgb.push((p & 0xFF) as u8);
    }
    let file = fs::File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("png header {}: {e}", path.display()))?;
    writer
        .write_image_data(&rgb)
        .map_err(|e| format!("png data {}: {e}", path.display()))?;
    Ok(())
}
