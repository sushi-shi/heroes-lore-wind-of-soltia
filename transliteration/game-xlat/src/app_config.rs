//! Transliterated from `java/src/main/java/defpackage/AppConfig.java`
//! (original `w.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Reads the game's build/deployment configuration out of the JAD/manifest app
//! properties and exposes it as static flags: full vs. demo build (the `HO-Demo`
//! unlock token), which locales are offered (`HO-LangList`), and whether an
//! in-game "buy the full game" link appears (`HO-BuySetup` + a per-locale
//! `HO-URL-xx`). Not the settings RMS store — read-only launch config.
//!
//! Static-state class → one [`AppConfigState`] owned by [`Game`](crate::game)
//! (`ownership.tsv`). Cross-class seams:
//! * `midlet.getAppProperty(name)` — the sole use of the `midlet` static — is
//!   modelled by [`AppConfigState::app_properties`], the JAD property map; see
//!   [`AppConfigState::get_app_property`].
//! * `StringTable.instance.locales` (populates `locales`) and
//!   `StringTable.instance.localeIndex` (the current locale) are `StringTable`
//!   (unported): `locales` is a settable field; `localeIndex` is passed in.
//! * `FontManager.*` writes in [`apply`] and the `StringTable.instance.get(...)`
//!   label fallbacks in [`resolve_buy_label`] are **deferred** cross-class
//!   boundaries (`docs/TRANSLITERATION.md`, *Accepted deviations*) — neither
//!   carries arithmetic, so no opcode-shape fidelity is lost.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `w.a:()I (initAvailableLocales) => ["iinc","iinc","iinc"]`,
//! `w.a:()Ljava/lang/String; (resolveBuyLabel) => ["iinc","iadd","iadd","isub","i2c","iadd"]`,
//! and `[]` for every other method (`init`, `apply`, `isFullVersion`,
//! `showFullVersionLink`, `showDemoBuyLink`, `resolveBuyUrl`, `isBuySetupEnabled`).

use crate::debug::DebugState;
use std::collections::HashMap;

/// The demo-unlock token compared against `HO-Demo`.
const DEMO_UNLOCK_TOKEN: &str = "BEJ8K52N7A";

/// Java `w` / `AppConfig` static state (declaration order preserved).
#[derive(Debug, Default)]
pub struct AppConfigState {
    /// Stand-in for `static MIDlet midlet` — the JAD app-property map behind every
    /// `midlet.getAppProperty(name)` lookup (`ownership.tsv`: `midlet`).
    pub app_properties: HashMap<String, String>,
    /// `static String[] locales` — supported locale codes (from `StringTable`).
    pub locales: Vec<String>,
    /// `static String buyUrl` — resolved per-locale buy URL, or `None`.
    pub buy_url: Option<String>,
    /// `static boolean menuBuyEnabled` (`w.a:Z`).
    pub menu_buy_enabled: bool,
    /// `static boolean exitBuyEnabled` (`w.b:Z`).
    pub exit_buy_enabled: bool,
    /// `static boolean fullVersion` (`w.c:Z`).
    pub full_version: bool,
    /// `static boolean[] localeEnabled` — per-locale availability mask.
    pub locale_enabled: Vec<bool>,
}

impl AppConfigState {
    /// `midlet.getAppProperty(name)` — returns the JAD property value or `null`
    /// (→ [`None`]).
    pub fn get_app_property(&self, name: &str) -> Option<String> {
        self.app_properties.get(name).cloned()
    }
}

/// `String.trim()` — removes leading and trailing characters `<= ' '` (` `).
/// Operates on UTF-16 code units in the JDK; the config tokens here are ASCII, so
/// a `char`-based trim is identical.
fn java_string_trim(s: &str) -> String {
    s.trim_matches(|c: char| (c as u32) <= 0x20).to_string()
}

/// `String.indexOf(str)` — the index of the first occurrence of `needle` in `hay`
/// as a UTF-16 code-unit offset, or `-1` if absent (exactly the JDK semantics the
/// `>= 0` / `> -1` presence tests rely on).
fn java_index_of(hay: &str, needle: &str) -> i32 {
    let hay16: Vec<u16> = hay.encode_utf16().collect();
    let needle16: Vec<u16> = needle.encode_utf16().collect();
    if needle16.is_empty() {
        return 0;
    }
    if needle16.len() > hay16.len() {
        return -1;
    }
    for start in 0..=(hay16.len() - needle16.len()) {
        if hay16[start..start + needle16.len()] == needle16[..] {
            return start as i32;
        }
    }
    -1
}

/// `public static final void init(MIDlet midlet)` (`w.a:(MIDlet)V`). Captures the
/// MIDlet (here: the property seam is already on the state) and resolves the
/// full-version flag, mirroring it into [`Debug`](crate::debug).
pub fn init(s: &mut AppConfigState, debug: &mut DebugState) {
    // AppConfig.midlet = midlet;   (the app_properties seam is the captured MIDlet)
    // fullVersion = isFullVersion();
    s.full_version = is_full_version(s);
    // Debug.fullVersion = fullVersion;
    debug.full_version = s.full_version;
}

/// `private static boolean isFullVersion()` (`w.c:()Z`). True unless the `HO-Demo`
/// property holds the demo-unlock token.
pub fn is_full_version(s: &AppConfigState) -> bool {
    // String demoToken = midlet.getAppProperty("HO-Demo");
    let demo_token = s.get_app_property("HO-Demo");
    // boolean full = true;
    let mut full = true;
    // if (demoToken != null && demoToken.trim().equals("BEJ8K52N7A")) full = false;
    if let Some(token) = demo_token {
        if java_string_trim(&token) == DEMO_UNLOCK_TOKEN {
            full = false;
        }
    }
    full
}

/// `public static final void apply()` (`w.a:()V`). Applies buy-link setup (the
/// `AppConfig`-owned state: [`menu_buy_enabled`](AppConfigState::menu_buy_enabled),
/// [`exit_buy_enabled`](AppConfigState::exit_buy_enabled),
/// [`buy_url`](AppConfigState::buy_url)). `locale_index` is
/// `StringTable.instance.localeIndex`.
///
/// DEFERRED cross-class boundary (unported `FontManager` / `GameMIDlet`): the
/// `FontManager.mainMenuLabels[5]` push (via [`resolve_buy_label`]), and the
/// `MIDlet-Version` → `FontManager.versionText` string assembly. Neither carries
/// arithmetic (`apply`'s shape is `[]`), so no opcode-shape fidelity is lost.
pub fn apply(s: &mut AppConfigState, locale_index: i32) {
    // if (isBuySetupEnabled(true)) menuBuyEnabled = true;
    if is_buy_setup_enabled(s, locale_index, true) {
        s.menu_buy_enabled = true;
    }
    // if (isBuySetupEnabled(false)) exitBuyEnabled = true;
    if is_buy_setup_enabled(s, locale_index, false) {
        s.exit_buy_enabled = true;
    }
    // DEFERRED: FontManager.mainMenuLabels[5] = (menuBuyEnabled ? resolveBuyLabel()
    //           : FontManager.mainMenuLabels[6]).toCharArray();  (FontManager unported)
    // buyUrl = resolveBuyUrl();
    s.buy_url = resolve_buy_url(s, locale_index);
    // DEFERRED try { versionLine = GameMIDlet.instance.getAppProperty("MIDlet-Version");
    //   if (fullVersion) versionLine += " " + FontManager.getString(3917);
    //   FontManager.versionText = versionLine.toCharArray(); } catch {}
    //   (GameMIDlet / FontManager unported; no arithmetic).
}

/// `public static final boolean showFullVersionLink()` (`w.a:()Z`).
pub fn show_full_version_link(s: &AppConfigState) -> bool {
    // return fullVersion && menuBuyEnabled && buyUrl != null;
    s.full_version && s.menu_buy_enabled && s.buy_url.is_some()
}

/// `public static final boolean showDemoBuyLink()` (`w.b:()Z`).
// The `? false : true` ternary is the source's redundant idiom, preserved
// verbatim rather than folded to `!(...)` (contract: do not fold a redundant op).
#[allow(clippy::needless_bool)]
pub fn show_demo_buy_link(s: &AppConfigState) -> bool {
    // return (fullVersion || !menuBuyEnabled || buyUrl == null) ? false : true;
    if s.full_version || !s.menu_buy_enabled || s.buy_url.is_none() {
        false
    } else {
        true
    }
}

/// `public static final int initAvailableLocales()` (`w.a:()I`). Builds
/// [`locale_enabled`](AppConfigState::locale_enabled) from `HO-LangList`; returns
/// the single forced locale index when exactly one is listed, otherwise `-1` (and,
/// when none are listed, enables every locale).
pub fn init_available_locales(s: &mut AppConfigState) -> i32 {
    // int localeCount = locales.length;
    let locale_count: i32 = s.locales.len() as i32;
    // localeEnabled = new boolean[localeCount];
    s.locale_enabled = vec![false; locale_count as usize];
    // int matched = 0; int lastMatch = -1;
    let mut matched: i32 = 0;
    let mut last_match: i32 = -1;
    // String langList = midlet.getAppProperty("HO-LangList");
    let lang_list = s.get_app_property("HO-LangList");
    // if (langList != null)
    if let Some(lang_list) = lang_list {
        // for (int i = 0; i < localeCount; i++)
        let mut i: i32 = 0;
        while i < locale_count {
            // if (langList.indexOf(locales[i]) >= 0)
            if java_index_of(&lang_list, &s.locales[i as usize]) >= 0 {
                // System.out.println(locales[i]);   (debug trace → no-op)
                // localeEnabled[i] = true; lastMatch = i; matched++;
                s.locale_enabled[i as usize] = true;
                last_match = i;
                matched = matched.wrapping_add(1);
            }
            // i++
            i = i.wrapping_add(1);
        }
    }
    // if (matched == 1) return lastMatch;
    if matched == 1 {
        return last_match;
    }
    // if (matched != 0) return -1;
    if matched != 0 {
        return -1;
    }
    // for (int i = 0; i < localeCount; i++) localeEnabled[i] = true;
    let mut i: i32 = 0;
    while i < locale_count {
        s.locale_enabled[i as usize] = true;
        i = i.wrapping_add(1);
    }
    // return -1;
    -1
}

/// `public static final String resolveBuyLabel()` (`w.a:()Ljava/lang/String;`).
/// Resolves the localized buy-link label (`HO-Label-xx`), decoding any `\uXXXX`
/// escapes and capping the result at 16 characters. `locale_index` is
/// `StringTable.instance.localeIndex`; `string_table_full` / `string_table_demo`
/// are the DEFERRED `StringTable.instance.get(3931)` / `get(3925)` fallbacks
/// (unported `StringTable`) — supplied so the arithmetic-bearing unescape path is
/// transliterated faithfully.
pub fn resolve_buy_label(
    s: &AppConfigState,
    locale_index: i32,
    string_table_full: &str,
    string_table_demo: &str,
) -> String {
    // String label = midlet.getAppProperty("HO-Label-" + locales[localeIndex]);
    let key = format!("HO-Label-{}", s.locales[locale_index as usize]);
    let label = s.get_app_property(&key);
    // if (label == null || label.length() == 0)
    //   return fullVersion ? StringTable.get(3931) : StringTable.get(3925);
    let label = match label {
        None => {
            return if s.full_version {
                string_table_full.to_string()
            } else {
                string_table_demo.to_string()
            };
        }
        Some(l) if l.is_empty() => {
            return if s.full_version {
                string_table_full.to_string()
            } else {
                string_table_demo.to_string()
            };
        }
        Some(l) => l,
    };
    // if (label.indexOf("\\u") < 0) { int length = label.length();
    //     return label.substring(0, length < 16 ? length : 16); }
    if java_index_of(&label, "\\u") < 0 {
        let label16: Vec<u16> = label.encode_utf16().collect();
        let length: i32 = label16.len() as i32;
        let end = if length < 16 { length } else { 16 } as usize;
        return String::from_utf16(&label16[0..end]).expect("valid UTF-16 substring");
    }
    // StringBuffer buffer = new StringBuffer(label);
    let mut buffer: Vec<u16> = label.encode_utf16().collect();
    // int i = 0; char[] hex = new char[4];
    let mut i: i32 = 0;
    let mut hex: [u16; 4] = [0; 4];
    // do { ... } while (i < buffer.length());
    loop {
        // int at = i; i++;
        let at = i;
        i = i.wrapping_add(1);
        // if (buffer.charAt(at) == '\\' && buffer.charAt(i) == 'u')
        if buffer[at as usize] == u16::from(b'\\') && buffer[i as usize] == u16::from(b'u') {
            // buffer.getChars(i + 1, i + 5, hex, 0);
            let src_begin = i.wrapping_add(1) as usize;
            let src_end = i.wrapping_add(5) as usize;
            hex[0..4].copy_from_slice(&buffer[src_begin..src_end]);
            // buffer.setCharAt(i - 1, (char) Integer.parseInt(charsToString(hex), 16));
            let hex_str = String::from_utf16(&hex).expect("valid UTF-16 hex chars");
            let parsed = i32::from_str_radix(&hex_str, 16).expect("NumberFormatException");
            buffer[i.wrapping_sub(1) as usize] = parsed as u16; // (char) → i2c
                                                                // buffer.delete(i, i + 5);
            let del_end = i.wrapping_add(5) as usize;
            buffer.drain(i as usize..del_end);
        }
        // } while (i < buffer.length());
        if i >= buffer.len() as i32 {
            break;
        }
    }
    // String decoded = buffer.toString(); int length = decoded.length();
    // return decoded.substring(0, length < 16 ? length : 16);
    let length: i32 = buffer.len() as i32;
    let end = if length < 16 { length } else { 16 } as usize;
    String::from_utf16(&buffer[0..end]).expect("valid UTF-16 decoded")
}

/// `private static String resolveBuyUrl()` (`w.b:()Ljava/lang/String;`). Resolves
/// the per-locale buy URL (`HO-URL-xx`), or `None` if unset. `locale_index` is
/// `StringTable.instance.localeIndex`.
pub fn resolve_buy_url(s: &AppConfigState, locale_index: i32) -> Option<String> {
    // String url = midlet.getAppProperty("HO-URL-" + locales[localeIndex]);
    let key = format!("HO-URL-{}", s.locales[locale_index as usize]);
    let url = s.get_app_property(&key);
    // if (url == null || url.length() == 0) return null;
    match url {
        None => None,
        Some(u) if u.is_empty() => None,
        Some(u) => Some(u),
    }
}

/// `private static boolean isBuySetupEnabled(boolean menu)` (`w.a:(Z)Z`). True
/// when `HO-BuySetup` enables a buy link (containing "menu" when `menu`, else
/// "exit") and a locale buy URL is configured. `locale_index` is
/// `StringTable.instance.localeIndex`.
pub fn is_buy_setup_enabled(s: &AppConfigState, locale_index: i32, menu: bool) -> bool {
    // String buySetup = midlet.getAppProperty("HO-BuySetup");
    let buy_setup = s.get_app_property("HO-BuySetup");
    // String url = midlet.getAppProperty("HO-URL-" + locales[localeIndex]);
    let key = format!("HO-URL-{}", s.locales[locale_index as usize]);
    let url = s.get_app_property(&key);
    // if (buySetup == null || buySetup.length()==0 || url==null || url.length()==0) return false;
    let buy_setup = match &buy_setup {
        None => return false,
        Some(b) if b.is_empty() => return false,
        Some(b) => b,
    };
    match &url {
        None => return false,
        Some(u) if u.is_empty() => return false,
        Some(_) => {}
    }
    // if (menu) return buySetup.indexOf("menu") > -1;
    if menu {
        return java_index_of(buy_setup, "menu") > -1;
    }
    // return buySetup.indexOf("exit") > -1;
    java_index_of(buy_setup, "exit") > -1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(props: &[(&str, &str)], locales: &[&str]) -> AppConfigState {
        let mut s = AppConfigState {
            locales: locales.iter().map(|l| l.to_string()).collect(),
            ..AppConfigState::default()
        };
        for (k, v) in props {
            s.app_properties.insert(k.to_string(), v.to_string());
        }
        s
    }

    #[test]
    fn is_full_version_reads_the_demo_token() {
        // No HO-Demo → full.
        assert!(is_full_version(&state_with(&[], &["en"])));
        // HO-Demo == the unlock token (with surrounding whitespace, trim()ed) → demo.
        assert!(!is_full_version(&state_with(
            &[("HO-Demo", "  BEJ8K52N7A ")],
            &["en"]
        )));
        // A different token → still full.
        assert!(is_full_version(&state_with(
            &[("HO-Demo", "nope")],
            &["en"]
        )));
    }

    #[test]
    fn init_mirrors_full_version_into_debug() {
        let mut s = state_with(&[("HO-Demo", "BEJ8K52N7A")], &["en"]);
        let mut debug = DebugState::default();
        init(&mut s, &mut debug);
        assert!(!s.full_version);
        assert!(
            !debug.full_version,
            "Debug.fullVersion must mirror AppConfig"
        );

        let mut s = state_with(&[], &["en"]);
        init(&mut s, &mut debug);
        assert!(s.full_version && debug.full_version);
    }

    #[test]
    fn init_available_locales_forces_a_single_match() {
        // Exactly one locale in HO-LangList → its index is returned, only it enabled.
        let mut s = state_with(&[("HO-LangList", "de")], &["en", "de", "fr"]);
        assert_eq!(init_available_locales(&mut s), 1);
        assert_eq!(s.locale_enabled, vec![false, true, false]);

        // Several matches → -1, each match enabled.
        let mut s = state_with(&[("HO-LangList", "en,fr")], &["en", "de", "fr"]);
        assert_eq!(init_available_locales(&mut s), -1);
        assert_eq!(s.locale_enabled, vec![true, false, true]);

        // No HO-LangList (or no match) → -1, ALL enabled.
        let mut s = state_with(&[], &["en", "de", "fr"]);
        assert_eq!(init_available_locales(&mut s), -1);
        assert_eq!(s.locale_enabled, vec![true, true, true]);
    }

    #[test]
    fn buy_setup_and_url_gate_the_links() {
        // menu link needs HO-BuySetup containing "menu" AND a locale HO-URL.
        let s = state_with(
            &[
                ("HO-BuySetup", "menu,exit"),
                ("HO-URL-en", "http://buy.example/en"),
            ],
            &["en"],
        );
        assert!(is_buy_setup_enabled(&s, 0, true));
        assert!(is_buy_setup_enabled(&s, 0, false));
        assert_eq!(
            resolve_buy_url(&s, 0),
            Some("http://buy.example/en".to_string())
        );

        // No URL → both gates closed, URL is None.
        let s = state_with(&[("HO-BuySetup", "menu")], &["en"]);
        assert!(!is_buy_setup_enabled(&s, 0, true));
        assert_eq!(resolve_buy_url(&s, 0), None);
    }

    #[test]
    fn apply_sets_buy_flags_and_url() {
        let mut s = state_with(
            &[("HO-BuySetup", "menu"), ("HO-URL-en", "http://x")],
            &["en"],
        );
        apply(&mut s, 0);
        assert!(s.menu_buy_enabled);
        assert!(!s.exit_buy_enabled); // "exit" not in HO-BuySetup
        assert_eq!(s.buy_url, Some("http://x".to_string()));
    }

    #[test]
    fn show_link_predicates() {
        let mut s = state_with(&[], &["en"]);
        s.full_version = true;
        s.menu_buy_enabled = true;
        s.buy_url = Some("u".to_string());
        assert!(show_full_version_link(&s));
        assert!(!show_demo_buy_link(&s)); // full version → no demo link

        s.full_version = false;
        assert!(!show_full_version_link(&s));
        assert!(show_demo_buy_link(&s)); // demo + menu link + url → demo link shows
    }

    #[test]
    fn resolve_buy_label_plain_and_capped() {
        // Plain label (no \u), under 16 chars → returned as-is.
        let s = state_with(&[("HO-Label-en", "Buy Now")], &["en"]);
        assert_eq!(resolve_buy_label(&s, 0, "FULL", "DEMO"), "Buy Now");

        // Over 16 chars → capped at 16.
        let s = state_with(&[("HO-Label-en", "ABCDEFGHIJKLMNOPQRSTUVWXYZ")], &["en"]);
        assert_eq!(resolve_buy_label(&s, 0, "FULL", "DEMO"), "ABCDEFGHIJKLMNOP");

        // Missing/empty → the StringTable fallback (by full/demo flag).
        let s = state_with(&[], &["en"]);
        assert_eq!(resolve_buy_label(&s, 0, "FULL", "DEMO"), "DEMO");
        let mut s = state_with(&[("HO-Label-en", "")], &["en"]);
        s.full_version = true;
        assert_eq!(resolve_buy_label(&s, 0, "FULL", "DEMO"), "FULL");
    }

    #[test]
    fn resolve_buy_label_decodes_unicode_escapes() {
        // "A" → 'A'. The unescape loop rewrites the backslash cell then deletes
        // the following 5 chars ('u' + 4 hex).
        let s = state_with(&[("HO-Label-en", "\\u0041BC")], &["en"]);
        assert_eq!(resolve_buy_label(&s, 0, "FULL", "DEMO"), "ABC");

        // Two escapes in a row.
        let s = state_with(&[("HO-Label-en", "\\u0048\\u0069!")], &["en"]);
        assert_eq!(resolve_buy_label(&s, 0, "FULL", "DEMO"), "Hi!");
    }
}
