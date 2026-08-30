//! Transliterated from `java/src/main/java/defpackage/GuardianTab.java`
//! (original `bm.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Guardian tab (tab 3) of [`CharacterMenu`](crate::character_menu): the five guardian
//! companion slots. The cursor starts on the active guardian; OK on a filled slot opens
//! a popup to either make it the active guardian (with a summon check) or open its
//! `GuardianSkillPanel`. The right side shows the selected guardian's name/level/exp.
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. `handleKey` is fully real — the `hero.guardians[cursorIndex]
//! == null` slot test (over the modelled `Hero.guardians` array) gates the
//! ported `showPopup(3, …)`. The `<init>` active-guardian scan is DEFERRED: it compares
//! `hero.guardians[slot] == hero.getActiveGuardian()`, but `Guardian` is unported (its
//! `GuardianRef` is a placeholder) and `Hero.getActiveGuardian` (which asserts non-null)
//! is not ported — guardians/active-guardian are null in this slice (guardian creation
//! DEFERRED in `Hero.initClass`), so the scan is skipped and `cursorIndex` stays `0`. In
//! `onPopupResult` the `previousChild instanceof PopupMenu`/`tag == 3` gate is real; the
//! make-active (`Hero.setActiveGuardian` + `AssetCache.guardianText`) and skill-panel
//! (`GuardianSkillPanel`) commits are DEFERRED (all unported). In `paint` the paginated
//! `drawListPage` and the empty-selection `FontManager.drawChars(text.get(31))` are
//! drawn; the per-slot guardian icons (`AssetCache.guardianIcons` + `Guardian.type`), the
//! selected-guardian detail (name/level/exp bar), and the `BaseCanvas.drawLabelBox`
//! header are DEFERRED (Guardian + those art banks unported; the detail is unreachable
//! this slice — every guardian slot is null).
//!
//! `GuardianTab` has no fields of its own and no `static`s → no
//! `java/reconstruction/ownership.tsv` rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bm.<init>:(Lcb;)V => [iadd,i2b]`
//! (the DEFERRED scan's `slot++` step), `bm.a:(II)Z => []` (handleKey — pure branches),
//! `bm.a:(BB)V => []` (onPopupResult — pure branches), `bm.a:(…Graphics;II)V => [iinc,
//! iinc,iadd,iadd,iadd,irem,imul,iadd,iinc,iadd,iadd,…,imul,idiv,iadd,iadd,iadd,iadd]`
//! (paint — the ported `panelX=x+2`/`panelY=y+15` (iadd,iadd) + the empty-selection
//! `drawChars` at `panelX+34`/`panelY+18` (iadd,iadd); the icon-loop `irem,imul` and the
//! exp-bar `imul,idiv` live in the DEFERRED guardian-icon loop + detail block).

use crate::character_menu;
use crate::font_manager;
use crate::game::Game;
use crate::menu::{self, MenuChild, MenuNode};

/// Java `bm` / `GuardianTab` state — just the `Menu` (`cb`) base (`GuardianTab` adds no
/// fields of its own).
#[derive(Debug, Default, Clone)]
pub struct GuardianTabState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
}

/// `public GuardianTab(Menu parentMenu)` (`bm.<init>:(Lcb;)V => [iadd,i2b]`): the
/// five-slot guardian tab. The active-guardian scan (which would land the cursor on the
/// summoned guardian) is DEFERRED — see the module header — leaving `cursorIndex` at `0`.
pub fn construct(g: &mut Game) {
    // super(parentMenu, (byte) 5);   (parent is the CharacterMenu → non-null → present)
    g.guardian_tab.base = menu::construct(true, 5);
    // Hero hero = GameState.hero();
    // for (byte slot = 0; slot < 5; slot = (byte)(slot+1))
    //   if (hero.guardians[slot] == hero.getActiveGuardian()) { ((Menu) this).cursorIndex = slot; return; }
    // (DEFERRED: Hero.getActiveGuardian (asserts non-null) + Guardian are unported this
    //  lane; guardians/active-guardian are null in this slice (guardian creation DEFERRED
    //  in Hero.initClass), so the scan (the shape's iadd,i2b `slot++`) is skipped.)
}

/// `public final boolean handleKey(int action, int keyCode)` (`bm.a:(II)Z => []`):
/// child forward / non-wrapping vertical nav; FIRE on a filled guardian slot opens the
/// make-active/skill popup. Returns whether consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Guardian, action, key_code)
        || menu::move_cursor_vertical_no_wrap(&mut g.guardian_tab.base, action, key_code)
    {
        return true;
    }
    // Hero hero = GameState.hero();
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    let cursor = g.guardian_tab.base.cursor_index as i32;
    // if ((keyCode != 53 && action != 8) || hero.guardians[((Menu) this).cursorIndex] == null) return false;
    if (key_code != 53 && action != 8)
        || g.entity_arena[hero_id]
            .as_hero()
            .expect("Hero node")
            .guardians[cursor as usize]
            .is_none()
    {
        return false;
    }
    // showPopup((byte) 3, (byte) 2, {CharacterMenu.text.get(22), CharacterMenu.text.get(23)});
    let l22 = character_menu::text_get(g, 22);
    let l23 = character_menu::text_get(g, 23);
    menu::show_popup(g, MenuNode::Guardian, 3, 2, vec![l22, l23]);
    // return false;
    false
}

/// `public final void onPopupResult(byte tag, byte result)` (`bm.a:(BB)V => []`):
/// snapshots the previous child, runs the base dismiss (`super`), then — on a popup
/// answer with `tag == 3` — either makes the guardian active or opens its skill panel.
/// Both commits are DEFERRED (Guardian + `Hero.setActiveGuardian` + `GuardianSkillPanel`
/// unported this lane).
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // Menu previousChild = ((Menu) this).child;
    let previous_child = g.guardian_tab.base.child;
    // super.onPopupResult(tag, result);
    menu::on_popup_result_base(g, MenuNode::Guardian, tag, result);
    // Hero hero = GameState.hero();
    // if ((previousChild instanceof PopupMenu) && tag == 3) { switch (result) { ... } }
    if previous_child == MenuChild::Popup && tag == 3 {
        match result {
            // case 0: make-active + success/fail showMessage.
            0 => {
                // if (!hero.setActiveGuardian(hero.guardians[cursorIndex]))
                //   showMessage({text.get(27), text.get(28), text.get(29)});
                // else showMessage({ StringTable.get(3933) + " " + guardianText.get(guardians[cursorIndex].type) });
                // (DEFERRED: Hero.setActiveGuardian, AssetCache.guardianText, and the Guardian
                //  detail (guardians[].type) are unported this lane; guardians are null here.)
            }
            // case 1: open the guardian's skill panel.
            1 => {
                // ((Menu) this).child = new GuardianSkillPanel(this, hero.guardians[cursorIndex]);
                // (DEFERRED: GuardianSkillPanel + the Guardian detail are unported this lane.)
            }
            _ => {}
        }
    }
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`bm.a:(…Graphics;II)V`): the paginated guardian list. The `drawListPage` grid and
/// the empty-selection `text.get(31)` label are drawn; the per-slot guardian icons, the
/// selected-guardian detail block, and the `BaseCanvas.drawLabelBox` header are DEFERRED
/// (Guardian + those art banks unported; the detail is unreachable this slice — every
/// slot is null).
pub fn paint(g: &mut Game, x: i32, y: i32) {
    // int panelX = x + 2; int panelY = y + 15;
    let panel_x = x.wrapping_add(2);
    let panel_y = y.wrapping_add(15);
    // Hero hero = GameState.hero();
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    let cursor = g.guardian_tab.base.cursor_index as i32;
    // if (hero.guardians[((Menu) this).cursorIndex] == null) { ... drawChars(text.get(31)); return; }
    let selected_guardian_none = g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .guardians[cursor as usize]
        .is_none();
    let text31 = if selected_guardian_none {
        character_menu::text_get(g, 31)
    } else {
        Vec::new()
    };
    let base = g.guardian_tab.base.clone();

    let Game {
        screen,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(30), panelX + 5, panelY);
    // (DEFERRED: BaseCanvas.drawLabelBox is unported.)
    // drawListPage(graphics, panelX, panelY, false);
    menu::draw_list_page(&mut graphics, &base, panel_x, panel_y, false);
    // for (int slot = 0; slot < 5; slot++)
    //   if (hero.guardians[slot] != null) graphics.drawImage(AssetCache.guardianIcons[guardians[slot].type], …);
    // (DEFERRED: AssetCache.guardianIcons + Guardian.type unported — the per-slot icon
    //  loop (the shape's irem/imul `(slot%5)*23`) is skipped.)
    if selected_guardian_none {
        // graphics.setColor(14663551);
        graphics.set_color(14663551);
        // FontManager.drawChars(graphics, panelX + 34, panelY + 18, CharacterMenu.text.get(31), 1);
        font_manager::draw_chars(
            font_manager,
            &mut graphics,
            panel_x.wrapping_add(34),
            panel_y.wrapping_add(18),
            &text31,
            1,
        );
        // return;
    }
    // The guardians[cursorIndex] != null detail block (name/level/exp bar + guardianText +
    // portraitFrame + statLabel art + the exp-bar imul/idiv) is DEFERRED (Guardian + those
    // banks unported). Unreachable this slice: every guardian slot is null.
}
