//! Transliterated from `java/src/main/java/defpackage/Debug.java`
//! (original `x.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Tiny assertion/build-flag helper. Holds the global full-version flag (mirrored
//! from [`crate::app_config::AppConfigState::full_version`], which gates sound, the
//! buy prompt and the level-8 endgame content) and provides [`assert_true`], the
//! check used across the engine.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `x.<clinit>:()V => []`
//! and `x.a:(Z)V => []` (no arithmetic).

/// Java `x` / `Debug` state: the single `static boolean fullVersion`.
///
/// Its `<clinit>` (byte-verified against `javap -c -p x.class`) sets
/// `fullVersion = false`, then builds a local array `{7, 9, 13, 16, 19}` and calls
/// `System.currentTimeMillis()` discarding the result — dead residue with no
/// effect (`docs/TRANSLITERATION.md`, *No-ops*), so [`Default`] reproduces only the
/// observable `fullVersion = false`.
#[derive(Debug, Default)]
pub struct DebugState {
    /// `public static boolean fullVersion = false;` (`x.a:Z`). Copied from
    /// `AppConfig.fullVersion` at startup by `AppConfig.init`.
    pub full_version: bool,
}

/// `public static final void assertTrue(boolean condition) throws RuntimeException`
/// (`x.a:(Z)V`). Throws `RuntimeException("ASSERT FAILED")` when `condition` is
/// false. The throw is uncaught at the engine call sites, so a Rust panic is the
/// faithful termination.
pub fn assert_true(condition: bool) {
    // if (!condition) throw new RuntimeException("ASSERT FAILED");
    if !condition {
        panic!("ASSERT FAILED"); // Debug.java:21
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_full_version_is_false() {
        // `<clinit>` sets fullVersion = false.
        assert!(!DebugState::default().full_version);
    }

    #[test]
    fn assert_true_passes_when_condition_holds() {
        assert_true(true); // must not panic
        assert_true(1 + 1 == 2);
    }

    #[test]
    #[should_panic(expected = "ASSERT FAILED")]
    fn assert_true_panics_when_condition_is_false() {
        assert_true(false);
    }
}
