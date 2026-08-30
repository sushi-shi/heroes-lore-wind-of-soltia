//! Transliterated from `java/src/main/java/defpackage/Menu.java`
//! (original `cb.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The abstract base of every in-game menu/panel/dialog: a stack of nested screens
//! (`parent`/`child`) with a linear cursor (`cursorIndex`) over `itemCount`
//! entries, a lazy `needsRepaint` dirty flag, and a static **draw kit**.
//!
//! ## The child stack (flat-Rust model of `Menu.child`)
//!
//! In Java, `Menu.child` is a polymorphic `Menu` reference and `render` /
//! `passKeyToChild` / `invalidateDown` dispatch down the stack through virtual
//! calls. The flat transliteration has no dynamic dispatch, so — following the
//! [`CurrentScreen`](crate::game::CurrentScreen) precedent — the child link is a
//! discriminant [`MenuChild`] stored on [`MenuBase::child`], and a companion
//! [`MenuNode`] names each *concrete* menu that carries its own `*State`
//! (`MainMenu`, `ClassSelectMenu`, …). The recursive walkers dispatch on those:
//! [`node_base`]/[`node_base_mut`] select the current menu's `MenuBase`,
//! [`paint_node`]/[`dispatch_handle_key`] its concrete `paint`/`handleKey`, and
//! [`child_node`] resolves a child discriminant to the node to recurse into. With
//! no child (`MenuChild::None`) the walk behaves exactly as the old main-menu-only
//! specialisation did (the menu oracle stays pixel-exact).
//!
//! ## ANTI-BOG boundary
//!
//! This increment ports the fresh-install main-menu path plus the first child
//! transition (New Game → `ClassSelectMenu`): the instance fields (as [`MenuBase`],
//! carried per concrete menu), the cursor navigation (`moveCursorVertical` +
//! `moveCursorHorizontal` + `moveCursor` + `stepCursor`), `passKeyToChild`,
//! `invalidateDown`, and the lazy [`render`]. The large static **draw kit**
//! (`drawBevelBox`/`drawButton`/`drawSelectableBox`/`drawTabButton`/
//! `drawTextField`/`drawPanelFrame`/…, `drawListPage`, the pagination helpers,
//! `showPopup`/`onPopupResult`/`close`) is not reached by the ported menus and is
//! **DEFERRED**.
//!
//! Menu has **no `static` fields** (every field is per-instance), so it contributes
//! no `java/reconstruction/ownership.tsv` rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `cb.a:(IIZ)Z => []`
//! (moveCursorVertical — pure branches), `cb.d:(II)Z => []` (moveCursorHorizontal),
//! `cb.a:(B)V => []` (moveCursor), `cb.a:(BZ)V =>
//! [iadd,i2b,isub,i2b,isub,i2b,isub,i2b]` (stepCursor), `cb.b:(II)Z => []`
//! (passKeyToChild), `cb.b:(…Graphics;II)V => []` (render — no arithmetic),
//! `cb.c:()V => []` (invalidateDown).

use crate::class_confirm_menu;
use crate::class_select_menu;
use crate::game::Game;
use crate::main_menu;
use crate::start_trait_menu;

/// The pushed sub-screen of a menu — the flat model of the polymorphic
/// `Menu.child` reference (`null` → [`MenuChild::None`]). Each non-`None` variant
/// names the concrete child menu type. Variants whose menu is not yet ported are
/// DEFERRED: the discriminant is set faithfully where Java does `child = new …`,
/// but resolving/rendering it (`child_node`) is a `DEFERRED` boundary until that
/// menu lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuChild {
    /// `child == null`.
    #[default]
    None,
    /// `child instanceof ClassSelectMenu` (`c`) — the starting-class picker.
    ClassSelect,
    /// `child instanceof ClassConfirmMenu` (`by`) — the class Yes/No confirm.
    ClassConfirm,
    /// `child instanceof StartTraitMenu` (`bk`) — the starting-guardian picker.
    StartTrait,
    /// `child instanceof PopupMenu` (DEFERRED — `showMessage`/`showPopup`).
    Popup,
}

/// Identifies a *concrete* menu that owns a `MenuBase` + `paint`/`handleKey` — the
/// node the recursive walkers ([`render`], [`pass_key_to_child`],
/// [`invalidate_down`]) currently operate on. The root is always
/// [`MenuNode::Main`]; children are resolved from a [`MenuChild`] by [`child_node`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuNode {
    /// `MainMenu` (`bf`) — the always-root front menu.
    Main,
    /// `ClassSelectMenu` (`c`) — pushed as `MainMenu`'s child by New Game.
    ClassSelect,
    /// `ClassConfirmMenu` (`by`) — pushed as `ClassSelectMenu`'s child by FIRE.
    ClassConfirm,
    /// `StartTraitMenu` (`bk`) — pushed as `ClassConfirmMenu`'s child by "Yes".
    StartTrait,
}

/// The instance fields of a `Menu` (`cb`), carried by each concrete menu's state
/// (e.g. [`crate::main_menu::MainMenuState`], [`crate::class_select_menu::ClassSelectMenuState`]).
/// The `parent` reference field is a presence flag (`false` while null); `child`
/// is the [`MenuChild`] discriminant.
#[derive(Debug, Default, Clone)]
pub struct MenuBase {
    /// `public Menu parent;` — enclosing menu (`null` at the root → false).
    pub parent: bool,
    /// `public byte itemCount;` — number of selectable entries.
    pub item_count: i8,
    /// `public Menu child = null;` — pushed sub-screen (`null` → [`MenuChild::None`]).
    pub child: MenuChild,
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
/// `child = null`). `parent` is a presence flag (`true` when `parentMenu != null`).
pub fn construct(parent: bool, item_count: i8) -> MenuBase {
    MenuBase {
        // parent = parentMenu;   (presence flag)
        parent,
        // itemCount = itemCount;
        item_count,
        // child = null;   (field initializer)
        child: MenuChild::None,
        // pendingInitialPaint = true;
        pending_initial_paint: true,
        // needsRepaint = true;
        needs_repaint: true,
        // cursorIndex = 0;
        cursor_index: 0,
    }
}

// --------------------------------------------------------------------------
// Child-stack dispatch (the flat model of virtual `Menu` recursion)
// --------------------------------------------------------------------------

/// The [`MenuBase`] of a concrete menu node (immutable borrow).
fn node_base(g: &Game, node: MenuNode) -> &MenuBase {
    match node {
        MenuNode::Main => &g.main_menu.base,
        MenuNode::ClassSelect => &g.class_select_menu.base,
        MenuNode::ClassConfirm => &g.class_confirm_menu.base,
        MenuNode::StartTrait => &g.start_trait_menu.base,
    }
}

/// The [`MenuBase`] of a concrete menu node (mutable borrow).
fn node_base_mut(g: &mut Game, node: MenuNode) -> &mut MenuBase {
    match node {
        MenuNode::Main => &mut g.main_menu.base,
        MenuNode::ClassSelect => &mut g.class_select_menu.base,
        MenuNode::ClassConfirm => &mut g.class_confirm_menu.base,
        MenuNode::StartTrait => &mut g.start_trait_menu.base,
    }
}

/// Resolves a child discriminant to the [`MenuNode`] to recurse into
/// (`MenuChild::None` → no child). The `ClassConfirm`/`StartTrait` children are now
/// real nodes; the remaining DEFERRED child menu (`Popup`) has no ported node yet —
/// reaching one is the transliteration's next-lane boundary.
fn child_node(child: MenuChild) -> Option<MenuNode> {
    match child {
        MenuChild::None => None,
        MenuChild::ClassSelect => Some(MenuNode::ClassSelect),
        MenuChild::ClassConfirm => Some(MenuNode::ClassConfirm),
        MenuChild::StartTrait => Some(MenuNode::StartTrait),
        MenuChild::Popup => {
            unimplemented!("DEFERRED: PopupMenu (showMessage/showPopup child) — next lane")
        }
    }
}

/// Dispatches the abstract `Menu.paint` to the concrete node's `paint`.
fn paint_node(g: &mut Game, node: MenuNode, origin_x: i32, origin_y: i32) {
    match node {
        MenuNode::Main => main_menu::paint(g, origin_x, origin_y),
        MenuNode::ClassSelect => class_select_menu::paint(g, origin_x, origin_y),
        MenuNode::ClassConfirm => class_confirm_menu::paint(g, origin_x, origin_y),
        MenuNode::StartTrait => start_trait_menu::paint(g, origin_x, origin_y),
    }
}

/// Dispatches the abstract `Menu.handleKey` to the concrete node's `handleKey`.
fn dispatch_handle_key(g: &mut Game, node: MenuNode, action: i32, key_code: i32) -> bool {
    match node {
        MenuNode::Main => main_menu::handle_key(g, action, key_code),
        MenuNode::ClassSelect => class_select_menu::handle_key(g, action, key_code),
        MenuNode::ClassConfirm => class_confirm_menu::handle_key(g, action, key_code),
        MenuNode::StartTrait => start_trait_menu::handle_key(g, action, key_code),
    }
}

/// `public final boolean passKeyToChild(int action, int keyCode)`
/// (`cb.b:(II)Z => []`): forwards a key to the pushed child; returns whether the
/// child consumed it (marking this screen dirty otherwise). Now dispatches to the
/// CURRENT child instead of assuming none.
pub fn pass_key_to_child(g: &mut Game, node: MenuNode, action: i32, key_code: i32) -> bool {
    // if (this.child != null && this.child.handleKey(action, keyCode)) return true;
    let child = node_base(g, node).child;
    if child != MenuChild::None {
        if let Some(cn) = child_node(child) {
            if dispatch_handle_key(g, cn, action, key_code) {
                return true;
            }
        }
    }
    // this.needsRepaint = true; return false;
    node_base_mut(g, node).needs_repaint = true;
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

/// `public final boolean moveCursorHorizontal(int action, int keyCode)`
/// (`cb.d:(II)Z => []`): moves the cursor left/right (keys 4/6 or LEFT/RIGHT
/// actions), with wrap-around. Needed by [`class_select_menu`].
pub fn move_cursor_horizontal(base: &mut MenuBase, action: i32, key_code: i32) -> bool {
    match key_code {
        // case 52: moveCursor((byte) 3); return true;
        52 => {
            move_cursor(base, 3);
            true
        }
        // case 54: moveCursor((byte) 4); return true;
        54 => {
            move_cursor(base, 4);
            true
        }
        _ => match action {
            // case 2: moveCursor((byte) 3); return true;
            2 => {
                move_cursor(base, 3);
                true
            }
            // case 5: moveCursor((byte) 4); return true;
            5 => {
                move_cursor(base, 4);
                true
            }
            // default: return false;
            _ => false,
        },
    }
}

/// `public void moveCursor(byte direction)` (`cb.a:(B)V => []`): steps the cursor
/// one entry in `direction` (4 = forward, else backward), always wrapping.
pub fn move_cursor(base: &mut MenuBase, direction: i8) {
    // stepCursor(direction, true);
    step_cursor(base, direction, true);
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
/// every pushed descendant as needing a repaint. Now recurses into the current
/// child instead of assuming none.
pub fn invalidate_down(g: &mut Game, node: MenuNode) {
    // if (this.child != null) this.child.invalidateDown();
    let child = node_base(g, node).child;
    if let Some(cn) = child_node(child) {
        invalidate_down(g, cn);
    }
    // this.needsRepaint = true;
    node_base_mut(g, node).needs_repaint = true;
}

/// `public final void render(Graphics graphics, int originX, int originY)`
/// (`cb.b:(…Graphics;II)V => []`): lazily repaints when dirty, then recurses into
/// the pushed child. The root is always `MainMenu`; [`render_node`] performs the
/// generic (per-node) walk the abstract `Menu.render` describes.
pub fn render(g: &mut Game, origin_x: i32, origin_y: i32) {
    render_node(g, MenuNode::Main, origin_x, origin_y);
}

/// The generic per-node render walk (the abstract `Menu.render`, dispatched by
/// [`MenuNode`]). With `child == MenuChild::None` this is bit-for-bit the old
/// main-menu-only specialisation.
fn render_node(g: &mut Game, node: MenuNode, origin_x: i32, origin_y: i32) {
    // boolean painted = false;
    let mut painted = false;
    // if (this.needsRepaint) { needsRepaint=false; paint(...); painted=true; }
    if node_base(g, node).needs_repaint {
        node_base_mut(g, node).needs_repaint = false;
        paint_node(g, node, origin_x, origin_y);
        painted = true;
    }
    // if (this.child != null) child.render(...); else if (pendingInitialPaint) { if (!painted) paint(...); pendingInitialPaint=false; }
    let child = node_base(g, node).child;
    match child_node(child) {
        Some(cn) => render_node(g, cn, origin_x, origin_y),
        None => {
            if node_base(g, node).pending_initial_paint {
                if !painted {
                    paint_node(g, node, origin_x, origin_y);
                }
                node_base_mut(g, node).pending_initial_paint = false;
            }
        }
    }
}
