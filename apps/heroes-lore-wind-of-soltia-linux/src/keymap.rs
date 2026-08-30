//! Map a `winit` physical key to the raw Nokia/MIDP code the game's
//! `keyPressed(int)` expects — the same codes `HeadlessCapture.java` and the
//! [`capture`](crate::capture) route driver use (UP..RIGHT = -1..-4, FIRE = -5,
//! SOFT1/2 = -6/-7, digits 48–57, `*` = 42, `#` = 35).
//!
//! | keyboard                              | Nokia key        | code   |
//! |---------------------------------------|------------------|--------|
//! | Arrow Up / Down / Left / Right        | D-pad            | -1..-4 |
//! | Enter, Space, Numpad Enter, Numpad 5  | Fire / select    | -5     |
//! | F1                                    | left soft key    | -6     |
//! | F2                                    | right soft key   | -7     |
//! | 0–9 (top row or numpad)               | number keys      | 48–57  |
//! | Numpad `*` / `[`                      | star `*`         | 42     |
//! | `]` / `\`                             | pound `#`        | 35     |
//!
//! (Escape is handled by the window shell as "quit", not routed to the game.)
//!
//! ## Where these codes come from
//! The numeric-keypad codes are the GENERIC MIDP constants defined once in the
//! device runtime [`j2me_me::canvas`] (`KEY_NUM0..KEY_NUM9`, `KEY_STAR`,
//! `KEY_POUND`) — the very values the ported menu code branches on (`case 52/54/56`)
//! and `Canvas::common_game_action` decodes. They are sourced from there, not
//! re-hardcoded, so the phone-keypad contract has a single home.
//!
//! The negative D-pad / FIRE / soft-key codes are the MIDP Nokia device codes; the
//! generic `j2me-me` surface does not (yet) give them names, so they live here as
//! documented [`nokia`] constants. They belong in the generic device runtime
//! alongside `KEY_NUM*` (and this whole winit→J2ME adapter belongs in a generic
//! `j2me-platform-native` host crate so every game inherits it) — see the port's
//! integration notes.

use j2me_me::canvas::{
    KEY_NUM0, KEY_NUM1, KEY_NUM2, KEY_NUM3, KEY_NUM4, KEY_NUM5, KEY_NUM6, KEY_NUM7, KEY_NUM8,
    KEY_NUM9, KEY_POUND, KEY_STAR,
};
use winit::keyboard::KeyCode;

/// The MIDP Nokia device key codes (the negative half of the keypad) that the
/// generic [`j2me_me::canvas`] surface does not yet name. Kept together and
/// documented so the mapping below carries no bare magic numbers; these are the
/// exact codes `Canvas::common_game_action` decodes (`-1..-5`) and the shared
/// headless routes / `capture` driver inject. Candidates to lift into `j2me-me`.
mod nokia {
    /// `Canvas.KEY_UP` — D-pad up.
    pub const KEY_UP: i32 = -1;
    /// `Canvas.KEY_DOWN` — D-pad down.
    pub const KEY_DOWN: i32 = -2;
    /// `Canvas.KEY_LEFT` — D-pad left.
    pub const KEY_LEFT: i32 = -3;
    /// `Canvas.KEY_RIGHT` — D-pad right.
    pub const KEY_RIGHT: i32 = -4;
    /// `Canvas.KEY_FIRE` — centre select.
    pub const KEY_FIRE: i32 = -5;
    /// Left soft key.
    pub const KEY_SOFT1: i32 = -6;
    /// Right soft key.
    pub const KEY_SOFT2: i32 = -7;
}

/// The raw Nokia code for a physical key, or `None` if the game has no use for it.
pub fn nokia_code(key: KeyCode) -> Option<i32> {
    Some(match key {
        KeyCode::ArrowUp => nokia::KEY_UP,
        KeyCode::ArrowDown => nokia::KEY_DOWN,
        KeyCode::ArrowLeft => nokia::KEY_LEFT,
        KeyCode::ArrowRight => nokia::KEY_RIGHT,

        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::Numpad5 => {
            nokia::KEY_FIRE
        }

        KeyCode::F1 => nokia::KEY_SOFT1,
        KeyCode::F2 => nokia::KEY_SOFT2,

        KeyCode::Digit0 | KeyCode::Numpad0 => KEY_NUM0,
        KeyCode::Digit1 | KeyCode::Numpad1 => KEY_NUM1,
        KeyCode::Digit2 | KeyCode::Numpad2 => KEY_NUM2,
        KeyCode::Digit3 | KeyCode::Numpad3 => KEY_NUM3,
        KeyCode::Digit4 | KeyCode::Numpad4 => KEY_NUM4,
        // Numpad5 is Fire above; the top-row 5 remains a digit.
        KeyCode::Digit5 => KEY_NUM5,
        KeyCode::Digit6 | KeyCode::Numpad6 => KEY_NUM6,
        KeyCode::Digit7 | KeyCode::Numpad7 => KEY_NUM7,
        KeyCode::Digit8 | KeyCode::Numpad8 => KEY_NUM8,
        KeyCode::Digit9 | KeyCode::Numpad9 => KEY_NUM9,

        KeyCode::NumpadMultiply | KeyCode::BracketLeft => KEY_STAR,
        KeyCode::BracketRight | KeyCode::Backslash => KEY_POUND,

        _ => return None,
    })
}
