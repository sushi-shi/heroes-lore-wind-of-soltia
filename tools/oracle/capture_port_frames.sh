#!/usr/bin/env bash
#
# Capture the Rust PORT's frames over the shared routes, for the live frame oracle.
#
# This is the port half of the two-sided oracle. The reference half
# (tools/oracle/capture_reference.sh) runs the original bytecode on FreeJ2ME-Plus;
# this half runs the strict Rust transliteration through the native host
# (apps/heroes-lore-wind-of-soltia-linux) in its headless `--script` mode. BOTH are
# driven by the SAME route files in tools/oracle/routes, and both write one PNG per
# `shot` label, so tools/oracle/compare_frames.py pairs frames by label and diffs
# them EXACTLY (`differing_pixels == 0` is the one clean state).
#
# Nothing third-party is built here: the port is a workspace crate. This script only
# builds that crate, runs it over each route into git-ignored
# _reference/oracle/port/pass-N/, and writes a provenance manifest.tsv that
# compare_frames.py fails closed against (jar sha256, per-route sha256, pass count).
#
# GAME-AGNOSTIC. Every per-game knob it needs -- the JAR path and the canvas
# geometry -- is read from game.toml's [oracle] section, so this script carries no
# game-specific literal and stamps into any 2D J2ME port unchanged. The crate name
# is derived from game.toml's slug (`<slug>-linux`), mirroring the workspace layout.
#
# Usage:
#   tools/oracle/capture_port_frames.sh [options]
#
#     --out DIR        capture root (default _reference/oracle/port)
#     --routes A,B     only these routes, by file stem
#     --route-dir DIR  read routes from here instead of tools/oracle/routes
#     --passes N       run the whole matrix N times (default 2, to catch flakiness)
#     --release        build/run the release profile (default: dev, overflow-checked)
#     --build-only     build the port and stop
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

case "${1:-}" in -h|--help) sed -n '2,27p' "$0"; exit 0 ;; esac

log() { printf '[port-capture] %s\n' "$*" >&2; }

# ------------------------------------------------------------ per-game config
# Read the [oracle] knobs from game.toml with a tiny tomllib reader (preferring
# python3 on PATH, falling back to `nix shell nixpkgs#python3`), exactly like the
# reference-side script -- so this carries no game-specific literal.
read_oracle_config() {
  local reader='
import sys, shlex
try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib
with open(sys.argv[1], "rb") as fh:
    cfg = tomllib.load(fh)
o = cfg.get("oracle") or sys.exit("game.toml has no [oracle] section")
def need(k):
    if k not in o: sys.exit("game.toml [oracle] is missing key: " + k)
    return o[k]
def emit(n, v): print(n + "=" + shlex.quote(str(v)))
emit("CFG_JAR", need("jar"))
emit("CFG_CANVAS_W", need("canvas_w"))
emit("CFG_CANVAS_H", need("canvas_h"))
emit("CFG_SLUG", cfg.get("slug", "j2me"))
'
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "$reader" "$REPO_ROOT/game.toml"
  elif command -v nix >/dev/null 2>&1; then
    nix shell nixpkgs#python3 --command python3 -c "$reader" "$REPO_ROOT/game.toml"
  else
    echo "need python3 (or nix) to read game.toml [oracle]" >&2; return 1
  fi
}
config="$(read_oracle_config)" || { log "FATAL: could not read [oracle] knobs from game.toml"; exit 1; }
eval "$config"

# ------------------------------------------------------------------- defaults
OUT_DIR="$REPO_ROOT/_reference/oracle/port"
ROUTE_DIR="$REPO_ROOT/tools/oracle/routes"
JAR="$REPO_ROOT/$CFG_JAR"
CRATE="${CFG_SLUG}-linux"
ROUTE_FILTER=""
PASSES=2
BUILD_ONLY=0
PROFILE="dev"
PROFILE_DIR="debug"

while [ $# -gt 0 ]; do
  case "$1" in
    --out) OUT_DIR="$2"; shift 2 ;;
    --routes) ROUTE_FILTER="$2"; shift 2 ;;
    --route-dir) ROUTE_DIR="$2"; shift 2 ;;
    --passes) PASSES="$2"; shift 2 ;;
    --release) PROFILE="release"; PROFILE_DIR="release"; shift ;;
    --build-only) BUILD_ONLY=1; shift ;;
    -h|--help) sed -n '2,27p' "$0"; exit 0 ;;
    *) echo "unknown option $1" >&2; exit 2 ;;
  esac
done

[ -f "$JAR" ] || { log "FATAL: missing $JAR -- materialize _originals first (just bootstrap <resources>)"; exit 1; }

# The cargo runner: prefer cargo on PATH, else the nix devshell's cargo, so this
# works both inside `nix develop` and from a bare shell.
cargo_run() {
  if command -v cargo >/dev/null 2>&1; then
    ( cd "$REPO_ROOT" && cargo "$@" )
  elif command -v nix >/dev/null 2>&1; then
    ( cd "$REPO_ROOT" && nix develop --command cargo "$@" )
  else
    log "FATAL: no cargo (and no nix) to build the port"; return 1
  fi
}

# ---------------------------------------------------------------- build the port
log "building $CRATE ($PROFILE profile)"
if [ "$PROFILE" = "release" ]; then
  cargo_run build --release -p "$CRATE" >&2
else
  cargo_run build -p "$CRATE" >&2
fi
BIN="$REPO_ROOT/target/$PROFILE_DIR/$CRATE"
[ -x "$BIN" ] || { log "FATAL: built binary not found at $BIN"; exit 1; }

[ "$BUILD_ONLY" = "1" ] && { log "build only; stopping"; exit 0; }

# ---------------------------------------------------------------- routes
routes=()
for file in "$ROUTE_DIR"/*.txt; do
  stem="$(basename "$file" .txt)"
  if [ -n "$ROUTE_FILTER" ]; then
    case ",$ROUTE_FILTER," in *",$stem,"*) ;; *) continue ;; esac
  fi
  routes+=("$stem")
done
[ "${#routes[@]}" -gt 0 ] || { log "FATAL: no routes selected"; exit 1; }
log "routes: ${routes[*]}"

# A scoped or lower-pass rerun must not inherit an older selected route from a pass
# it will not rewrite: remove this run's route directories from every existing pass
# before recording anything, and drop a pass dir that becomes empty.
for pass_dir in "$OUT_DIR"/pass-*; do
  [ -d "$pass_dir" ] || continue
  for stem in "${routes[@]}"; do rm -rf "$pass_dir/$stem"; done
  rmdir "$pass_dir" 2>/dev/null || true
done

run_route() { # run_route <pass-dir> <route-stem>
  local pass_dir="$1" stem="$2"
  local route_dir="$pass_dir/$stem"
  # The binary writes <out>/<stem>/<label>.png, so hand it the PASS dir as --out and
  # let it create the per-route subdir (compare_frames.py reads pass-N/<stem>/*.png).
  rm -rf "$route_dir"; mkdir -p "$pass_dir"
  local logf; logf="$(mktemp)"
  if ! "$BIN" --jar "$JAR" --script "$ROUTE_DIR/$stem.txt" --out "$pass_dir" > "$logf" 2>&1; then
    log "FATAL: $stem failed:"; cat "$logf" >&2; rm -f "$logf"; return 1
  fi
  mkdir -p "$route_dir"          # the binary created it; be defensive
  mv "$logf" "$route_dir/run.log"
  local shots
  shots="$(find "$route_dir" -name '*.png' | wc -l)"
  if [ "$shots" -eq 0 ]; then
    log "FATAL: $stem exited cleanly but wrote no frames; see $route_dir/run.log"
    return 1
  fi
  # Stamp the route beside the frames it produced, so a partial re-run cannot vouch
  # for a route it did not run.
  sha256sum "$ROUTE_DIR/$stem.txt" | cut -d' ' -f1 > "$route_dir/route.sha256"
  log "  $stem: $shots frames"
}

mkdir -p "$OUT_DIR"
for pass in $(seq 1 "$PASSES"); do
  log "pass $pass of $PASSES"
  for stem in "${routes[@]}"; do
    run_route "$OUT_DIR/pass-$pass" "$stem"
  done
done

# ---------------------------------------------------------------- manifest
row() { printf '%s\t%s\n' "$1" "$2"; }
{
  echo "# Port frame captures -- machine-readable provenance."
  echo "# Regenerate with tools/oracle/capture_port_frames.sh."
  row captured_utc "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  row repo_head "$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
  row port_crate "$CRATE"
  row port_profile "$PROFILE"
  row jar "$(basename "$JAR")"
  row jar_sha256 "$(sha256sum "$JAR" | cut -d' ' -f1)"
  row canvas "${CFG_CANVAS_W}x${CFG_CANVAS_H}"
  row passes "$PASSES"
  row routes_captured "${routes[*]}"
  for stamp in "$OUT_DIR"/pass-1/*/route.sha256; do
    [ -f "$stamp" ] || continue
    printf 'route_sha256\t%s\t%s\n' "$(basename "$(dirname "$stamp")")" "$(cat "$stamp")"
  done
} > "$OUT_DIR/manifest.tsv"

log "port captures written to $OUT_DIR"
