//! Transliterated from `java/src/main/java/defpackage/CostConfirmDialog.java`
//! (original `bo.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Cost-confirmation dialog (`CostConfirmDialog extends Menu`) shown before a paid
//! action — the item-combine fee opened from [`CombineMenu`](crate::combine_menu).
//! Under a [`title`](CostConfirmDialogState::title) it lists the affected item names
//! ([`item_lines`](CostConfirmDialogState::item_lines)), the player's current gold,
//! and the [`cost`](CostConfirmDialogState::cost) to be charged (labelled
//! [`cost_label`](CostConfirmDialogState::cost_label)). Pressing OK reports back to
//! the parent through `onPopupResult` tagged
//! [`result_tag`](CostConfirmDialogState::result_tag); Back closes the dialog.
//!
//! ## ANTI-BOG boundary
//!
//! Constructor + `handleKey` are ported **fully** — the OK/Back
//! `parent.onPopupResult` / `parent.close` callbacks are made real via the flat
//! model's [`parent_of`](crate::menu::parent_of) scan. `paint` is **PARTIAL**: the
//! inset panel + the three outlined caption rows + the title/item/cost captions
//! (pure `Menu`/`FontManager` draw kit, all ported) are drawn; only the two
//! `Menu.drawGold` widgets (the current-gold line and the cost line — `Menu.drawGold`
//! is DEFERRED with the rest of the item/gold draw kit) are DEFERRED. `GameState.hero()`
//! is read only for the DEFERRED current-gold `drawGold`, so it too is DEFERRED.
//!
//! `CostConfirmDialog`'s fields (`title`, `itemLines`, `costLabel`, `cost`,
//! `resultTag`) are all per-INSTANCE (the `Menu` base fields likewise), so it
//! contributes no `java/reconstruction/ownership.tsv` static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `bo.<init>:(Lcb;[C[Ljava/lang/Object;[CIB)V => []` (constructor — field stores),
//! `bo.a:(II)Z => []` (handleKey — pure branches),
//! `bo.a:(…Graphics;II)V => [isub,isub,iadd,…,imul,…]` (paint — the ported panel/
//! caption geometry; `imul` is the item-line `line * 18` stride).

use crate::font_manager;
use crate::game::Game;
use crate::menu::{self, MenuNode};

/// Java `bo` / `CostConfirmDialog` instance state — the `Menu` (`cb`) base fields
/// plus the dialog's own per-instance fields.
#[derive(Debug, Default, Clone)]
pub struct CostConfirmDialogState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private char[] title;` (obf `a`) — dialog title/header line.
    pub title: Vec<u16>,
    /// `private Object[] itemLines;` (obf `a`, `[Ljava/lang/Object;`) — item-name
    /// lines listed in the dialog body (each a `char[]`; entries may be `null`).
    pub item_lines: Vec<Option<Vec<u16>>>,
    /// `private char[] costLabel;` (obf `b`) — label for the cost row.
    pub cost_label: Vec<u16>,
    /// `private int cost;` (obf `a`, `I`) — gold amount charged on confirm.
    pub cost: i32,
    /// `private byte resultTag;` (obf `c`) — tag echoed back through `onPopupResult`.
    pub result_tag: i8,
}

/// `public CostConfirmDialog(Menu parent, char[] title, Object[] itemLines, char[] costLabel, int cost, byte tag)`
/// (`bo.<init>:(Lcb;[C[Ljava/lang/Object;[CIB)V => []`).
pub fn construct(
    g: &mut Game,
    title: Vec<u16>,
    item_lines: Vec<Option<Vec<u16>>>,
    cost_label: Vec<u16>,
    cost: i32,
    tag: i8,
) {
    // super(parent, (byte) 0);   (parent is the opening CombineMenu → present)
    g.cost_confirm_dialog.base = menu::construct(true, 0);
    // this.title = title;
    g.cost_confirm_dialog.title = title;
    // this.resultTag = tag;
    g.cost_confirm_dialog.result_tag = tag;
    // this.itemLines = itemLines;
    g.cost_confirm_dialog.item_lines = item_lines;
    // this.costLabel = costLabel;
    g.cost_confirm_dialog.cost_label = cost_label;
    // this.cost = cost;
    g.cost_confirm_dialog.cost = cost;
}

/// `public final boolean handleKey(int action, int keyCode)` (`bo.a:(II)Z => []`):
/// child forward; Back (`-8`) closes the parent; OK (`53`/action 8) reports
/// `parent.onPopupResult(resultTag, 0)`. Always returns true.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::CostConfirm, action, key_code) {
        return true;
    }
    // if (keyCode == -8) { ((Menu) this).parent.close(); return true; }
    if key_code == -8 {
        let parent = menu::parent_of(g, MenuNode::CostConfirm)
            .expect("NullPointerException: CostConfirmDialog.parent");
        menu::close(g, parent);
        return true;
    }
    // if (keyCode != 53 && action != 8) return true;
    if key_code != 53 && action != 8 {
        return true;
    }
    // ((Menu) this).parent.onPopupResult(this.resultTag, (byte) 0);
    let parent = menu::parent_of(g, MenuNode::CostConfirm)
        .expect("NullPointerException: CostConfirmDialog.parent");
    let tag = g.cost_confirm_dialog.result_tag;
    menu::on_popup_result(g, parent, tag, 0);
    // return true;
    true
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`bo.a:(…Graphics;II)V`): draws the inset panel, the three outlined caption rows,
/// and the title/item-name/cost captions. The two `Menu.drawGold` widgets (current
/// gold + cost) are DEFERRED (the item/gold draw kit is unported).
pub fn paint(g: &mut Game, _x: i32, _y: i32) {
    // int boxX = BaseCanvas.halfW - 67;
    let box_x = g.base_canvas.half_w.wrapping_sub(67);
    // int boxY = BaseCanvas.halfH - 60;
    let box_y = g.base_canvas.half_h.wrapping_sub(60);
    // Hero hero = GameState.hero();
    //   (read only for the DEFERRED current-gold drawGold below — DEFERRED.)
    let title = g.cost_confirm_dialog.title.clone();
    let cost_label = g.cost_confirm_dialog.cost_label.clone();
    let item_lines = g.cost_confirm_dialog.item_lines.clone();

    let Game {
        screen,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // Menu.drawInsetPanel(graphics, boxX, boxY, 135, 120);
    menu::draw_inset_panel(&mut graphics, box_x, box_y, 135, 120);
    // Menu.fillOutlinedRect(graphics, boxX + 3, boxY + 3, 129, 17, 10452863);
    menu::fill_outlined_rect(
        &mut graphics,
        box_x.wrapping_add(3),
        box_y.wrapping_add(3),
        129,
        17,
        10452863,
    );
    // graphics.setColor(16777215);
    graphics.set_color(16777215);
    // FontManager.drawChars(graphics, boxX + 6, boxY + 4, this.title, 1);
    font_manager::draw_chars(
        font_manager,
        &mut graphics,
        box_x.wrapping_add(6),
        box_y.wrapping_add(4),
        &title,
        1,
    );
    // Menu.fillOutlinedRect(graphics, boxX + 3, boxY + 25, 129, 60, 10452863);
    menu::fill_outlined_rect(
        &mut graphics,
        box_x.wrapping_add(3),
        box_y.wrapping_add(25),
        129,
        60,
        10452863,
    );
    // graphics.setColor(16777215);
    graphics.set_color(16777215);
    // for (int line = 0; line < this.itemLines.length; line++) {
    //     if (this.itemLines[line] != null)
    //         FontManager.drawChars(graphics, boxX + 6, boxY + 27 + (line * 18), (char[]) this.itemLines[line], 1);
    // }
    let mut line: i32 = 0;
    while line < item_lines.len() as i32 {
        if let Some(item_line) = &item_lines[line as usize] {
            font_manager::draw_chars(
                font_manager,
                &mut graphics,
                box_x.wrapping_add(6),
                box_y.wrapping_add(27).wrapping_add(line.wrapping_mul(18)),
                item_line,
                1,
            );
        }
        line = line.wrapping_add(1);
    }
    // Menu.drawGold(graphics, (boxX + 135) - 5, boxY + 90, hero.bag.gold);
    // (DEFERRED: Menu.drawGold — the item/gold draw kit is unported.)
    // Menu.fillOutlinedRect(graphics, boxX + 3, boxY + 98, 129, 15, 10452863);
    menu::fill_outlined_rect(
        &mut graphics,
        box_x.wrapping_add(3),
        box_y.wrapping_add(98),
        129,
        15,
        10452863,
    );
    // graphics.setColor(16777215);
    graphics.set_color(16777215);
    // FontManager.drawChars(graphics, boxX + 6, boxY + 99, this.costLabel, 1);
    font_manager::draw_chars(
        font_manager,
        &mut graphics,
        box_x.wrapping_add(6),
        box_y.wrapping_add(99),
        &cost_label,
        1,
    );
    // Menu.drawGold(graphics, (boxX + 135) - 5, boxY + 105, this.cost);
    // (DEFERRED: Menu.drawGold — the item/gold draw kit is unported.)
}
