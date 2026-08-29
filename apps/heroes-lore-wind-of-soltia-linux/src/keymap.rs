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

use winit::keyboard::KeyCode;

/// The raw Nokia code for a physical key, or `None` if the game has no use for it.
pub fn nokia_code(key: KeyCode) -> Option<i32> {
    Some(match key {
        KeyCode::ArrowUp => -1,
        KeyCode::ArrowDown => -2,
        KeyCode::ArrowLeft => -3,
        KeyCode::ArrowRight => -4,

        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::Numpad5 => -5,

        KeyCode::F1 => -6,
        KeyCode::F2 => -7,

        KeyCode::Digit0 | KeyCode::Numpad0 => b'0' as i32,
        KeyCode::Digit1 | KeyCode::Numpad1 => b'1' as i32,
        KeyCode::Digit2 | KeyCode::Numpad2 => b'2' as i32,
        KeyCode::Digit3 | KeyCode::Numpad3 => b'3' as i32,
        KeyCode::Digit4 | KeyCode::Numpad4 => b'4' as i32,
        // Numpad5 is Fire above; the top-row 5 remains a digit.
        KeyCode::Digit5 => b'5' as i32,
        KeyCode::Digit6 | KeyCode::Numpad6 => b'6' as i32,
        KeyCode::Digit7 | KeyCode::Numpad7 => b'7' as i32,
        KeyCode::Digit8 | KeyCode::Numpad8 => b'8' as i32,
        KeyCode::Digit9 | KeyCode::Numpad9 => b'9' as i32,

        KeyCode::NumpadMultiply | KeyCode::BracketLeft => b'*' as i32,
        KeyCode::BracketRight | KeyCode::Backslash => b'#' as i32,

        _ => return None,
    })
}
