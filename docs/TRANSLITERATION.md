# Java-to-Rust transliteration contract — Heroes Lore: Wind of Soltia

This is the binding guide for **implementation #1** of the Rust game: a strict,
mechanical transliteration of `java/src/main/java/defpackage/` into the crate
`hlws-game-xlat`.

Implementation #1 exists to be *provably the same program*, not to be good Rust.
It is the executable specification a later idiomatic implementation (#2) is
validated against, so its only virtues are fidelity and speed of construction.
**Do not make it pretty. Do not refactor it. Do not "improve" a name or fold a
redundant operation.** Every rule below exists because breaking it silently
changes behaviour.

This game is **pure 2D and pure integer**: `javax.microedition.lcdui.Graphics`,
no M3G, no floats. The numeric authority (`_reference/numeric_shapes.json`) found
**zero float/double opcodes** across all 90 classes / 944 methods. Consequently
`hlws-jvm` ships **no** float helpers, and none appear in transliterated code. If
a `float`/`double` ever surfaces, stop — it is a decompilation error, not a
licence to invent an `f32`.

## Scope

| Layer | Crate | Style |
| --- | --- | --- |
| Game classes | `hlws-game-xlat` | Transliteration; this document |
| Device runtime | `hlws-me` | Idiomatic Rust implementing the Java ME contracts |
| Neutral 2D buffer | `hlws-canvas` | Idiomatic Rust ARGB framebuffer |
| Java primitives | `hlws-jvm` | Idiomatic Rust implementing JVM integer semantics |
| Host | `apps/hlws-linux` | Idiomatic Rust |

Only `hlws-game-xlat` is transliterated. The layers beneath it are ordinary Rust
whose *observable behaviour* must match the Java ME / JVM specification. The
independent format parsers in `hlws-formats` (`crates/formats`) are **not** part
of the port — they are a second, separately-derived implementation used only as
an oracle (see *Cross-check oracles*).

## The single most important rule

**Java promotes `byte`, `short` and `char` to `int` before arithmetic.** A cast
back is a *narrowing of an `int` result*, not saturating byte arithmetic.

```java
// Crc32.java:41 — n : short
n = (short) (n + 1);
```
```rust
n = (n as i32 + 1) as i16;   // widen to int, add, narrow back to short
```

Writing `n + 1` on an `i16` is wrong twice over: it panics at `i16::MAX` in debug
builds, and wraps at a different width than Java in release. `as i32` first,
narrow last, always. `byte` narrows with `as i8`, `short` with `as i16`, `char`
with `as u16`.

## Primitive mapping

| Java | Rust | Notes |
| --- | --- | --- |
| `boolean` | `bool` | |
| `byte` | `i8` | signed; `byte[]` is **`Vec<i8>`**, never `Vec<u8>`; a `byte[]` param is `&[i8]` / `&mut [i8]` |
| `short` | `i16` | |
| `char` | `u16` | **unsigned** 16-bit; a `char[]` is `Vec<u16>`; in arithmetic it **zero**-extends to `int` |
| `int` | `i32` | |
| `long` | `i64` | |
| `String` | `String` / `&str` | only resource paths and PNG chunk-type tags here |
| `T[]` | `Vec<T>` | fixed length after construction; never `push` unless Java does |
| `Object[]` (nullable) | `Vec<Option<T>>` | e.g. `PngMerger.mpdData` — lazily-loaded `byte[]` or `null` |
| reference | field on a `*State` struct / `Option<Handle>` | see *Statics and ownership* |
| `null` | `None` | |

`byte[]` as `Vec<i8>` is load-bearing: `PngMerger.mirrorScanlines` compares raw
scanline bytes against `0` and shifts signed pixel nibbles; the `.mpd` PNG
signature is `{-119, 80, …}`. Using `u8` changes comparison and sign-extension
results. A read of `data[i]` yields an `i8`, exactly as the JVM's `baload`
sign-extends into an `int`; `data[i] & 255` then re-zero-extends. Convert to `u8`
**only** at the true host boundary (handing bytes to a PNG decoder, hashing).

## Arithmetic

**Never a bare operator on an integer.** Route every integer op through
`hlws-jvm`:

| Java op | Rust |
| --- | --- |
| `+ - *` | `wrapping_add` / `wrapping_sub` / `wrapping_mul` |
| `/` | `hlws_jvm::java_div(a, b)?` (`long`: `java_ldiv`) — traps `i32::MIN / -1` |
| `%` | `hlws_jvm::java_rem(a, b)?` (`long`: `java_lrem`) |
| `<<` `>>` | `hlws_jvm::ishl` / `ishr` (`long`: `lshl` / `lshr`) — Java masks the count (5 bits int, 6 bits long) |
| `>>>` | `hlws_jvm::iushr` (`long`: `lushr`) |
| `& \| ^` | direct (`&` `\|` `^`) — bitwise ops do not overflow |

A debug-only panic where Java wrapped is a divergence; release-mode silence hides
it. `as i32` first, narrow last. `java_div`/`java_rem` return
`Result<_, JavaError>`: at a site the original did not guard, `.expect(...)` on a
provably-non-zero constant divisor (a panic there is the faithful
`ArithmeticException`); inside a `try/catch` region, propagate with `?`.

Where a divisor is a nonzero **constant** (e.g. `% 65521`, `/ 3`), routing
through `java_rem`/`java_div` and unwrapping is still required — uniformity keeps
review mechanical and the one real `i32::MIN / -1` site from hiding.

`Math.abs(int)` is **not** `i32::abs`: `Math.abs(i32::MIN)` returns `i32::MIN`
(overflow), Rust's panics. Transliterate as `if x < 0 { x.wrapping_neg() } else { x }`.

### Opcode-shape authority (R8)

`_reference/numeric_shapes.json` is the ordered list of arithmetic/conversion
opcodes for every original method, extracted from the shipped `.class` files (the
R8 authority). Before transliterating a method, read its shape and confirm the
decompiled Java's arithmetic **multiset and structure** match it. The Java source
is what you transliterate; the shape is the guard that the decompilation is
faithful. A divergence in the *multiset* (an op present in one and not the other)
is a blocker; a divergence only in javac's internal *evaluation order* (e.g. an
`iinc` the decompiler rendered as `x + 1`) is expected and is not — you
transliterate the Java expression verbatim, preserving its parenthesisation.

## Statics and ownership

Java `static` fields become fields on one `*State` struct per class; a top-level
`Game` struct aggregates them. Methods become **free functions** taking the state
by `&mut`, never `self`-methods — `self` methods conflict the moment a method
needs two sub-structs at once.

```java
public final class Adler32 {
    private int sum = 1;
    public final void update(byte[] data, int offset, int length) { … }
}
```
```rust
pub struct Adler32State { pub sum: i32 }
impl Default for Adler32State { fn default() -> Self { Self { sum: 1 } } }
pub fn update(s: &mut Adler32State, data: &[i8], offset: i32, length: i32) { … }
```

A purely-static utility class (no instance/`static` mutable state — `ByteUtil`'s
byte helpers, every `PngMerger` static) becomes **free functions with no state
parameter**. Only the mutable state of a class needs a `*State` struct
(`ByteUtil.rng`, `Adler32.sum`, `Crc32.crc`, the `PngMerger` instance fields).

As stateful classes land, `ownership.tsv` will assign every Java field to a
single Rust owner and type; consult it before porting a class and never invent a
field's home. One Java static has exactly one persistent Rust owner: no second
copy, no copy-in/copy-out.

### Accepted deviations

Any structural deviation is recorded here before it is written.

- **Shared static checksum engines.** `PngMerger.crc` / `PngMerger.adler` are
  `static` singletons, but every use is immediately preceded by `.reset()`. A
  freshly-constructed `Crc32State` / `Adler32State` is *bit-identical* to a reset
  one, so the transforms construct a local engine instead of threading the
  static. Behaviour is unchanged; the alias is not observable.
- **Deferred resource/`Image` boundary.** `PngMerger.load` / `readIndex` /
  `loadMpd` read bytes through `AssetCache.readResource`, and
  `image` / `imageMirrored` / `imageGray` / `allImages` wrap the result in
  `javax.microedition.lcdui.Image` (+ `BaseCanvas.yieldTick`). Those cross into
  as-yet-unported classes and are **deferred** with an explicit marker. The
  transliterated decoder core (header parse, assembly, transforms) is complete
  and driven directly from injected bytes by the oracle.

## Overloads

Rust has no method overloading. When two Java methods share a name, the primary
keeps the base `snake_case` name and the secondary is disambiguated by a suffix
naming what distinguishes it. `PngMerger.applyEffect(byte[],int)` (a convenience
delegating to the three-arg form) becomes `apply_effect_default`; the primary
`applyEffect(byte[],int,int)` is `apply_effect`. Where the original recovered an
overload only in raw-obfuscated code, the raw form gets a `_code` suffix and the
semantic form keeps the base name.

## Naming

`camelCase` → `snake_case`, mechanically. Constants stay `SCREAMING_SNAKE_CASE`.
Keep the recovered semantic names from the named-Java crosswalk — they are
reviewed evidence, not suggestions. Do not rename, abbreviate, or "improve" a
name during transliteration.

Each ported file opens with a provenance header naming the Java file and the
original obfuscated `.class`:

```rust
//! Transliterated from `java/src/main/java/defpackage/Adler32.java`
//! (original `an.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
```

## Exceptions

`hlws_jvm::JavaError` enumerates the raiseable exceptions; `JavaResult<T>` is the
alias.

- A method the original wraps in `try { } catch (Exception e) { … }` returns
  `Result<_, JavaError>`, and the call site does exactly what the catch block did.
- A method the original does **not** guard may panic. That is faithful: the
  MIDlet would have died too. An explicit `throw new ArrayIndexOutOfBoundsException()`
  at a leaf (`ByteUtil.readU16`, `PngMerger.readU32`) is transliterated as a
  `panic!` reproducing the exact bounds predicate — an uncaught throw terminates.
- Inside a guarded region, array access goes through `jget!` / `jset!`; outside
  one, index directly (`arr[i as usize]`), which panics on a bad index exactly as
  `baload`/`aaload` would.

## Preserved defects

Every one of these is deliberate. Each carries a comment naming the Java file and
line. A reviewer who "fixes" one has broken the port.

| Site | Defect |
| --- | --- |
| `Adler32.java:57` | `getValue()` returns `((long) sum) & (-1)`. The `-1` is an `int` promoted to `-1L`, so the mask **sign-extends** the 32-bit state — a negative `long` results whenever bit 31 of `sum` is set, instead of the zero-extended unsigned checksum. `PngMerger` casts back to `int`, which undoes it, so the defect is latent; it is preserved verbatim as `(sum as i64) & (-1i64)`. |
| `Adler32.java:20` | `update` accumulates the two running sums into **signed** `int`s while folding up to the classic `NMAX = 5552` bytes before each `% 65521` reduction — but `NMAX` was derived for an *unsigned* 32-bit accumulator. On a large or high-byte input `sumB` overflows `i32::MAX`, and Java's signed `%` then yields a value that diverges from true RFC-1950 Adler-32. Reproduced exactly (wrapping arithmetic + signed `java_rem`). Latent in practice: the engine only ever folds a single small per-frame IDAT (well under `NMAX`), where Java == RFC. Pinned by `adler32_oracle::adler32_reproduces_the_java_algorithm_including_signed_overflow` (transliteration equals an independent signed-block reference, and differs from clean RFC, on a 20 000-byte input). |
| `ByteUtil.java:64` | `writeI32` computes `int v = value & (-1)` — a redundant identity mask left in the shipped code. Preserved as `let v = value & (-1);`. |

## No-ops

`System.gc()`, `Thread.yield()`, `e.printStackTrace()` and discarded allocations
(`GameplayScreen`-style dead `new`s, `Debug`'s dead static array) become nothing
or a single log line — **unless** ordering between them is observable, in which
case the surrounding order is preserved (none among the current leaf decoders).
`PngMerger.unloadAllMpd` interleaves no ordered clears, so its `System.gc()` is a
plain no-op.

## Cross-check oracles ("two implementations, one truth")

A class is not done when it compiles — compilation is not equivalence. Each
decoder is gated by an **independent** second implementation over **real** blobs
from `_originals/…v207.jar`, following the gothic `*_oracle.rs` pattern and the
project rulebook (`docs/GATES.md`, R1/R3):

- **`Adler32` / `Crc32`** — diff the transliterated checksum against an
  independent reference (a structurally-different in-test reimplementation) **and**
  published test vectors, over random inputs and real IDAT/asset bytes. Both must
  agree byte-for-byte.
- **`PngMerger`** — reconstruct **every** atlas frame and cross-check against
  `hlws-formats`' independent `mpd`/`mph` parse: the reconstruction's frame
  offset must equal the `.mph` record offset and land on a real `.mpd` `IHDR`,
  and every reassembled frame PNG must actually **decode** (liveness).

Every oracle carries:

- **Liveness** — assert real work happened (blobs processed, frames decoded),
  never `0`. A decoder that processes 0 blobs must **fail**, not pass vacuously.
- **Count floors** — the non-vacuity floor for the atlas corpus is **≥ 170
  `.mpd`** and **≥ 1800 frames** (baseline: 179 / 1831).
- **A negative control** — a one-unit perturbation (a corrupted byte, a
  cross-frame mismatch) proven to turn the gate red, so an agreement that
  survived a real mismatch cannot read as a pass.
- **Loud failure when `_originals/` is absent** — never a skip that reads green.

## Verification

A class is done when its cross-check oracle passes over the real corpus, its
opcode multiset reconciles with `_reference/numeric_shapes.json`, and every
deviation and preserved defect is recorded here.
