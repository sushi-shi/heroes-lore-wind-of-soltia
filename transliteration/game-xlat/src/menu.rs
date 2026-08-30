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
//! `invalidateDown`, and the lazy [`render`]. A later increment adds the popup
//! machinery — [`show_popup`]/[`show_popup_labels`]/[`show_message`],
//! [`on_popup_result`] (+ [`on_popup_result_base`]), [`close`], [`invalidate_up`],
//! and the [`parent_of`] parent scan — plus the shared panel draw kit the front-menu
//! dialogs need ([`draw_panel_frame`]/[`fill_panel_interior`] over
//! [`draw_bevel_box`]/[`fill_inset2`]). A later increment adds the paginated
//! scrollable-list kit the item pickers (`ItemPickerList`/`SellList`) need
//! ([`current_page`]/[`page_count`]/[`page_first_index`]/[`page_last_index`],
//! [`draw_inset_panel`] over [`draw_bevel_outline`]/[`fill_inset1`],
//! [`fill_outlined_rect`], [`draw_tab_button`], [`draw_list_page`]) plus
//! [`move_cursor_vertical_no_wrap`]. The remaining static draw kit
//! (`drawButton`/`drawSelectableBox`/`drawTextField`, the item/gold widgets
//! `drawItemIcon`/`drawItemInfo`/`drawQuickSlotRow`/`drawGold`) is not reached by the
//! ported menus and is **DEFERRED**.
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

use crate::about_screen;
use crate::class_confirm_menu;
use crate::class_select_menu;
use crate::confirm_dialog;
use crate::continue_menu;
use crate::game::Game;
use crate::item_picker_list;
use crate::main_menu;
use crate::options_menu;
use crate::popup_menu;
use crate::sell_list;
use crate::start_trait_menu;
use j2me_jvm::{java_div, java_rem};

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
    /// `child instanceof PopupMenu` (`af`) — a `showMessage`/`showPopup` dialog.
    Popup,
    /// `child instanceof ConfirmDialog` (`am`) — the two-line Yes/No dialog.
    Confirm,
    /// `child instanceof ContinueMenu` (`a`) — the load-game slot picker.
    Continue,
    /// `child instanceof OptionsMenu` (`be`) — the options screen.
    Options,
    /// `child instanceof AboutScreen` (`bl`) — the credits/about screen.
    About,
    /// `child instanceof ItemPickerList` (`m`) — the generic scrollable item-slot picker.
    ItemPicker,
    /// `child instanceof SellList` (`bb`) — the shop's sell-from-bag list.
    SellList,
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
    /// `PopupMenu` (`af`) — pushed by any menu's `showPopup`/`showMessage`.
    Popup,
    /// `ConfirmDialog` (`am`) — pushed by `SkillTab`'s confirm prompts.
    Confirm,
    /// `ContinueMenu` (`a`) — pushed as `MainMenu`'s child by Continue.
    Continue,
    /// `OptionsMenu` (`be`) — pushed as `MainMenu`'s (or `SystemTab`'s) child by Options.
    Options,
    /// `AboutScreen` (`bl`) — pushed as `MainMenu`'s child by FIRE-select case 4.
    About,
    /// `ItemPickerList` (`m`) — the generic item-slot picker (pushed by equip/craft menus).
    ItemPicker,
    /// `SellList` (`bb`) — the shop sell list (pushed by the shop menu; extends `ItemPickerList`).
    SellList,
}

/// Every concrete [`MenuNode`], for the parent-scan ([`parent_of`]). The flat model
/// is a singleton stack, so a node's parent is the unique node whose resolved
/// [`child_node`] is that node.
const ALL_NODES: [MenuNode; 11] = [
    MenuNode::Main,
    MenuNode::ClassSelect,
    MenuNode::ClassConfirm,
    MenuNode::StartTrait,
    MenuNode::Popup,
    MenuNode::Confirm,
    MenuNode::Continue,
    MenuNode::Options,
    MenuNode::About,
    MenuNode::ItemPicker,
    MenuNode::SellList,
];

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
        MenuNode::Popup => &g.popup_menu.base,
        MenuNode::Confirm => &g.confirm_dialog.base,
        MenuNode::Continue => &g.continue_menu.base,
        MenuNode::Options => &g.options_menu.base,
        MenuNode::About => &g.about_screen.base,
        MenuNode::ItemPicker => &g.item_picker_list.base,
        MenuNode::SellList => &g.sell_list.picker.base,
    }
}

/// The [`MenuBase`] of a concrete menu node (mutable borrow).
fn node_base_mut(g: &mut Game, node: MenuNode) -> &mut MenuBase {
    match node {
        MenuNode::Main => &mut g.main_menu.base,
        MenuNode::ClassSelect => &mut g.class_select_menu.base,
        MenuNode::ClassConfirm => &mut g.class_confirm_menu.base,
        MenuNode::StartTrait => &mut g.start_trait_menu.base,
        MenuNode::Popup => &mut g.popup_menu.base,
        MenuNode::Confirm => &mut g.confirm_dialog.base,
        MenuNode::Continue => &mut g.continue_menu.base,
        MenuNode::Options => &mut g.options_menu.base,
        MenuNode::About => &mut g.about_screen.base,
        MenuNode::ItemPicker => &mut g.item_picker_list.base,
        MenuNode::SellList => &mut g.sell_list.picker.base,
    }
}

/// Resolves a child discriminant to the [`MenuNode`] to recurse into
/// (`MenuChild::None` → no child). Every child menu is now a real ported node.
fn child_node(child: MenuChild) -> Option<MenuNode> {
    match child {
        MenuChild::None => None,
        MenuChild::ClassSelect => Some(MenuNode::ClassSelect),
        MenuChild::ClassConfirm => Some(MenuNode::ClassConfirm),
        MenuChild::StartTrait => Some(MenuNode::StartTrait),
        MenuChild::Popup => Some(MenuNode::Popup),
        MenuChild::Confirm => Some(MenuNode::Confirm),
        MenuChild::Continue => Some(MenuNode::Continue),
        MenuChild::Options => Some(MenuNode::Options),
        MenuChild::About => Some(MenuNode::About),
        MenuChild::ItemPicker => Some(MenuNode::ItemPicker),
        MenuChild::SellList => Some(MenuNode::SellList),
    }
}

/// `((Menu) this).parent` — the flat model's parent link. The child stack is a
/// singleton chain, so a node's parent is the unique other node whose current
/// [`child`](MenuBase::child) resolves (via [`child_node`]) to it (`None` at the
/// root, or when the node is not currently linked into the stack).
pub fn parent_of(g: &Game, node: MenuNode) -> Option<MenuNode> {
    for &candidate in ALL_NODES.iter() {
        if candidate == node {
            continue;
        }
        if child_node(node_base(g, candidate).child) == Some(node) {
            return Some(candidate);
        }
    }
    None
}

/// Dispatches the abstract `Menu.paint` to the concrete node's `paint`.
fn paint_node(g: &mut Game, node: MenuNode, origin_x: i32, origin_y: i32) {
    match node {
        MenuNode::Main => main_menu::paint(g, origin_x, origin_y),
        MenuNode::ClassSelect => class_select_menu::paint(g, origin_x, origin_y),
        MenuNode::ClassConfirm => class_confirm_menu::paint(g, origin_x, origin_y),
        MenuNode::StartTrait => start_trait_menu::paint(g, origin_x, origin_y),
        MenuNode::Popup => popup_menu::paint(g, origin_x, origin_y),
        MenuNode::Confirm => confirm_dialog::paint(g, origin_x, origin_y),
        MenuNode::Continue => continue_menu::paint(g, origin_x, origin_y),
        MenuNode::Options => options_menu::paint(g, origin_x, origin_y),
        MenuNode::About => about_screen::paint(g, origin_x, origin_y),
        MenuNode::ItemPicker => item_picker_list::paint(g, origin_x, origin_y),
        MenuNode::SellList => sell_list::paint(g, origin_x, origin_y),
    }
}

/// Dispatches the abstract `Menu.handleKey` to the concrete node's `handleKey`.
fn dispatch_handle_key(g: &mut Game, node: MenuNode, action: i32, key_code: i32) -> bool {
    match node {
        MenuNode::Main => main_menu::handle_key(g, action, key_code),
        MenuNode::ClassSelect => class_select_menu::handle_key(g, action, key_code),
        MenuNode::ClassConfirm => class_confirm_menu::handle_key(g, action, key_code),
        MenuNode::StartTrait => start_trait_menu::handle_key(g, action, key_code),
        MenuNode::Popup => popup_menu::handle_key(g, action, key_code),
        MenuNode::Confirm => confirm_dialog::handle_key(g, action, key_code),
        MenuNode::Continue => continue_menu::handle_key(g, action, key_code),
        MenuNode::Options => options_menu::handle_key(g, action, key_code),
        MenuNode::About => about_screen::handle_key(g, action, key_code),
        MenuNode::ItemPicker => item_picker_list::handle_key(g, action, key_code),
        MenuNode::SellList => sell_list::handle_key(g, action, key_code),
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

/// `public final boolean moveCursorVerticalNoWrap(int action, int keyCode)`
/// (`cb.c:(II)Z => []`): [`move_cursor_vertical`] without wrap-around (stops at the
/// ends). Used by the scrollable leaf lists (`AboutScreen`, `ItemPickerList`,
/// `SellList`).
pub fn move_cursor_vertical_no_wrap(base: &mut MenuBase, action: i32, key_code: i32) -> bool {
    // return moveCursorVertical(action, keyCode, false);
    move_cursor_vertical(base, action, key_code, false)
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

// --------------------------------------------------------------------------
// Popup / dialog machinery (the `Menu` push + result callbacks)
// --------------------------------------------------------------------------

/// `public void onPopupResult(byte tag, byte result)` (`cb.a:(BB)V => []`): the
/// **base** `Menu.onPopupResult` — dismiss the child, reactivate the game screen,
/// and mark the ancestors dirty. Concrete overrides (e.g.
/// [`main_menu::on_popup_result`]) call this as their `super`.
pub fn on_popup_result_base(g: &mut Game, node: MenuNode, _tag: i8, _result: i8) {
    // this.child = null;
    node_base_mut(g, node).child = MenuChild::None;
    // if (GameLoop.gameScreen != null) GameLoop.gameScreen.activate();
    if g.game_loop.game_screen {
        // (DEFERRED: GameScreen.activate() — GameScreen not ported this lane;
        //  GameLoop.gameScreen is null on the front-menu path, so this never runs.)
    }
    // invalidateUp();
    invalidate_up(g, node);
}

/// Dispatches the virtual `Menu.onPopupResult(tag, result)` to `node`'s concrete
/// override (only `MainMenu` overrides it among the ported menus; the rest use the
/// base [`on_popup_result_base`]).
pub fn on_popup_result(g: &mut Game, node: MenuNode, tag: i8, result: i8) {
    match node {
        MenuNode::Main => main_menu::on_popup_result(g, tag, result),
        MenuNode::SellList => sell_list::on_popup_result(g, tag, result),
        _ => on_popup_result_base(g, node, tag, result),
    }
}

/// `public final void close()` (`cb.a:()V => []`): drops the pushed child,
/// reactivates the game screen, and marks the ancestors dirty. Identical body to the
/// base [`on_popup_result_base`] (Java's `Menu.close`/`Menu.onPopupResult` share it).
pub fn close(g: &mut Game, node: MenuNode) {
    // this.child = null;
    node_base_mut(g, node).child = MenuChild::None;
    // if (GameLoop.gameScreen != null) GameLoop.gameScreen.activate();
    if g.game_loop.game_screen {
        // (DEFERRED: GameScreen.activate() — see on_popup_result_base.)
    }
    // invalidateUp();
    invalidate_up(g, node);
}

/// `public final void invalidateUp()` (`cb.b:()V => []`): marks this screen and
/// every ancestor as needing a repaint. Walks up via [`parent_of`] (the flat model
/// of the `parent` reference chain).
pub fn invalidate_up(g: &mut Game, node: MenuNode) {
    // if (this.parent != null) this.parent.invalidateUp();
    if let Some(parent) = parent_of(g, node) {
        invalidate_up(g, parent);
    }
    // this.needsRepaint = true;
    node_base_mut(g, node).needs_repaint = true;
}

/// `public final void showPopup(byte style, byte tag, Object[] lines)`
/// (`cb.a:(BB[Ljava/lang/Object;)V => []`):
/// `child = new PopupMenu(this, style, tag, lines, null, null)`.
pub fn show_popup(g: &mut Game, node: MenuNode, style: i8, tag: i8, lines: Vec<Vec<u16>>) {
    // this.child = new PopupMenu(this, style, tag, lines, null, null);
    popup_menu::construct(g, style, tag, lines, None, None);
    node_base_mut(g, node).child = MenuChild::Popup;
}

/// `public final void showPopup(byte style, byte tag, Object[] lines, char[] okLabel, char[] cancelLabel)`
/// (`cb.a:(BB[Ljava/lang/Object;[C[C)V => []`): the custom-label overload.
pub fn show_popup_labels(
    g: &mut Game,
    node: MenuNode,
    style: i8,
    tag: i8,
    lines: Vec<Vec<u16>>,
    ok_label: Option<Vec<u16>>,
    cancel_label: Option<Vec<u16>>,
) {
    // this.child = new PopupMenu(this, style, tag, lines, okLabel, cancelLabel);
    popup_menu::construct(g, style, tag, lines, ok_label, cancel_label);
    node_base_mut(g, node).child = MenuChild::Popup;
}

/// `public final void showMessage(Object[] lines)`
/// (`cb.a:([Ljava/lang/Object;)V => []`):
/// `child = new PopupMenu(this, (byte) 1, (byte) 0, lines, null, null)`.
pub fn show_message(g: &mut Game, node: MenuNode, lines: Vec<Vec<u16>>) {
    // this.child = new PopupMenu(this, (byte) 1, (byte) 0, lines, null, null);
    popup_menu::construct(g, 1, 0, lines, None, None);
    node_base_mut(g, node).child = MenuChild::Popup;
}

// --------------------------------------------------------------------------
// Shared panel draw kit (the beveled-box statics the front-menu dialogs use)
// --------------------------------------------------------------------------

/// `public static final void drawPanelFrame(Graphics graphics, int x, int y, int width, int height)`
/// (`cb`): the standard beveled panel outline.
pub fn draw_panel_frame(graphics: &mut j2me_me::Graphics, x: i32, y: i32, width: i32, height: i32) {
    // drawBevelBox(graphics, x, y, width, height, 2039615, 6242111, 2039615);
    draw_bevel_box(graphics, x, y, width, height, 2039615, 6242111, 2039615);
}

/// `public static final void fillPanelInterior(Graphics graphics, int x, int y, int width, int height)`
/// (`cb`): fills the panel interior (inset two pixels) with the menu-blue background.
pub fn fill_panel_interior(
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    // fillInset2(graphics, x, y, width, height, 4136767);
    fill_inset2(graphics, x, y, width, height, 4136767);
}

/// `public static final void drawBevelBox(Graphics graphics, int x, int y, int width, int height, int frame, int highlight, int shadow)`
/// (`cb`): a raised beveled box outline — `frame` edges, `highlight` inner top/left,
/// `shadow` inner bottom/right. All offsets are `+1`/`-2`/`-3` constant iadd/isub.
// The eight-parameter list mirrors the Java `drawBevelBox` signature verbatim.
#[allow(clippy::too_many_arguments)]
pub fn draw_bevel_box(
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    frame: i32,
    highlight: i32,
    shadow: i32,
) {
    // graphics.setColor(frame);
    graphics.set_color(frame);
    // graphics.drawLine(x + 1, y, (x + width) - 2, y);
    graphics.draw_line(
        x.wrapping_add(1),
        y,
        x.wrapping_add(width).wrapping_sub(2),
        y,
    );
    // graphics.drawLine((x + width) - 1, y + 1, (x + width) - 1, (y + height) - 2);
    graphics.draw_line(
        x.wrapping_add(width).wrapping_sub(1),
        y.wrapping_add(1),
        x.wrapping_add(width).wrapping_sub(1),
        y.wrapping_add(height).wrapping_sub(2),
    );
    // graphics.drawLine(x + 1, (y + height) - 1, (x + width) - 2, (y + height) - 1);
    graphics.draw_line(
        x.wrapping_add(1),
        y.wrapping_add(height).wrapping_sub(1),
        x.wrapping_add(width).wrapping_sub(2),
        y.wrapping_add(height).wrapping_sub(1),
    );
    // graphics.drawLine(x, y + 1, x, (y + height) - 2);
    graphics.draw_line(
        x,
        y.wrapping_add(1),
        x,
        y.wrapping_add(height).wrapping_sub(2),
    );
    // graphics.setColor(highlight);
    graphics.set_color(highlight);
    // graphics.drawLine(x + 1, y + 1, (x + width) - 3, y + 1);
    graphics.draw_line(
        x.wrapping_add(1),
        y.wrapping_add(1),
        x.wrapping_add(width).wrapping_sub(3),
        y.wrapping_add(1),
    );
    // graphics.drawLine(x + 1, y + 1, x + 1, (y + height) - 3);
    graphics.draw_line(
        x.wrapping_add(1),
        y.wrapping_add(1),
        x.wrapping_add(1),
        y.wrapping_add(height).wrapping_sub(3),
    );
    // graphics.setColor(shadow);
    graphics.set_color(shadow);
    // graphics.drawLine((x + width) - 2, y + 1, (x + width) - 2, (y + height) - 3);
    graphics.draw_line(
        x.wrapping_add(width).wrapping_sub(2),
        y.wrapping_add(1),
        x.wrapping_add(width).wrapping_sub(2),
        y.wrapping_add(height).wrapping_sub(3),
    );
    // graphics.drawLine(x + 1, (y + height) - 2, (x + width) - 2, (y + height) - 2);
    graphics.draw_line(
        x.wrapping_add(1),
        y.wrapping_add(height).wrapping_sub(2),
        x.wrapping_add(width).wrapping_sub(2),
        y.wrapping_add(height).wrapping_sub(2),
    );
}

/// `public static final void fillInset2(Graphics graphics, int x, int y, int width, int height, int color)`
/// (`cb`): fills a `width`×`height` box inset by two pixels with `color`.
pub fn fill_inset2(
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: i32,
) {
    // graphics.setColor(color);
    graphics.set_color(color);
    // graphics.fillRect(x + 2, y + 2, width - 4, height - 4);
    graphics.fill_rect(
        x.wrapping_add(2),
        y.wrapping_add(2),
        width.wrapping_sub(4),
        height.wrapping_sub(4),
    );
}

// --------------------------------------------------------------------------
// Scrollable-list pagination + draw kit (the five-entry paginated list the item
// pickers use: `ItemPickerList` / `SellList`)
// --------------------------------------------------------------------------

/// `public final int currentPage()` (`cb`): the one-based page the cursor sits on
/// (five entries per page): `(cursorIndex / 5) + 1`.
pub fn current_page(base: &MenuBase) -> i32 {
    // return (this.cursorIndex / 5) + 1;
    java_div(base.cursor_index as i32, 5)
        .expect("cursorIndex / 5")
        .wrapping_add(1)
}

/// `public final int pageCount()` (`cb`): total number of five-entry pages:
/// `((itemCount - 1) / 5) + 1`.
pub fn page_count(base: &MenuBase) -> i32 {
    // return ((this.itemCount - 1) / 5) + 1;
    java_div((base.item_count as i32).wrapping_sub(1), 5)
        .expect("(itemCount - 1) / 5")
        .wrapping_add(1)
}

/// `public final int pageFirstIndex()` (`cb`): index of the first entry shown on
/// the current page: `(currentPage() - 1) * 5`.
pub fn page_first_index(base: &MenuBase) -> i32 {
    // return (currentPage() - 1) * 5;
    current_page(base).wrapping_sub(1).wrapping_mul(5)
}

/// `public final int pageLastIndex()` (`cb`): index of the last entry shown on the
/// current page, clamped to the list end.
pub fn page_last_index(base: &MenuBase) -> i32 {
    // int last = (currentPage() * 5) - 1;
    let last = current_page(base).wrapping_mul(5).wrapping_sub(1);
    // return last > this.itemCount - 1 ? this.itemCount - 1 : last;
    if last > (base.item_count as i32).wrapping_sub(1) {
        (base.item_count as i32).wrapping_sub(1)
    } else {
        last
    }
}

/// `private static final void drawBevelOutline(Graphics graphics, int x, int y, int width, int height, int lightColor, int darkColor)`
/// (`cb`): a two-tone rectangle outline — `lightColor` top/left, `darkColor`
/// bottom/right.
pub fn draw_bevel_outline(
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    light_color: i32,
    dark_color: i32,
) {
    // graphics.setColor(lightColor);
    graphics.set_color(light_color);
    // graphics.drawLine(x + 1, y, (x + width) - 2, y);
    graphics.draw_line(
        x.wrapping_add(1),
        y,
        x.wrapping_add(width).wrapping_sub(2),
        y,
    );
    // graphics.drawLine(x, y + 1, x, (y + height) - 2);
    graphics.draw_line(
        x,
        y.wrapping_add(1),
        x,
        y.wrapping_add(height).wrapping_sub(2),
    );
    // graphics.setColor(darkColor);
    graphics.set_color(dark_color);
    // graphics.drawLine((x + width) - 1, y + 1, (x + width) - 1, (y + height) - 1);
    graphics.draw_line(
        x.wrapping_add(width).wrapping_sub(1),
        y.wrapping_add(1),
        x.wrapping_add(width).wrapping_sub(1),
        y.wrapping_add(height).wrapping_sub(1),
    );
    // graphics.drawLine(x + 1, (y + height) - 1, (x + width) - 2, (y + height) - 1);
    graphics.draw_line(
        x.wrapping_add(1),
        y.wrapping_add(height).wrapping_sub(1),
        x.wrapping_add(width).wrapping_sub(2),
        y.wrapping_add(height).wrapping_sub(1),
    );
}

/// `private static final void fillInset1(Graphics graphics, int x, int y, int width, int height, int color)`
/// (`cb`): fills a `width`×`height` box inset by one pixel with `color`.
pub fn fill_inset1(
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: i32,
) {
    // graphics.setColor(color);
    graphics.set_color(color);
    // graphics.fillRect(x + 1, y + 1, width - 2, height - 2);
    graphics.fill_rect(
        x.wrapping_add(1),
        y.wrapping_add(1),
        width.wrapping_sub(2),
        height.wrapping_sub(2),
    );
}

/// `public static final void fillOutlinedRect(Graphics graphics, int x, int y, int width, int height, int color)`
/// (`cb`): a single-colour rectangle outline plus filled interior.
pub fn fill_outlined_rect(
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: i32,
) {
    // graphics.setColor(color);
    graphics.set_color(color);
    // graphics.drawLine(x + 1, y, (x + width) - 2, y);
    graphics.draw_line(
        x.wrapping_add(1),
        y,
        x.wrapping_add(width).wrapping_sub(2),
        y,
    );
    // graphics.drawLine(x, y + 1, x, (y + height) - 2);
    graphics.draw_line(
        x,
        y.wrapping_add(1),
        x,
        y.wrapping_add(height).wrapping_sub(2),
    );
    // graphics.drawLine((x + width) - 1, y + 1, (x + width) - 1, (y + height) - 2);
    graphics.draw_line(
        x.wrapping_add(width).wrapping_sub(1),
        y.wrapping_add(1),
        x.wrapping_add(width).wrapping_sub(1),
        y.wrapping_add(height).wrapping_sub(2),
    );
    // graphics.drawLine(x + 1, (y + height) - 1, (x + width) - 2, (y + height) - 1);
    graphics.draw_line(
        x.wrapping_add(1),
        y.wrapping_add(height).wrapping_sub(1),
        x.wrapping_add(width).wrapping_sub(2),
        y.wrapping_add(height).wrapping_sub(1),
    );
    // graphics.fillRect(x + 1, y + 1, width - 2, height - 2);
    graphics.fill_rect(
        x.wrapping_add(1),
        y.wrapping_add(1),
        width.wrapping_sub(2),
        height.wrapping_sub(2),
    );
}

/// `public static final void drawInsetPanel(Graphics graphics, int x, int y, int width, int height)`
/// (`cb`): a two-tone bevel outline over a filled interior.
pub fn draw_inset_panel(graphics: &mut j2me_me::Graphics, x: i32, y: i32, width: i32, height: i32) {
    // drawBevelOutline(graphics, x, y, width, height, 16768959, 12558207);
    draw_bevel_outline(graphics, x, y, width, height, 16768959, 12558207);
    // fillInset1(graphics, x, y, width, height, 14663551);
    fill_inset1(graphics, x, y, width, height, 14663551);
}

/// `public static final void drawTabButton(Graphics graphics, int x, int y, byte index, boolean selected)`
/// (`cb`): the selection-slot cursor box for tab/row `index`; `selected` chooses the
/// lit palette.
pub fn draw_tab_button(
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    index: i8,
    selected: bool,
) {
    // int slotX = x + 3;
    let slot_x = x.wrapping_add(3);
    // int slotY = y + 10 + (index * 23);
    let slot_y = y
        .wrapping_add(10)
        .wrapping_add((index as i32).wrapping_mul(23));
    // graphics.setColor(selected ? 4136767 : 6242111);
    graphics.set_color(if selected { 4136767 } else { 6242111 });
    // graphics.fillRect(slotX + 1, slotY, 24, 1);
    graphics.fill_rect(slot_x.wrapping_add(1), slot_y, 24, 1);
    // graphics.fillRect(slotX, slotY + 1, 1, 16);
    graphics.fill_rect(slot_x, slot_y.wrapping_add(1), 1, 16);
    // graphics.fillRect(slotX + 1, slotY + 17, 24, 1);
    graphics.fill_rect(slot_x.wrapping_add(1), slot_y.wrapping_add(17), 24, 1);
    // graphics.setColor(selected ? 10452799 : 14663551);
    graphics.set_color(if selected { 10452799 } else { 14663551 });
    // graphics.fillRect(slotX + 1, slotY + 1, 24, 1);
    graphics.fill_rect(slot_x.wrapping_add(1), slot_y.wrapping_add(1), 24, 1);
    // graphics.fillRect(slotX + 1, slotY + 1, 1, 16);
    graphics.fill_rect(slot_x.wrapping_add(1), slot_y.wrapping_add(1), 1, 16);
    // graphics.setColor(selected ? 4144959 : 8347519);
    graphics.set_color(if selected { 4144959 } else { 8347519 });
    // graphics.fillRect(slotX + 2, slotY + 16, 23, 1);
    graphics.fill_rect(slot_x.wrapping_add(2), slot_y.wrapping_add(16), 23, 1);
    // graphics.setColor(selected ? 6242111 : 10452863);
    graphics.set_color(if selected { 6242111 } else { 10452863 });
    // graphics.fillRect(slotX + 2, slotY + 2, 24, 14);
    graphics.fill_rect(slot_x.wrapping_add(2), slot_y.wrapping_add(2), 24, 14);
}

/// `public final void drawListPage(Graphics graphics, int x, int y, boolean arrows)`
/// (`cb`): draws the current page of a scrolling five-slot list. **PARTIAL** — the
/// up/down scroll arrows (`AssetCache.scrollUpArrow`/`scrollDownArrow`) are DEFERRED
/// (that art bank is unported); the tab buttons and the content box are drawn.
pub fn draw_list_page(
    graphics: &mut j2me_me::Graphics,
    base: &MenuBase,
    x: i32,
    y: i32,
    arrows: bool,
) {
    // byte selected = (byte) (this.cursorIndex % 5);
    let selected = java_rem(base.cursor_index as i32, 5).expect("cursorIndex % 5") as i8;
    // int remaining = this.itemCount - ((currentPage() - 1) * 5);
    let remaining =
        (base.item_count as i32).wrapping_sub(current_page(base).wrapping_sub(1).wrapping_mul(5));
    // int rowsOnPage = remaining; if (remaining > 5) rowsOnPage = 5;
    let mut rows_on_page = remaining;
    if remaining > 5 {
        rows_on_page = 5;
    }
    // for (byte row = 0; row < rowsOnPage; row = (byte) (row + 1)) if (row != selected) drawTabButton(...false);
    let mut row: i8 = 0;
    while (row as i32) < rows_on_page {
        if row != selected {
            draw_tab_button(graphics, x, y, row, false);
        }
        row = (row as i32).wrapping_add(1) as i8;
    }
    // drawBevelBox(graphics, x + 27, y + 10, 120, 137, 4136767, 10452799, 4144959);
    draw_bevel_box(
        graphics,
        x.wrapping_add(27),
        y.wrapping_add(10),
        120,
        137,
        4136767,
        10452799,
        4144959,
    );
    // fillInset2(graphics, x + 27, y + 10, 120, 137, 6242111);
    fill_inset2(
        graphics,
        x.wrapping_add(27),
        y.wrapping_add(10),
        120,
        137,
        6242111,
    );
    // drawTabButton(graphics, x, y, selected, true);
    draw_tab_button(graphics, x, y, selected, true);
    // if (arrows) { if (currentPage() > 1) drawImage(scrollUpArrow, x+70, y+4, 20);
    //               if (currentPage() < pageCount()) drawImage(scrollDownArrow, x+70, y+148, 20); }
    if arrows {
        // (DEFERRED: AssetCache.scrollUpArrow / scrollDownArrow — that art bank is not
        //  modelled in the partial AssetCache; the page-arrow overlays are skipped.)
    }
}
