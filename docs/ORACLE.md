# The Java ME frame oracle

A **reference-capture oracle**: it runs the *original* Heroes Lore: Wind of
Soltia v2.0.7 JAR on a *different, independent* J2ME reimplementation
(FreeJ2ME-Plus) headless, and captures one PNG per labelled `shot`. Those frames
are the reference side of a pixel-exact gate for the future Rust transliteration:
the port, driven by the *same route files*, must reproduce each frame
**byte-for-byte**.

This game is **pure 2D** (`javax.microedition.lcdui.Graphics` — no M3G, no 3D, no
lighting), so there is nothing to disagree about between two correct renderers:
the only clean state is `differing_pixels == 0`.

> **The reference is a second opinion, never ground truth.** An emulator is not
> the handset. FreeJ2ME-Plus is one reimplementation of the MIDP/LCDUI/RMS/MMAPI
> firmware the JAR never shipped; the Rust port is another. Running the original
> bytecode on FreeJ2ME is the only independent check we have on the *platform*
> half of the port — but where they disagree, neither is automatically right.

Everything here is modelled on **stalker-mobile**'s 3D oracle
(`tools/transliteration/`), adapted to 2D. Nothing third-party is committed: the
emulator is cloned and built into a git-ignored scratch dir, and all captures
land under git-ignored `_reference/oracle/`.

---

## Layout

```text
tools/oracle/
  capture_reference.sh     build FreeJ2ME-Plus + run the JAR headless + capture PNGs
  HeadlessCapture.java     the headless capture driver (painter -> PNG, key inject, RNG seed)
  compare_frames.py        the EXACT, in-process per-label comparator (+ inline self-test)
  agreement.toml           the ratchet: recorded per-label agreement (port pending)
  routes/
    00-boot.txt            publisher splash -> title logo -> main menu
    01-menu.txt            the main-menu carousel, one shot per selectable entry
  patches/
    freej2me-plus-deterministic-clock.patch
    freej2me-plus-deterministic-input.patch

_reference/oracle/          (git-ignored) reference/{pass-N}/{route}/{label}.png + manifest.tsv
```

## Run it

Inside the dev shell (`nix develop`):

```sh
just java-me-frames         # build the emulator + capture the reference (2 passes)
just java-me-frames-check   # verify the reference + prove the comparator bites
```

`java-me-frames-check` runs the comparator's **inline self-test** while the port
does not exist yet. Once a port capture lands under `_reference/oracle/port/`,
the same recipe runs the full exact comparison against `agreement.toml`.

---

## 1. Building FreeJ2ME-Plus

- **Repo:** `https://github.com/TASEmulators/freej2me-plus.git` (game-agnostic; the
  one constant `capture_reference.sh` still hard-codes, overridable via an optional
  `[oracle].freej2me_repo`)
- **Pinned commit:** `373c3636042a5ae80c745647e08d3633d08a95eb` — read from
  `game.toml [oracle].freej2me_commit`, not hard-coded in the script. Bump it there.
- **Build system:** the repo ships an Ant `build.xml`, but — following the
  sibling oracle — we compile the `src/` tree **directly with `javac`** (skipping
  `src/libretro/`), which is simpler and needs no Ant. `-source 8 -target 8`.
- **JDK:** a **non-headless** JDK 21 (`nixpkgs#jdk`, openjdk-21). FreeJ2ME's
  `PlatformFont` needs AWT font metrics that the dev shell's `jdk17_headless`
  lacks (no `libfontmanager`), so `capture_reference.sh`'s `find_java` **probes**
  every candidate JDK with a tiny `FontProbe` and **rejects** `JAVA_HOME` if it
  fails — it never trusts it. The flake dev shell now also realises `jdk` so the
  probe finds it by store path even offline.

The build reconciles: **1358 classes** compile clean; all key classes
(`MobilePlatform`, `Mobile`, `MIDletEnhancements`, `PlatformImage`) present.

### Patches: two, not four

The sibling 3D oracle carried **four** local patches. Two are M3G-only and this
pure-2D game never touches `javax.microedition.m3g`, so they are **dropped**:
`group-duplicate` and `node-composite-transform`. The **two kept** are not
M3G-specific:

| patch | touches | why |
|---|---|---|
| `freej2me-plus-deterministic-clock.patch` | `MIDletEnhancements.java` | Adds an off-by-default hook to *freeze* FreeJ2ME's substituted clock and advance it by each route command's exact ms budget. FreeJ2ME's bytecode rewriter already routes the game's `System.currentTimeMillis()`/`nanoTime()` through `MIDletEnhancements`, so this makes animations a function of frame count, not host speed. |
| `freej2me-plus-deterministic-input.patch` | `Display.java` | Adds an off-by-default hook to drain the Canvas key callback at the route boundary, killing host-thread input races. Its **upstream comment even names "Heroes Lore: Wind of Soltia"** as a game that spams the input queue. |

Both apply cleanly at the pinned commit (`git apply --ignore-space-change`), and
the build still compiles to 1358 classes with them applied.

## 2. The capture driver (`HeadlessCapture.java`)

Lifted near-verbatim from the sibling's `HeadlessCapture.java`. It uses
FreeJ2ME's own public API (`Mobile` / `MobilePlatform`) in place of the AWT
frontend: installs a painter that copies the front buffer, loads the JAR
(`rpg.GameMIDlet` via the manifest), runs the MIDlet on its own thread, and
executes a route script. Kept: painter→PNG on `shot`, Nokia keycode injection,
the deterministic clock + gated frame stepping, and RNG-by-reflection. Dropped:
the M3G switches and the sibling's game-specific obfuscated-field snapshot.

Two adaptations forced by *this* game:

- **`--sound 1` is required.** With sound off, the loader's audio-player thread
  throws on an UNREALIZED player (`getControl()` on a CLOSED/UNREALIZED player)
  and the boot sequence **deadlocks on a white screen**. With sound on the thread
  stays alive and the game boots; there is no audio device, so nothing is
  emitted. `game.toml [oracle].sound = 1` drives the `--sound` flag (the script
  reads it; it is not hard-coded).
- **RNG seeding is generalised.** The sibling named one obfuscated class; this
  game keeps its RNGs as `public static Random a` in classes **`ck`** and **`h`**
  (`h.a(int,int)` is a random-range helper). So `seed <n>` walks the archive's
  class list, asks the game's own loader which classes are loaded, and reseeds
  **every** static `java.util.Random` it finds — game-agnostic.

### Determinism: why routes carry `fps` + `seed`

The title screen has an animated element (fluttering birds) driven by the game's
RNG. Two things are needed to make its captured frame reproducible:

1. `fps 20` switches the driver to **gated frame stepping** on the frozen clock
   (and, as a side effect, loads the utility classes so `ck.a`/`h.a` exist).
2. `seed <n>` **reseeds** those RNGs in place. It must run *after* the classes
   load and *before* the animation consumes randomness — so routes seed once
   right after `fps`, and again just before the animated title `shot`.

With both, all captured labels are **byte-identical across the two passes**
(proven by the harness's own stability check).

## 3. Routes

One command per line; `#` is a comment; unknown `k=v` tokens are ignored so the
same file can drive the emulator and (later) the port. Verbs: `wait`, `tap`,
`hold`/`down`, `release`/`up`, `seed`, `fps`, `shot`, `echo`.

- **`00-boot.txt`** → `publisher-splash`, `title-logo`, `main-menu-new-game`.
- **`01-menu.txt`** → `menu-new-game`, `menu-options`, `menu-help`, `menu-about`,
  `menu-exit`. On a fresh install there is **no saved game, so `LOAD GAME` is
  disabled**: `DOWN` from `NEW GAME` skips straight over it to `OPTIONS`. Capturing
  that skip is deliberate — it is a real state decision, a good witness for the port.

Each route is a *fresh install* (FreeJ2ME writes its RMS store beside the process,
and every run uses a fresh working dir), so the sound prompt/menu appear as they
would on first launch.

## 4. The comparator (`compare_frames.py`)

2D-only descendant of the sibling's comparator. The PNG codec (dependency-free —
the dev shell has no Pillow/numpy), route reader, fail-closed provenance check,
cross-pass stability check and ratchet are lifted; the entire 3D
structural-attribution machinery is **deleted**. The comparison is
`compare_exact`: it walks the two raw pixel arrays and counts differing pixels.
`differing_pixels == 0` is the only clean state.

### How the diff is computed — and blind spots we design against

These are the ways the *sibling* oracle went green over real bugs. Each has a
countermeasure wired in, asserted on **every run** (not as a separate optional
test):

1. **Vacuous comparator (their worst).** The sibling shelled out to
   `magick compare -metric AE` and a parse bug scored a *wholly different* image
   as "6 pixels differ" — every golden vacuous for weeks. So we compute the diff
   **in-process**: decode both PNGs ourselves, compare the pixel arrays directly.
   **No `magick`/ImageMagick/AE proxy is ever invoked.** And we do not trust that
   code: an **inline self-test** runs each invocation — *ref-vs-ref must be 0*,
   *ref-vs-one-perturbed-pixel must be exactly 1*, and *two genuinely different
   labels must differ by many pixels* (the direct guard against scoring different
   images as near-identical). Any deviation **fails the whole run**.
2. **Stale / unverified captures.** The sibling silently answered inputs that no
   longer existed because nobody read the manifest. So we SHA-256 the **jar**,
   every **route file**, and the **emulator build** (commit + patch hashes) into
   `manifest.tsv`, and **fail closed** on any drift from the repo *at compare
   time*, before comparing a pixel.
3. **The reference is not ground truth.** The sibling's FreeJ2ME booted with the
   wrong key for `ENABLE_SOUND`, so every sound play was silently refused → a
   frozen, vacuous 0-vs-0. So we check the reference frames actually *exercised*
   the game: each is **non-blank** (a distinct-colour floor) and each route's
   frames are **not all identical** (the sequence advanced — not a frozen boot).
   (This game only boots at all under `--sound 1`; see §2.)
4. **Non-vacuity floor per frame.** Every captured reference frame must carry
   ≥ 16 distinct colours, so a blank/frozen capture can never read as a valid
   reference.

### The ratchet (`agreement.toml`)

Records each label's current agreement. A run fails on a **regression** and
equally on an **unrecorded improvement**, so a number can only move by someone
looking at the diff image and running `--update-ratchet`. Until the port is
captured, every label is recorded `verdict = "port-missing"`.

### Self-proof (what `just java-me-frames-check` shows today)

```
provenance : PASS (jar + routes + emulator build match the repo)
self-test  : PASS (8 labels; one-pixel diff detected; most-different pair differs by 76784 px)
non-vacuity: PASS (8 frames >= 16 colours; 2 route(s) advanced)
reference  : 2 passes, 0 unstable labels
```

Proven against a synthetic port: **ref-vs-ref → 0 differing pixels on all 8
labels** (exit 0); **ref-vs-perturbed → the perturbed label reported and the
ratchet gate fails** (exit 1).

---

## Reusability split

This oracle seeds the home `_template/` so every 2D J2ME port gets it for free.
**Every per-game knob has been lifted out of the scripts and into `game.toml`'s
`[oracle]` section**, so all three scripts under `tools/oracle/` now carry *no
game-specific literal* — `grep -E 'Soltia|v207|240|320|rpg\.'` over
`capture_reference.sh` and `compare_frames.py` is empty. `new-game.py` therefore
**stamps `tools/oracle/` into a new port unchanged**; onboarding a game means
filling in `game.toml [oracle]` and writing its routes, nothing more.

### The per-game surface

Three things, all in *this* repo — nothing in the scripts:

1. **`game.toml [oracle]`** — the knobs the scripts read on every run:
   ```toml
   [oracle]
   jar = "_originals/Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar"  # + its sha256 lands in the manifest
   canvas_w = 240
   canvas_h = 320
   sound = 1                                        # this game deadlocks on a white screen with sound off
   patches = ["deterministic-clock", "deterministic-input"]  # → tools/oracle/patches/freej2me-plus-<name>.patch
   freej2me_commit = "373c3636042a5ae80c745647e08d3633d08a95eb"
   ```
   `capture_reference.sh` reads them with a tiny `tomllib` reader (the same
   library `tools/originals/corpus_common.py` uses); `compare_frames.py` reads
   `[oracle].jar` for its default `--jar`.
2. **The routes** (`routes/*.txt`) — the game-specific screens & keystrokes.
3. **The patch *files*** (`patches/*.patch`) — the emulator hooks a given game
   needs. This game selects the two non-M3G hooks; another may select none, or a
   different set. `game.toml` *names* which to apply; the files live here.

### What is game-agnostic (→ home `_template/tools/oracle/`, stamped unchanged)

| Component | Why it is generic |
|---|---|
| `capture_reference.sh` | clone/pin/build/`find_java`/stamp/manifest flow; sources every knob from `game.toml [oracle]`, scratch dir derived from `slug` |
| `HeadlessCapture.java` | painter→PNG, keycode inject, gated clock; RNG seeding scans loaded classes instead of naming one, so it needs no per-game class name |
| `compare_frames.py` | in-process exact diff, inline self-test, fail-closed provenance, non-vacuity/liveness floors, ratchet — no game-specific structural code |
| The route-script format + verb semantics, and the blind-spot countermeasures (1–4 above) | shared by construction |

The generalisations that make this possible — RNG seeding by scanning loaded
classes rather than naming one, the comparator being 2D-exact with no structural
code, and (this change) **all remaining knobs moving to `game.toml [oracle]`** —
are exactly what the template promotes. The **JAR, canvas size, `--sound` quirk,
patch selection, and pinned commit** are now data in `game.toml`; the **routes and
patch files** are the only per-game artefacts that live beside the scripts.
