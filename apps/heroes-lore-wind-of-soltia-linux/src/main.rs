//! `heroes-lore-wind-of-soltia-linux` — a native host that plays the strict Heroes
//! Lore: Wind of Soltia transliteration.
//!
//! Three run modes:
//!   * **windowed** (default): open a `winit` window and play — `cargo run -p
//!     heroes-lore-wind-of-soltia-linux`. Needs a display.
//!   * **headless smoke** (`--exit-after-frames N`): construct the game, run N frames
//!     with no window, assert the framebuffer is a real non-blank frame, and exit
//!     0/1 — a usable gate on the command line.
//!   * **frame-oracle capture** (`--script <route.txt> --out <dir>`): drive one
//!     shared route through the host with no window and write one PNG per `shot`
//!     label into `<dir>/<route>/`, so its output pairs with the FreeJ2ME-Plus
//!     reference by label for `tools/oracle/compare_frames.py`.
//!
//! Flags:
//!   --jar <path>              the `_originals` build to load (default: the v207 JAR
//!                             found by walking up to `_originals/`)
//!   --exit-after-frames <N>   headless: run N frames, validate, exit (no window)
//!   --scale <n>               windowed: integer pixel scale (default 3)
//!   --script <route.txt>      frame-oracle: drive one shared route and write one
//!                             PNG per `shot` into --out (no window)
//!   --out <dir>               where `--script` writes frames (default: cwd)

use std::path::PathBuf;
use std::process::ExitCode;

use heroes_lore_wind_of_soltia_linux::capture;
use heroes_lore_wind_of_soltia_linux::frame::analyze;
use heroes_lore_wind_of_soltia_linux::host::GameHost;
use heroes_lore_wind_of_soltia_linux::shell;

fn main() -> ExitCode {
    let args = match Cli::parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("heroes-lore-wind-of-soltia-linux: {msg}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let jar_path = match args.jar.clone().or_else(default_jar) {
        Some(p) => p,
        None => {
            eprintln!(
                "heroes-lore-wind-of-soltia-linux: no JAR given and no baseline found \
                 under `_originals/`.\nPass one, e.g. --jar \
                 _originals/Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar"
            );
            return ExitCode::FAILURE;
        }
    };
    if !jar_path.is_file() {
        eprintln!(
            "heroes-lore-wind-of-soltia-linux: JAR not found: {}",
            jar_path.display()
        );
        return ExitCode::FAILURE;
    }

    if let Some(script) = args.script.clone() {
        return run_capture(&jar_path, &script, args.out.clone());
    }

    match args.exit_after_frames {
        Some(n) => run_headless(&jar_path, n),
        None => run_windowed(&jar_path, args.scale),
    }
}

/// Frame-oracle capture: drive one shared route through [`GameHost`] and write one
/// PNG per `shot` into `out` (default: the current directory). No window. Exits 0 on
/// success, 1 on any route/IO error — a usable gate.
fn run_capture(jar: &std::path::Path, script: &std::path::Path, out: Option<PathBuf>) -> ExitCode {
    let out = out.unwrap_or_else(|| PathBuf::from("."));
    match capture::run_route(jar, script, &out) {
        Ok(shots) => {
            println!(
                "heroes-lore-wind-of-soltia-linux capture: {} -> {shots} frames in {}",
                script.display(),
                out.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("heroes-lore-wind-of-soltia-linux capture: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Headless smoke: build the host, run `frames` frames, and validate the resulting
/// frame. Exits 0 on a real frame, 1 on a blank/degenerate one.
fn run_headless(jar: &std::path::Path, frames: u64) -> ExitCode {
    let mut host = match GameHost::new(jar) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("heroes-lore-wind-of-soltia-linux: failed to construct the game: {e}");
            return ExitCode::FAILURE;
        }
    };

    for _ in 0..frames {
        host.tick(&[]);
    }

    let stats = analyze(host.frame());
    println!(
        "heroes-lore-wind-of-soltia-linux headless: state={}, {frames} frames -> {} distinct \
         colours, {} white, dominant {}/{} px",
        host.title_state(),
        stats.distinct,
        stats.white,
        stats.dominant,
        stats.total
    );
    if stats.is_real_frame() {
        println!("heroes-lore-wind-of-soltia-linux: OK — a real non-blank frame.");
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "heroes-lore-wind-of-soltia-linux: FAIL — the frame is blank or a single flat fill."
        );
        ExitCode::FAILURE
    }
}

/// Windowed play. Needs a display; a missing one surfaces as a loud event-loop error.
fn run_windowed(jar: &std::path::Path, scale: u32) -> ExitCode {
    let host = match GameHost::new(jar) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("heroes-lore-wind-of-soltia-linux: failed to construct the game: {e}");
            return ExitCode::FAILURE;
        }
    };
    let title = "Heroes Lore: Wind of Soltia".to_string();
    match shell::run(host, scale, title) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!(
                "heroes-lore-wind-of-soltia-linux: windowed run failed (a display is required): {e}"
            );
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "\
usage: heroes-lore-wind-of-soltia-linux [--jar <path>]
                    [--exit-after-frames <N>] [--scale <n>]
                    [--script <route.txt> [--out <dir>]]";

struct Cli {
    jar: Option<PathBuf>,
    exit_after_frames: Option<u64>,
    scale: u32,
    script: Option<PathBuf>,
    out: Option<PathBuf>,
}

impl Cli {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Cli, String> {
        let mut cli = Cli {
            jar: None,
            exit_after_frames: None,
            scale: 3,
            script: None,
            out: None,
        };
        while let Some(arg) = args.next() {
            // Accept `--flag=value` as well as `--flag value` (the capture scripts
            // pass `--script=...`/`--out=...`).
            let (arg, inline) = match arg.split_once('=') {
                Some((flag, value)) if flag.starts_with("--") => {
                    (flag.to_string(), Some(value.to_string()))
                }
                _ => (arg.clone(), None),
            };
            let mut take = |label: &str| -> Result<String, String> {
                inline
                    .clone()
                    .or_else(|| args.next())
                    .ok_or_else(|| format!("{label} needs an argument"))
            };
            match arg.as_str() {
                "--jar" => cli.jar = Some(PathBuf::from(take("--jar")?)),
                "--script" => cli.script = Some(PathBuf::from(take("--script")?)),
                "--out" | "--capture-dir" => cli.out = Some(PathBuf::from(take("--out")?)),
                // Accepted for symmetry with the reference capture invocation; this
                // host has one CPU renderer, so the value is not consulted.
                "--renderer" => {
                    let _ = take("--renderer")?;
                }
                "--exit-after-frames" => {
                    let n = take("--exit-after-frames")?;
                    cli.exit_after_frames =
                        Some(n.parse().map_err(|_| format!("bad frame count: {n:?}"))?);
                }
                "--scale" => {
                    let n = take("--scale")?;
                    cli.scale = n.parse().map_err(|_| format!("bad scale: {n:?}"))?;
                }
                "-h" | "--help" => return Err("help".to_string()),
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        Ok(cli)
    }
}

/// The baseline v207 JAR, found by walking up from this crate to `_originals/`.
/// Used only when `--jar` is omitted (a convenience for `cargo run` in the repo).
fn default_jar() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let originals = dir.join("_originals");
        if originals.is_dir() {
            // Prefer the exact baseline name; fall back to a `_v207`-prefixed variant
            // (the materialized name may carry a dedup suffix).
            let mut best: Option<PathBuf> = None;
            if let Ok(entries) = std::fs::read_dir(&originals) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if name == "Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar" {
                        return Some(p);
                    }
                    if name.starts_with("Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207")
                        && name.ends_with(".jar")
                    {
                        best.get_or_insert(p);
                    }
                }
            }
            return best;
        }
        if !dir.pop() {
            return None;
        }
    }
}
