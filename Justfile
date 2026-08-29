set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

# --- Fresh clone -------------------------------------------------------------

# Fresh clone to verified: materialize the corpus, then reconcile it. The
# resource location is passed explicitly; it is never baked into the repo (R1).
# Phase 1 adds `classify` + `catalog` here as they land (R13 clean-slate).
bootstrap resources:
    nix run .#fetch-resources -- {{quote(resources)}}
    just originals-verify

# Verify the materialized _originals against builds.toml's sha256/bytes table.
originals-verify:
    python3 tools/originals/verify.py

# Prove the originals-verify gate can fail (playbook R3). Must exit 0.
originals-verify-canfail:
    python3 tools/originals/verify.py --self-test

# Regenerate builds.toml provenance from a resources dir (mechanical facts only;
# the judgment calls stay flagged for Phase 1 — see the file header).
gen-builds resources match:
    python3 tools/originals/gen_builds.py \
        --resources {{quote(resources)}} --match {{quote(match)}} \
        --slug "$(python3 -c 'import tomllib;print(tomllib.load(open("game.toml","rb"))["slug"])')" \
        --title "$(python3 -c 'import tomllib;print(tomllib.load(open("game.toml","rb"))["title"])')" \
        --out java/reconstruction/builds.toml

# --- Phase 2: numeric-shape opcode authority (playbook R8) -------------------

# Extract each method's ORDERED numeric/conversion opcode sequence from the
# baseline bytecode (the "numeric shape" that catches int/int -> float drift and
# stray float ops during transliteration) and print a non-vacuous risk summary.
# Derived per-method data lands ONLY in git-ignored _reference/ (R1); the tool +
# this recipe are the sole tracked artifacts. Fails loudly if no numeric method
# is found (i.e. the disassembly parse broke).
numeric-shape:
    python3 tools/numeric/extract_shapes.py

# Prove the numeric-shape gate can fail (playbook R3): assert a correct shape,
# then a one-op-corrupted shape for a real method, and confirm the tool detects
# the mismatch. Exits 0 because the failure WAS caught (like the verify canfail).
numeric-shape-canfail:
    python3 tools/numeric/extract_shapes.py --self-test

# --- Java crosswalk oracle (playbook: the "stalker" readable oracle) ---------

# Compile the TRACKED, hand-maintained, de-obfuscated named-Java crosswalk under
# java/src/main/java (the readable oracle the later Rust transliteration is
# validated against) together with the minimal MIDP/CLDC stubs under java/stubs.
# Bootstrapped once from the JADX/CFR decompile of the v2.0.7 baseline; from here
# it is a maintained source tree, NOT regenerated on every build. Class output
# lands in git-ignored target/java-classes. All 90 baseline classes (89 default
# package + rpg.GameMIDlet) plus the stub API surface must compile.
build-java:
    mkdir -p target/java-classes
    find java/stubs java/src/main/java -name '*.java' > target/java-classes/sources.txt
    javac -encoding UTF-8 -d target/java-classes @target/java-classes/sources.txt
    printf 'build-java: %s app classes + %s stub classes compiled\n' \
        "$(find target/java-classes/defpackage target/java-classes/rpg -name '*.class' | wc -l | tr -d ' ')" \
        "$(find target/java-classes/javax -name '*.class' | wc -l | tr -d ' ')"

# --- Java ME frame oracle (the reference side of the pixel-exact port gate) --

# Build FreeJ2ME-Plus from source (pinned commit + the two non-M3G patches) in a
# git-ignored scratch dir, then run the ORIGINAL v2.0.7 JAR headless and capture
# one PNG per labeled `shot` in every route under tools/oracle/routes/. Two
# passes, so a non-reproducible frame is caught. Writes only to git-ignored
# _reference/oracle/reference/ (PNGs + a provenance manifest); never the JAR or
# the built emulator. This is an INDEPENDENT witness for the phone platform, not
# ground truth (see docs/ORACLE.md).
java-me-frames:
    bash tools/oracle/capture_reference.sh

# Check the reference frames. Until the Rust port can be captured, this runs the
# comparator's inline self-test: it verifies capture provenance (jar + routes +
# emulator build) fail-closed, proves the in-process pixel diff actually reacts
# (ref-vs-ref = 0, ref-vs-one-perturbed-pixel > 0, distinct labels differ a lot),
# and enforces the non-vacuity/liveness floors. Once a port capture exists under
# _reference/oracle/port/, the same recipe runs the full EXACT per-label compare
# against tools/oracle/agreement.toml (0 differing pixels is the only clean state).
java-me-frames-check:
    if [ -d _reference/oracle/port/pass-1 ]; then \
      python3 tools/oracle/compare_frames.py; \
    else \
      python3 tools/oracle/compare_frames.py --self-test; \
    fi

# --- Test batteries ----------------------------------------------------------

test:
    if [ -d tools/tests ]; then python3 -m unittest discover -s tools/tests; fi
    if [ -f Cargo.toml ]; then cargo test --workspace; fi

# Every gate the project has today. Grows as phases land; every gate cited here
# must exist and be proven able to fail (playbook R3, R14).
check:
    just originals-verify
    just originals-verify-canfail
    just numeric-shape
    just numeric-shape-canfail
    just build-java
    if [ -d tools/tests ]; then python3 -m unittest discover -s tools/tests; fi
    if [ -f Cargo.toml ]; then cargo fmt --all --check; fi
    if [ -f Cargo.toml ]; then cargo clippy --workspace --all-targets -- -D warnings; fi
    if [ -f Cargo.toml ]; then cargo test --workspace; fi
