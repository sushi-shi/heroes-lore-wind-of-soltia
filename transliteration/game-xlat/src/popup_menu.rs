//! Transliterated from `java/src/main/java/defpackage/PopupMenu.java`
//! (original `af.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! General-purpose popup / dialog (`PopupMenu extends Menu`) spawned by
//! [`Menu::show_popup`](crate::menu::show_popup) / [`show_message`](crate::menu::show_message).
//! Its [`popup_type`](PopupMenuState::popup_type) selects the layout: `1`/`11` OK
//! message, `2`/`6`/`12` yes-no confirm, `3`/`4`/`5`/`8` selectable list (8 keeps
//! row 0 as a header), `9` a bare message. Answers are reported back to the parent
//! via `parent.onPopupResult(type, result)` — `result` is the chosen row, `0` for
//! OK/Yes, `99` for cancel/Back (see [`crate::menu::on_popup_result`]).
//!
//! ## ANTI-BOG boundary
//!
//! This increment ports the constructor (field stores + the `joinedText` build +
//! the label defaulting), `handleKey` (fully — the popup->parent
//! `onPopupResult` callback is made **real** via the flat model's
//! [`parent_of`](crate::menu::parent_of) scan), and a **DEFERRED** `paint`. The
//! `paint` is fully deferred: its very first op (`FontManager.clearScreen`) and the
//! box geometry (`FontManager.percentOf(BaseCanvas.width, 80)`) cross into unported
//! `FontManager` statics, and the body draws `FontManager.drawWrappedText` /
//! `drawSoftKeys` / `AssetCache.cursorArrow` — all unported. The `boxHeight`
//! pre-measurement (`FontManager.measureBlockHeight` / `percentOf`) is likewise
//! DEFERRED, and the `positiveLabel`/`negativeLabel` defaults that resolve to
//! `FontManager.labelYes`/`labelNo`/`labelBack` (unported) are DEFERRED — only the
//! `FontManager.labelOk` default (ported) is filled.
//!
//! `PopupMenu`'s fields are all per-INSTANCE (the `Menu` base fields likewise), so
//! it contributes no `java/reconstruction/ownership.tsv` static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `af.<init>:(Lcb;BB[Ljava/lang/Object;[C[C)V =>
//! [isub,iinc,isub,iadd,isub,iadd,isub,iadd,iadd,iinc,isub,iadd]` (constructor —
//! the ported subset builds `joinedText` (`i != options.length - 1` → the `isub`)
//! and stores; the remaining `iadd/isub`s live in the DEFERRED `boxHeight`
//! measurement), `af.a:(II)Z => []` (handleKey — pure branches), and
//! `af.a:(…Graphics;II)V => [ishr,isub,ishr,…]` (paint — fully DEFERRED).

use crate::game::Game;
use crate::menu::{self, MenuNode};

/// Java `af` / `PopupMenu` instance state — the `Menu` (`cb`) base fields plus the
/// popup's own instance fields.
#[derive(Debug, Default, Clone)]
pub struct PopupMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private byte type;` (obf `c`) — layout/behaviour selector (`type` is a Rust
    /// keyword, so it is `popup_type` here).
    pub popup_type: i8,
    /// `private Object[] options;` (obf `a`) — the option rows (each a `char[]`).
    pub options: Vec<Vec<u16>>,
    /// `private char[] joinedText;` (obf `a`) — `options` joined by newlines.
    pub joined_text: Vec<u16>,
    /// `private int boxHeight;` (obf `a`) — pre-measured panel height (DEFERRED → 0).
    pub box_height: i32,
    /// `private char[] positiveLabel;` (obf `b`) — left/confirm softkey label.
    pub positive_label: Option<Vec<u16>>,
    /// `private char[] negativeLabel;` (obf `c`) — right/cancel softkey label.
    pub negative_label: Option<Vec<u16>>,
}

/// `public PopupMenu(Menu parent, byte type, byte itemCount, Object[] options, char[] positiveLabel, char[] negativeLabel)`
/// (`af.<init>:(Lcb;BB[Ljava/lang/Object;[C[C)V`). Builds `joinedText`, stores the
/// options, defaults the softkey labels per `type`, pre-measures `boxHeight`
/// (DEFERRED), and — for `type == 6` — starts the cursor on row 1.
pub fn construct(
    g: &mut Game,
    popup_type: i8,
    item_count: i8,
    options: Vec<Vec<u16>>,
    mut positive_label: Option<Vec<u16>>,
    negative_label: Option<Vec<u16>>,
) {
    // super(parent, itemCount);   (parent is the calling menu → non-null → present)
    g.popup_menu.base = menu::construct(true, item_count);
    // this.type = type;
    g.popup_menu.popup_type = popup_type;
    // StringBuffer joined = new StringBuffer();
    // for (int i = 0; i < options.length; i++) { char[] option = options[i];
    //   if (option.length > 0) { joined.append(option); if (i != options.length - 1) joined.append('\n'); } }
    let mut joined: Vec<u16> = Vec::new();
    let n = options.len();
    for (i, option) in options.iter().enumerate() {
        if !option.is_empty() {
            joined.extend_from_slice(option);
            if i != n - 1 {
                joined.push(b'\n' as u16);
            }
        }
    }
    // this.joinedText = joined.toString().toCharArray();
    g.popup_menu.joined_text = joined;
    // this.options = options;
    g.popup_menu.options = options;
    // --- label defaulting per type ---
    // (Only `positive_label` is reassigned here — the `negative_label` defaults resolve
    //  to `FontManager.labelNo`/`labelBack`, both unported → DEFERRED, so it stays as passed.)
    if popup_type == 2 || popup_type == 12 {
        // positiveLabel = positiveLabel == null ? FontManager.labelYes : positiveLabel;
        if positive_label.is_none() {
            // (DEFERRED: FontManager.labelYes unported — the null default is left None.)
        }
        // if (negativeLabel == null) negativeLabel = FontManager.labelNo;
        if negative_label.is_none() {
            // (DEFERRED: FontManager.labelNo unported.)
        }
    } else if popup_type == 1 || popup_type == 11 {
        // if (positiveLabel == null) positiveLabel = FontManager.labelOk;
        if positive_label.is_none() {
            positive_label = g.font_manager.label_ok.clone();
        }
    } else if popup_type != 9 {
        // positiveLabel = positiveLabel == null ? FontManager.labelOk : positiveLabel;
        if positive_label.is_none() {
            positive_label = g.font_manager.label_ok.clone();
        }
        // if (negativeLabel == null) negativeLabel = FontManager.labelBack;
        if negative_label.is_none() {
            // (DEFERRED: FontManager.labelBack unported.)
        }
    }
    // this.positiveLabel = positiveLabel; this.negativeLabel = negativeLabel;
    g.popup_menu.positive_label = positive_label;
    g.popup_menu.negative_label = negative_label;
    // switch (type) { ... this.boxHeight = ... measureBlockHeight(percentOf(width,80)-10, ...) ... }
    // (DEFERRED: boxHeight measurement — FontManager.measureBlockHeight / percentOf /
    //  BaseCanvas.width percent are unported; boxHeight stays 0.)
    // if (type == 6) ((Menu) this).cursorIndex = (byte) 1;
    if popup_type == 6 {
        g.popup_menu.base.cursor_index = 1;
    }
}

/// `public final boolean handleKey(int action, int keyCode)` (`af.a:(II)Z => []`):
/// forwards to any child, then dispatches by `type`. OK/Yes/list-select report
/// `parent.onPopupResult(type, result)` (`result` = `0` for OK/Yes, the row for a
/// list, `99` for Back). Always returns true.
// The `case 3/4/5/8` arm keeps the Java `if (!moveCursorVerticalNoWrap(...)) { … }`
// structure verbatim (the guard has the cursor-move side effect), so the
// `collapsible_match` suggestion to fold it into a match guard is declined.
#[allow(clippy::collapsible_match)]
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Popup, action, key_code) {
        return true;
    }
    // switch (this.type) {
    match g.popup_menu.popup_type as i32 {
        // case 1: case 11:
        1 | 11 => {
            // if (keyCode == 53 || action == 8) parent.onPopupResult(type, (byte) 0);
            if key_code == 53 || action == 8 {
                popup_result(g, 0);
            }
        }
        // case 2: case 6: case 12:
        2 | 6 | 12 => {
            // if (keyCode == 53) parent.onPopupResult(type, 0); else if (keyCode == -8) parent.onPopupResult(type, 99);
            if key_code == 53 {
                popup_result(g, 0);
            } else if key_code == -8 {
                popup_result(g, 99);
            }
        }
        // case 3: case 4: case 5: case 8:
        3 | 4 | 5 | 8 => {
            // if (!moveCursorVerticalNoWrap(action, keyCode)) {
            if !menu::move_cursor_vertical(&mut g.popup_menu.base, action, key_code, false) {
                // if (keyCode == 53 || action == 8) parent.onPopupResult(type, cursorIndex);
                if key_code == 53 || action == 8 {
                    let cursor = g.popup_menu.base.cursor_index;
                    popup_result(g, cursor);
                // else if (keyCode == -8) parent.onPopupResult(type, 99);
                } else if key_code == -8 {
                    popup_result(g, 99);
                }
            }
        }
        _ => {}
    }
    // return true;
    true
}

/// `((Menu) this).parent.onPopupResult(this.type, result)` — the flat model
/// resolves the parent via [`menu::parent_of`] (the popup is the current child of
/// exactly one menu) and dispatches the virtual `onPopupResult`.
fn popup_result(g: &mut Game, result: i8) {
    let tag = g.popup_menu.popup_type;
    // ((Menu) this).parent  — the menu whose child is this popup.
    let parent =
        menu::parent_of(g, MenuNode::Popup).expect("NullPointerException: PopupMenu.parent");
    menu::on_popup_result(g, parent, tag, result);
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`af.a:(…Graphics;II)V`): **DEFERRED**. The full body crosses into unported
/// `FontManager` statics from its first statement onward (`clearScreen`,
/// `percentOf`, `drawWrappedText`, `drawSoftKeys`, `AssetCache.cursorArrow`), and
/// the panel geometry depends on the DEFERRED `percentOf`, so nothing portable
/// remains to draw. Faithful full form (per `type`):
///   FontManager.clearScreen(graphics);
///   int boxWidth = FontManager.percentOf(BaseCanvas.width, 80);
///   int boxX = BaseCanvas.halfW - (boxWidth >> 1);
///   int boxY = BaseCanvas.halfH - (this.boxHeight >> 1);
///   Menu.drawPanelFrame(graphics, boxX, boxY, boxWidth, this.boxHeight);
///   Menu.fillPanelInterior(graphics, boxX, boxY, boxWidth, this.boxHeight);
///   switch (type) { … drawWrappedText(joinedText) / list rows with AssetCache.cursorArrow … }
///   FontManager.drawSoftKeys(graphics, this.positiveLabel, this.negativeLabel);
pub fn paint(_g: &mut Game, _x: i32, _y: i32) {
    // (DEFERRED — see the doc comment. FontManager.clearScreen / percentOf /
    //  drawWrappedText / drawSoftKeys and AssetCache.cursorArrow are unported.)
}
