//! Transliterated from `java/src/main/java/defpackage/SystemTab.java`
//! (original `d.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! System tab (tab 5) of [`CharacterMenu`](crate::character_menu): Save / Help /
//! Options / Quit. Save runs a tiny two-frame state machine in [`paint`] via
//! [`saveState`](SystemTabState::save_state) (draws a "saving" box, then performs
//! [`GameState.saveGame`](crate::game_state) and reports the result); Help and Options
//! push their menus; Save (in the demo build) and Quit route through [`prompt_exit`],
//! which shows the buy-full prompt in the demo or the quit-confirm otherwise, handled
//! in [`on_popup_result`].
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. `<init>`/`handleKey` are fully real over modelled state —
//! `AppConfig.fullVersion`, `GameState.map.bossMap`, the save-state gate, the ported
//! `moveCursorVertical`, the Options push (`new OptionsMenu(this, true)` — ported), and
//! `promptExit`; only the Help push (`new HelpMenu(this, true)`) is DEFERRED (HelpMenu
//! unported). `onPopupResult` is real (`GameState.requestState`, `AudioManager.stopBgm`,
//! `MainMenu.pendingExitPrompt`); only the full-version buy hop
//! (`FontManager.requestBuyAndExit`) is DEFERRED. `promptExit` pushes the real popups —
//! its `FontManager.getString(3919)` line + `labelExit` land; `FontManager.labelBuy`
//! (unmodeled) is passed `None` and the demo `FontManager.confirmPrompt` line
//! (unmodeled) is empty. `paint` runs the full save state machine
//! (`saveState` transitions + `GameState.saveGame` + the result `showMessage`) and draws
//! the "saving" box (`drawPanelFrame`/`fillPanelInterior` + `drawChars(text.get(53))`);
//! the `Menu.drawButton` row + the `BaseCanvas.drawLabelBox` header are DEFERRED (those
//! button/label widgets unported).
//!
//! `SystemTab`'s only field (`saveState`) is per-INSTANCE (no `static`s), so it
//! contributes no `java/reconstruction/ownership.tsv` rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `d.<init>:(Lcb;)V => []`,
//! `d.a:(II)Z => []` (handleKey — pure branches/switch), `d.a:(BB)V => []`
//! (onPopupResult — pure branches), `d.d:()V => []` (promptExit — pure branches),
//! `d.a:(…Graphics;II)V => [iinc,iadd,isub,ishr,iadd,iadd,iadd,iadd,isub,isub,iadd,iadd]`
//! (paint — the ported tail is the "saving" box `boxX=halfW-55`/`boxY=halfH-11` (isub,
//! isub) + `boxX+5`/`boxY+5` (iadd,iadd); the leading iadd (`panelY=y+15`), the
//! `buttonX=(width-108)>>1` (isub,ishr) and the drawButton panelY offsets live in the
//! DEFERRED label/button draws).

use crate::audio_manager;
use crate::character_menu;
use crate::font_manager;
use crate::game::Game;
use crate::game_state;
use crate::menu::{self, MenuChild, MenuNode};
use crate::options_menu;

/// Java `d` / `SystemTab` state — the `Menu` (`cb`) base + the save state machine field.
#[derive(Debug, Default, Clone)]
pub struct SystemTabState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private byte saveState;` (obf `c`) — save state machine: 0 = idle, 2 = save
    /// requested (show box), 1 = perform save.
    pub save_state: i8,
}

/// `public SystemTab(Menu parentMenu)` (`d.<init>:(Lcb;)V => []`): the four-button
/// system tab (Save / Help / Options / Quit).
pub fn construct(g: &mut Game) {
    // super(parentMenu, (byte) 4);   (parent is the CharacterMenu → non-null → present)
    g.system_tab.base = menu::construct(true, 4);
    // this.saveState = (byte) 0;
    g.system_tab.save_state = 0;
}

/// `public final synchronized boolean handleKey(int action, int keyCode)`
/// (`d.a:(II)Z => []`): child forward / save-in-progress / vertical nav guard, then the
/// four-button FIRE dispatch. The `synchronized` monitor is a no-op in the
/// single-threaded flat model. Returns whether consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || this.saveState != 0 || moveCursorVertical(action, keyCode, false)) return true;
    if menu::pass_key_to_child(g, MenuNode::System, action, key_code)
        || g.system_tab.save_state != 0
        || menu::move_cursor_vertical(&mut g.system_tab.base, action, key_code, false)
    {
        return true;
    }
    // if (keyCode != 53 && action != 8) return false;
    if key_code != 53 && action != 8 {
        return false;
    }
    // switch (((Menu) this).cursorIndex) { ... }
    match g.system_tab.base.cursor_index as i32 {
        // case 0: Save.
        0 => {
            // if (AppConfig.fullVersion) { promptExit(); return true; }
            if g.app_config.full_version {
                prompt_exit(g);
                return true;
            }
            // if (GameState.map.bossMap) { showMessage({text.get(51), text.get(52)}); return true; }
            let boss_map = g
                .game_state
                .map
                .as_ref()
                .expect("NullPointerException: GameState.map")
                .boss_map;
            if boss_map {
                let l51 = character_menu::text_get(g, 51);
                let l52 = character_menu::text_get(g, 52);
                menu::show_message(g, MenuNode::System, vec![l51, l52]);
                return true;
            }
            // this.saveState = (byte) 2; invalidateUp(); return true;
            g.system_tab.save_state = 2;
            menu::invalidate_up(g, MenuNode::System);
            true
        }
        // case 1: Help.
        1 => {
            // ((Menu) this).child = new HelpMenu(this, true); return true;
            // (DEFERRED: HelpMenu unported this lane — the child construction is deferred.)
            true
        }
        // case 2: Options.
        2 => {
            // ((Menu) this).child = new OptionsMenu(this, true); return true;
            options_menu::construct(g, true);
            g.system_tab.base.child = MenuChild::Options;
            true
        }
        // case 3: Quit.
        3 => {
            // promptExit(); return true;
            prompt_exit(g);
            true
        }
        // default: return true;
        _ => true,
    }
}

/// `public final void promptExit()` (`d.d:()V => []`): shows the buy-full prompt (demo
/// build) or the quit-to-menu confirm. `FontManager.labelBuy` (unmodeled) is passed
/// `None`; the demo `FontManager.confirmPrompt` line (unmodeled) is empty.
pub fn prompt_exit(g: &mut Game) {
    // if (AppConfig.fullVersion) {
    if g.app_config.full_version {
        // showPopup((byte) 12, (byte) 2, {FontManager.getString(3919).toCharArray()}, FontManager.labelBuy, FontManager.labelExit);
        let line = font_manager::get_string(g, 3919);
        // (DEFERRED: FontManager.labelBuy is an unmodeled FontManager field → passed None.)
        let label_exit = g.font_manager.label_exit.clone();
        menu::show_popup_labels(g, MenuNode::System, 12, 2, vec![line], None, label_exit);
    } else {
        // showPopup((byte) 2, (byte) 2, {FontManager.confirmPrompt});
        // (DEFERRED: FontManager.confirmPrompt is an unmodeled FontManager field → empty line.)
        menu::show_popup(g, MenuNode::System, 2, 2, vec![Vec::new()]);
    }
}

/// `public final void onPopupResult(byte tag, byte result)` (`d.a:(BB)V => []`): runs
/// the base dismiss (`super`), then handles the buy/quit prompt result. Only the
/// full-version buy hop (`FontManager.requestBuyAndExit`) is DEFERRED.
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // super.onPopupResult(tag, result);
    menu::on_popup_result_base(g, MenuNode::System, tag, result);
    // if (tag == 12 || tag == 2) {
    if tag == 12 || tag == 2 {
        // if (!AppConfig.fullVersion) {
        if !g.app_config.full_version {
            // if (result == 0) { GameState.requestState((byte)14, (byte)1); AudioManager.stopBgm(); return; }
            if result == 0 {
                game_state::request_state_a0(g, 14, 1);
                audio_manager::stop_bgm(g);
                return;
            }
            // return;
            return;
        }
        // if (result == 0) { FontManager.requestBuyAndExit(AppConfig.buyUrl); return; }
        if result == 0 {
            // (DEFERRED: FontManager.requestBuyAndExit(AppConfig.buyUrl) — requestBuyAndExit
            //  unported this lane.)
            return;
        }
        // GameState.requestState((byte)14, (byte)1); AudioManager.stopBgm(); MainMenu.pendingExitPrompt = true;
        game_state::request_state_a0(g, 14, 1);
        audio_manager::stop_bgm(g);
        g.main_menu.pending_exit_prompt = true;
    }
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`d.a:(…Graphics;II)V`): the four buttons + the save state machine. The state machine
/// (`saveState` transitions + `GameState.saveGame` + the result `showMessage`) and the
/// "saving" box (`drawPanelFrame`/`fillPanelInterior` + `drawChars(text.get(53))`) are
/// real; the `Menu.drawButton` row + the `BaseCanvas.drawLabelBox` header are DEFERRED.
pub fn paint(g: &mut Game, _x: i32, _y: i32) {
    // int panelY = y + 15;
    // BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(41), x + 5, panelY);
    // int buttonX = (BaseCanvas.width - 108) >> 1;
    // Menu.drawButton(graphics, buttonX, panelY + {15,37,59,81}, 108, text.get({42..45}), cursorIndex == {0..3});
    // (DEFERRED: BaseCanvas.drawLabelBox + the four Menu.drawButton calls — those label /
    //  button widgets are unported. The shape's leading iadd (panelY), the isub,ishr
    //  (buttonX = (width-108)>>1) and the panelY button offsets live here.)
    let save_state = g.system_tab.save_state;
    // if (this.saveState != 2) {
    if save_state != 2 {
        // if (this.saveState == 1) {
        if save_state == 1 {
            // this.saveState = (byte) 0;
            g.system_tab.save_state = 0;
            // try { GameState.saveGame(); showMessage({text.get(46)}); }
            // catch (Exception unused) { showMessage({text.get(47), text.get(48)}); }
            match game_state::save_game(g) {
                Ok(()) => {
                    let l46 = character_menu::text_get(g, 46);
                    menu::show_message(g, MenuNode::System, vec![l46]);
                }
                Err(_) => {
                    let l47 = character_menu::text_get(g, 47);
                    let l48 = character_menu::text_get(g, 48);
                    menu::show_message(g, MenuNode::System, vec![l47, l48]);
                }
            }
            // return;
            return;
        }
        // return;
        return;
    }
    // this.saveState = (byte) 1;
    g.system_tab.save_state = 1;
    // int boxX = BaseCanvas.halfW - 55; int boxY = BaseCanvas.halfH - 11;
    let box_x = g.base_canvas.half_w.wrapping_sub(55);
    let box_y = g.base_canvas.half_h.wrapping_sub(11);
    let text53 = character_menu::text_get(g, 53);
    {
        let Game {
            screen,
            font_manager,
            ..
        } = &mut *g;
        let target = screen.as_mut().expect("framebuffer");
        let mut graphics = j2me_me::Graphics::new(target);
        // Menu.drawPanelFrame(graphics, boxX, boxY, 110, 22);
        menu::draw_panel_frame(&mut graphics, box_x, box_y, 110, 22);
        // Menu.fillPanelInterior(graphics, boxX, boxY, 110, 22);
        menu::fill_panel_interior(&mut graphics, box_x, box_y, 110, 22);
        // graphics.setColor(16777215);
        graphics.set_color(16777215);
        // FontManager.drawChars(graphics, boxX + 5, boxY + 5, CharacterMenu.text.get(53), 1);
        font_manager::draw_chars(
            font_manager,
            &mut graphics,
            box_x.wrapping_add(5),
            box_y.wrapping_add(5),
            &text53,
            1,
        );
    }
    // invalidateUp();
    menu::invalidate_up(g, MenuNode::System);
}
