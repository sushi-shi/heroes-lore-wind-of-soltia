//! Transliterated from `java/src/main/java/defpackage/ConfirmDialog.java`
//! (original `am.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Two-line confirmation dialog (`ConfirmDialog extends Menu`) used by `SkillTab`
//! for the learn-skill / accept-quest prompts. It draws a highlighted title
//! ([`line1`](ConfirmDialogState::line1)) above a body line
//! ([`line2`](ConfirmDialogState::line2)) in a centered panel. Pressing OK reports
//! `result = 1` and Back reports `result = 0` to the parent through
//! `onPopupResult`, tagged with [`result_tag`](ConfirmDialogState::result_tag) so
//! the parent knows which prompt answered.
//!
//! ## ANTI-BOG boundary
//!
//! This increment ports the constructor (field stores) and `handleKey` (fully — the
//! dialog->parent `onPopupResult` callback is made **real** via the flat model's
//! [`parent_of`](crate::menu::parent_of) scan), plus a **PARTIAL** `paint` (the
//! centered panel frame + interior only). The `lineCount` measurement
//! (`FontManager.lineCount`) is DEFERRED (unported), so `lineCount` stays `0` and
//! the panel height reflects that; the two body texts
//! (`FontManager.drawWrappedText`) are DEFERRED too. `ConfirmDialog`'s only creator
//! (`SkillTab`) is not yet ported — it is reachable in this increment only by being
//! pushed as a menu's child directly.
//!
//! `ConfirmDialog`'s fields are all per-INSTANCE (the `Menu` base fields likewise),
//! so it contributes no `java/reconstruction/ownership.tsv` static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `am.<init>:(Lcb;[C[CB)V => [iadd,i2b,iadd,i2b]` (constructor — the two
//! `lineCount += lineCount(lineN, 135)` accumulations, both DEFERRED),
//! `am.a:(II)Z => []` (handleKey — pure branches), and
//! `am.a:(…Graphics;II)V => [imul,iadd,isub,idiv,isub,iadd,iadd,imul,iadd,iadd]`
//! (paint — the ported prefix is `[imul (lineCount*15), iadd (+10), isub (halfW-72),
//! idiv (dialogHeight/2), isub (halfH-…)]`; the remaining `iadd/imul`s live in the
//! DEFERRED body-text draw).

use crate::game::Game;
use crate::menu::{self, MenuNode};
use j2me_jvm::java_div;

/// Java `am` / `ConfirmDialog` instance state — the `Menu` (`cb`) base fields plus
/// the dialog's own instance fields.
#[derive(Debug, Default, Clone)]
pub struct ConfirmDialogState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private char[] line1;` (obf `a`) — highlighted title/prompt line.
    pub line1: Vec<u16>,
    /// `private char[] line2;` (obf `b`) — body/detail line under the title.
    pub line2: Vec<u16>,
    /// `private byte lineCount;` (obf `d`) — total wrapped line count driving the
    /// panel height (DEFERRED measurement → 0).
    pub line_count: i8,
    /// `public byte resultTag;` (obf `c`) — caller tag echoed back through
    /// `onPopupResult`.
    pub result_tag: i8,
}

/// `public ConfirmDialog(Menu parent, char[] line1, char[] line2, byte tag)`
/// (`am.<init>:(Lcb;[C[CB)V => [iadd,i2b,iadd,i2b]`): stores the two lines and the
/// tag; the wrapped `lineCount` accumulation is DEFERRED (`FontManager.lineCount`
/// unported), leaving `lineCount` at `0`.
pub fn construct(g: &mut Game, line1: Vec<u16>, line2: Vec<u16>, tag: i8) {
    // super(parent, (byte) 0);   (parent is the caller → non-null → present)
    g.confirm_dialog.base = menu::construct(true, 0);
    // this.line1 = line1;
    g.confirm_dialog.line1 = line1;
    // this.line2 = line2;
    g.confirm_dialog.line2 = line2;
    // this.resultTag = tag;
    g.confirm_dialog.result_tag = tag;
    // this.lineCount = (byte) 0;
    g.confirm_dialog.line_count = 0;
    // this.lineCount = (byte) (this.lineCount + FontManager.lineCount(line1, 135));   [iadd, i2b]
    // this.lineCount = (byte) (this.lineCount + FontManager.lineCount(line2, 135));   [iadd, i2b]
    // (DEFERRED: FontManager.lineCount unported; lineCount stays 0.)
}

/// `public final boolean handleKey(int action, int keyCode)` (`am.a:(II)Z => []`):
/// forwards to any child, then OK (`53`/action 8) → `onPopupResult(resultTag, 1)`,
/// Back (`-8`) → `onPopupResult(resultTag, 0)`. Always returns true.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Confirm, action, key_code) {
        return true;
    }
    // if (keyCode == 53 || action == 8) { parent.onPopupResult(resultTag, (byte) 1); return true; }
    if key_code == 53 || action == 8 {
        confirm_result(g, 1);
        return true;
    }
    // if (keyCode != -8) return true;
    if key_code != -8 {
        return true;
    }
    // parent.onPopupResult(resultTag, (byte) 0); return true;
    confirm_result(g, 0);
    true
}

/// `((Menu) this).parent.onPopupResult(this.resultTag, result)` — the flat model
/// resolves the parent via [`menu::parent_of`] and dispatches the virtual callback.
fn confirm_result(g: &mut Game, result: i8) {
    let tag = g.confirm_dialog.result_tag;
    let parent =
        menu::parent_of(g, MenuNode::Confirm).expect("NullPointerException: ConfirmDialog.parent");
    menu::on_popup_result(g, parent, tag, result);
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`am.a:(…Graphics;II)V`): **PARTIAL** — the centered panel frame + interior. The
/// two body texts (`FontManager.drawWrappedText`) and the `line2Y` offset
/// (`FontManager.lineCount`) are DEFERRED (unported).
pub fn paint(g: &mut Game, _x: i32, _y: i32) {
    // int dialogHeight = (this.lineCount * 15) + 10;
    let dialog_height = (g.confirm_dialog.line_count as i32)
        .wrapping_mul(15)
        .wrapping_add(10);
    // int boxX = BaseCanvas.halfW - 72;
    let box_x = g.base_canvas.half_w.wrapping_sub(72);
    // int boxY = BaseCanvas.halfH - (dialogHeight / 2);
    let box_y = g
        .base_canvas
        .half_h
        .wrapping_sub(java_div(dialog_height, 2).expect("dialogHeight / 2"));

    let Game { screen, .. } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // Menu.drawPanelFrame(graphics, boxX, boxY, 145, dialogHeight);
    menu::draw_panel_frame(&mut graphics, box_x, box_y, 145, dialog_height);
    // Menu.fillPanelInterior(graphics, boxX, boxY, 145, dialogHeight);
    menu::fill_panel_interior(&mut graphics, box_x, box_y, 145, dialog_height);
    // graphics.setColor(14663551);   (the title colour, before the DEFERRED text)
    graphics.set_color(14663551);

    // (DEFERRED — the two body texts cross into unported FontManager statics. Faithful
    //  full form:
    //    int textY = boxY + 5;
    //    FontManager.drawWrappedText(graphics, boxX + 5, textY, 135, 1, this.line1);
    //    int line2Y = textY + (15 * FontManager.lineCount(this.line1, 135));
    //    graphics.setColor(16777215);
    //    FontManager.drawWrappedText(graphics, boxX + 5, line2Y, 135, 1, this.line2);
    //  FontManager.drawWrappedText / lineCount are unported.)
}
