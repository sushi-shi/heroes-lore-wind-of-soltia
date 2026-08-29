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
