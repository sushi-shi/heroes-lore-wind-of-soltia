//! Transliterated from `java/src/main/java/defpackage/Menu.java`
//! (original `cb.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The abstract base of every in-game menu/panel/dialog: a stack of nested screens
//! (`parent`/`child`) with a linear cursor (`cursorIndex`) over `itemCount`
//! entries, a lazy `needsRepaint` dirty flag, and a static **draw kit**.
//!
//! ## ANTI-BOG boundary
//!
//! This increment ports **only** what the fresh-install main-menu path exercises:
//! the instance fields (as [`MenuBase`], carried per concrete menu), the cursor
//! navigation (`moveCursorVertical` + `stepCursor`), `passKeyToChild`,
//! `invalidateDown`, and the lazy [`render`] (specialised to the single reachable
//! concrete subclass `MainMenu`, since `child` is always `null` here — popups are
//! deferred). The large static **draw kit** (`drawBevelBox`/`drawButton`/
//! `drawSelectableBox`/`drawTabButton`/`drawTextField`/`drawPanelFrame`/…,
//! `drawListPage`, the pagination helpers, `showPopup`/`onPopupResult`/`close`) is
//! **NOT reached by `MainMenu.paint`** — which draws through `MainMenu.drawMenuPanel`
//! (its own static) + `FontManager.drawMenuItem`/`drawSoftKeys` — and is DEFERRED.
//!
//! Menu has **no `static` fields** (every field is per-instance), so it contributes
//! no `java/reconstruction/ownership.tsv` rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `cb.a:(IIZ)Z => []`
//! (moveCursorVertical — pure branches), `cb.a:(BZ)V =>
//! [iadd,i2b,isub,i2b,isub,i2b,isub,i2b]` (stepCursor), `cb.b:(II)Z => []`
//! (passKeyToChild), `cb.b:(…Graphics;II)V => []` (render — no arithmetic),
//! `cb.c:()V => []` (invalidateDown).

use crate::game::Game;
use crate::main_menu;

/// The instance fields of a `Menu` (`cb`), carried by each concrete menu's state
/// (here [`crate::main_menu::MainMenuState`]). Reference fields (`parent`/`child`)
/// are presence flags (`false` while null); on the fresh main-menu path `parent`
/// and `child` are always null.
#[derive(Debug, Default, Clone)]
pub struct MenuBase {
    /// `public Menu parent;` — enclosing menu (`null` at the root → false).
    pub parent: bool,
    /// `public byte itemCount;` — number of selectable entries.
    pub item_count: i8,
    /// `public Menu child = null;` — pushed sub-screen (`null` here → false).
    pub child: bool,
    /// `private boolean pendingInitialPaint = true;`
    pub pending_initial_paint: bool,
    /// `public boolean needsRepaint = true;`
    pub needs_repaint: bool,
    /// `public byte cursorIndex = 0;`
    pub cursor_index: i8,
}

/// `public Menu(Menu parentMenu, byte itemCount)` (`cb.<init>:(Lcb;B)V => []`):
/// `parent = parentMenu; itemCount = itemCount;` over the field initializers
/// (`pendingInitialPaint = true`, `needsRepaint = true`, `cursorIndex = 0`,
/// `child = null`).
pub fn construct(item_count: i8) -> MenuBase {
    MenuBase {
        // parent = parentMenu;   — the main menu is the root (null → false).
        parent: false,
        // itemCount = itemCount;
        item_count,
        // child = null;   (field initializer)
        child: false,
        // pendingInitialPaint = true;
        pending_initial_paint: true,
        // needsRepaint = true;
        needs_repaint: true,
        // cursorIndex = 0;
        cursor_index: 0,
    }
}

/// `public final boolean passKeyToChild(int action, int keyCode)`
/// (`cb.b:(II)Z => []`): forwards a key to the pushed child; here `child` is always
/// null (popups deferred), so it only marks the screen dirty and returns false.
pub fn pass_key_to_child(base: &mut MenuBase, _action: i32, _key_code: i32) -> bool {
    // if (this.child != null && this.child.handleKey(action, keyCode)) return true;
    //   — child null on the main-menu path (popup dispatch DEFERRED).
    if base.child {
        // (DEFERRED: child.handleKey(action, keyCode))
    }
    // this.needsRepaint = true; return false;
    base.needs_repaint = true;
    false
}

/// `public final boolean moveCursorVertical(int action, int keyCode, boolean wrap)`
/// (`cb.a:(IIZ)Z => []`): moves the cursor up/down (keys 2/8 or UP/DOWN actions).
pub fn move_cursor_vertical(base: &mut MenuBase, action: i32, key_code: i32, wrap: bool) -> bool {
    match key_code {
        // case 50: stepCursor((byte) 3, wrap); return true;
        50 => {
            step_cursor(base, 3, wrap);
            true
        }
        // case 56: stepCursor((byte) 4, wrap); return true;
        56 => {
            step_cursor(base, 4, wrap);
            true
        }
        _ => match action {
            // case 1: stepCursor((byte) 3, wrap); return true;
            1 => {
                step_cursor(base, 3, wrap);
                true
            }
            // case 6: stepCursor((byte) 4, wrap); return true;
            6 => {
                step_cursor(base, 4, wrap);
                true
            }
            // default: return false;
            _ => false,
        },
    }
}

/// `public final void stepCursor(byte direction, boolean wrap)`
/// (`cb.a:(BZ)V => [iadd,i2b,isub,i2b,isub,i2b,isub,i2b]`): steps the cursor one
/// entry (`direction` 4 advances, otherwise retreats), with optional wrap.
pub fn step_cursor(base: &mut MenuBase, direction: i8, wrap: bool) {
    // if (direction != 4) { ... retreat ... return; }
    if direction != 4 {
        // this.cursorIndex = (byte) (this.cursorIndex - 1);
        base.cursor_index = (base.cursor_index as i32).wrapping_sub(1) as i8;
        // if (this.cursorIndex < 0) { if (wrap) cursorIndex = itemCount-1; else cursorIndex = 0; return; }
        if (base.cursor_index as i32) < 0 {
            if wrap {
                base.cursor_index = (base.item_count as i32).wrapping_sub(1) as i8;
            } else {
                base.cursor_index = 0;
            }
        }
        return;
    }
    // this.cursorIndex = (byte) (this.cursorIndex + 1);
    base.cursor_index = (base.cursor_index as i32).wrapping_add(1) as i8;
    // if (this.cursorIndex >= this.itemCount) { ... }
    if (base.cursor_index as i32) >= (base.item_count as i32) {
        if wrap {
            // this.cursorIndex = (byte) 0; return;
            base.cursor_index = 0;
            return;
        }
        // this.cursorIndex = (byte) (this.itemCount - 1);
        base.cursor_index = (base.item_count as i32).wrapping_sub(1) as i8;
        // if (this.cursorIndex < 0) this.cursorIndex = (byte) 0;
        if (base.cursor_index as i32) < 0 {
            base.cursor_index = 0;
        }
    }
}

/// `public final void invalidateDown()` (`cb.c:()V => []`): marks this screen and
/// every pushed descendant as needing a repaint. `child` is null on the main-menu
/// path, so only this screen is marked.
pub fn invalidate_down(base: &mut MenuBase) {
    // if (this.child != null) this.child.invalidateDown();
    if base.child {
        // (DEFERRED: child.invalidateDown())
    }
    // this.needsRepaint = true;
    base.needs_repaint = true;
}

/// `public final void render(Graphics graphics, int originX, int originY)`
/// (`cb.b:(…Graphics;II)V => []`): lazily repaints when dirty, then recurses into
/// the pushed child. Specialised to the single reachable concrete subclass
/// (`MainMenu`): the abstract `paint` dispatches to [`main_menu::paint`], and
/// `child` is always null here (its `render` recursion — popups — is DEFERRED).
pub fn render(g: &mut Game, origin_x: i32, origin_y: i32) {
    // boolean painted = false;
    let mut painted = false;
    // if (this.needsRepaint) { needsRepaint=false; paint(...); painted=true; }
    if g.main_menu.base.needs_repaint {
        g.main_menu.base.needs_repaint = false;
        main_menu::paint(g, origin_x, origin_y);
        painted = true;
    }
    // if (this.child != null) child.render(...); else if (pendingInitialPaint) { if (!painted) paint(...); pendingInitialPaint=false; }
    if g.main_menu.base.child {
        // (DEFERRED: child.render(graphics, originX, originY))
    } else if g.main_menu.base.pending_initial_paint {
        if !painted {
            main_menu::paint(g, origin_x, origin_y);
        }
        g.main_menu.base.pending_initial_paint = false;
    }
}
