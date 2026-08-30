//! Map a `winit` physical key to the raw Nokia/MIDP code the game's
//! `keyPressed(int)` expects, via the generic **`j2me-input`** crate — a
//! configurable, remappable mapping shared by every port, replacing the old
//! hard-coded winit->Nokia table this file used to carry.
//!
//! ## The mapping (default `Preset::Mobile`)
//!
//! `j2me-input`'s `Preset::Standard` is **code-for-code identical** to the table
//! this host used before (verified key by key):
//!
//! | keyboard                              | Nokia key        | code   |
//! |---------------------------------------|------------------|--------|
//! | Arrow Up / Down / Left / Right        | D-pad            | -1..-4 |
//! | Enter, Space, Numpad Enter, Numpad 5  | Fire / select    | -5     |
//! | F1 / F2                               | left/right soft  | -6/-7  |
//! | 0–9 (top row or numpad)               | number keys      | 48–57  |
//! | `[` / Numpad `*`                      | star `*`         | 42     |
//! | `]` / `\`                             | pound `#`        | 35     |
//!
//! The default preset here is `Mobile`, a **strict superset** of `Standard` that
//! also binds a left-hand cluster (**W/A/S/D** move, **X** fire, **Q/E** soft keys,
//! **R/F** = `*`/`#`). Every code the old table produced is unchanged; Mobile only
//! *adds* bindings for keys the old table left unmapped (so no existing input
//! behaviour shifts). `Escape` stays unmapped and is handled by the window shell
//! as "quit", exactly as before.
//!
//! ## Remapping without recompiling
//!
//! An optional `[keymap]` table — in the repo's `game.toml`, or a standalone
//! `keymap.toml` alongside it — selects a preset and layers per-key overrides on
//! top (bind by action name or raw Nokia code, or `"none"` to unbind). See the
//! `j2me-input` crate docs for the format. Absent, the default Mobile preset is
//! used. The raw digit/`*`/`#` codes still originate in the generic device runtime
//! (`j2me-device-nokia`, re-exported through `j2me-input`), the negative D-pad /
//! Fire / soft-key codes too — so the phone-keypad contract now has ONE generic
//! home instead of the copy that used to live here.

use std::sync::OnceLock;

use j2me_input::{Keymap, Preset};
use winit::keyboard::KeyCode;

/// The process-wide keymap, built once from the optional `[keymap]` override
/// (falling back to the default `Mobile` preset). Reading a config file is a
/// one-time host setup cost, so it is memoised here.
fn keymap() -> &'static Keymap {
    static KM: OnceLock<Keymap> = OnceLock::new();
    KM.get_or_init(|| match load_keymap_config() {
        Some(cfg) => Keymap::from_config(Some(&cfg), Preset::Mobile).unwrap_or_else(|err| {
            eprintln!("heroes-lore-wind-of-soltia: keymap config ignored ({err}); using Mobile");
            Keymap::new(Preset::Mobile)
        }),
        None => Keymap::new(Preset::Mobile),
    })
}

/// Find the keymap config: a standalone `keymap.toml`, else the repo `game.toml`
/// (whose `[keymap]` table, if any, is read and every other section ignored),
/// walking up from this crate to the workspace root. `None` if neither is found.
fn load_keymap_config() -> Option<String> {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        for name in ["keymap.toml", "game.toml"] {
            let candidate = dir.join(name);
            if candidate.is_file() {
                if let Ok(text) = std::fs::read_to_string(&candidate) {
                    return Some(text);
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// The raw Nokia code for a physical key, or `None` if the active keymap leaves it
/// unbound. Drop-in replacement for the former hand-rolled table — same signature,
/// same codes for the previously-mapped keys, now generic + remappable.
pub fn nokia_code(key: KeyCode) -> Option<i32> {
    keymap().nokia_code(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use j2me_input::Keymap;
    use winit::keyboard::KeyCode as K;

    /// The OLD hard-coded contract is preserved byte-for-byte under the default
    /// Mobile preset — this is the "does the generic mapping actually match what
    /// the game needs" guard: every key the former table mapped still yields the
    /// same raw Nokia code, so the swap hides no incompatibility.
    #[test]
    fn generic_mapping_matches_the_former_hardcoded_table() {
        let km = Keymap::new(Preset::Mobile);
        // D-pad, Fire (incl. Numpad5-is-Fire / Digit5-is-digit), soft keys.
        assert_eq!(km.nokia_code(K::ArrowUp), Some(-1));
        assert_eq!(km.nokia_code(K::ArrowDown), Some(-2));
        assert_eq!(km.nokia_code(K::ArrowLeft), Some(-3));
        assert_eq!(km.nokia_code(K::ArrowRight), Some(-4));
        assert_eq!(km.nokia_code(K::Enter), Some(-5));
        assert_eq!(km.nokia_code(K::Space), Some(-5));
        assert_eq!(km.nokia_code(K::NumpadEnter), Some(-5));
        assert_eq!(km.nokia_code(K::Numpad5), Some(-5));
        assert_eq!(km.nokia_code(K::F1), Some(-6));
        assert_eq!(km.nokia_code(K::F2), Some(-7));
        // Digits and symbols.
        assert_eq!(km.nokia_code(K::Digit0), Some(48));
        assert_eq!(km.nokia_code(K::Digit5), Some(53)); // top-row 5 stays a digit
        assert_eq!(km.nokia_code(K::Digit9), Some(57));
        assert_eq!(km.nokia_code(K::BracketLeft), Some(42));
        assert_eq!(km.nokia_code(K::NumpadMultiply), Some(42));
        assert_eq!(km.nokia_code(K::BracketRight), Some(35));
        assert_eq!(km.nokia_code(K::Backslash), Some(35));
        // Escape stays unmapped → the shell keeps it as "quit".
        assert_eq!(km.nokia_code(K::Escape), None);
    }

    /// The NEW capability the adoption adds: the Mobile left-hand cluster (a strict
    /// superset — none of these keys were mapped by the old table).
    #[test]
    fn mobile_adds_the_wasd_cluster() {
        let km = Keymap::new(Preset::Mobile);
        assert_eq!(km.nokia_code(K::KeyW), Some(-1)); // up
        assert_eq!(km.nokia_code(K::KeyA), Some(-3)); // left
        assert_eq!(km.nokia_code(K::KeyS), Some(-2)); // down
        assert_eq!(km.nokia_code(K::KeyD), Some(-4)); // right
        assert_eq!(km.nokia_code(K::KeyX), Some(-5)); // fire
        assert_eq!(km.nokia_code(K::KeyQ), Some(-6)); // soft left
        assert_eq!(km.nokia_code(K::KeyE), Some(-7)); // soft right
    }

    /// A `[keymap]` override remaps a key without recompiling (the configurable
    /// remapping this whole change is for).
    #[test]
    fn config_override_rebinds_a_key() {
        let cfg = "[keymap]\npreset = \"mobile\"\nKeyH = \"SoftLeft\"\nKeyX = \"none\"\n";
        let km = Keymap::from_config(Some(cfg), Preset::Mobile).expect("valid config");
        assert_eq!(km.nokia_code(K::KeyH), Some(-6)); // H bound to left soft key
        assert_eq!(km.nokia_code(K::KeyX), None); // X unbound (was Fire in Mobile)
        assert_eq!(km.nokia_code(K::ArrowLeft), Some(-3)); // base bindings intact
    }
}
