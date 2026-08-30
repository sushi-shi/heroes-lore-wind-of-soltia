//! Transliterated from `java/src/main/java/defpackage/StartTraitMenu.java`
//! (original `bk.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! New-game starting-guardian picker (`StartTraitMenu extends Menu`), pushed by
//! [`ClassConfirmMenu`](crate::class_confirm_menu) once a class has been chosen.
//! The player toggles exactly two of the three starting guardians
//! ([`guardian_selected`](StartTraitMenuState::guardian_selected)); reaching two
//! selections flips the screen into a yes/no confirmation
//! ([`confirming`](StartTraitMenuState::confirming) /
//! [`confirm_yes`](StartTraitMenuState::confirm_yes)), and confirming calls
//! [`start_game`] to launch a new game with the chosen
//! [`class_id`](StartTraitMenuState::class_id) and guardian mask.
//!
//! ## ANTI-BOG boundary + the newGame seam
//!
//! This increment ports the constructor, `handleKey` (the full guardian-toggle
//! logic incl. the exactly-2-of-3 rule + the `confirming`/`confirmYes` state
//! machine), a **PARTIAL** `paint` (the shared parchment fill + title plate +
//! heading + menu panel only), and `startGame`. In `startGame`, the real
//! `GameState.newGame(false, classId, guardianSelected)` launch
//! (`StartTraitMenu.java:180-182`) is a **DEFERRED SEAM**: a parallel lane owns
//! `GameState`, so instead of calling in, the launch is recorded as
//! [`pending_new_game`](StartTraitMenuState::pending_new_game) for the integrator
//! to wire. The class/guardian art (`AssetCache.menuGuardianPreview` /
//! `AssetCache.commonText` / `AssetCache.guardianText` / `FontManager.drawWrappedText`
//! / `FontManager.labelBack`), the icon-bob animation, and the soft keys are
//! DEFERRED in `paint` (unported statics; the start-trait art is not
//! oracle-captured). The constructor's `showMessage(...)` intro popup and the Back
//! key's `parent.onPopupResult(...)` are DEFERRED (`PopupMenu` not ported), so the
//! flat model's `child` stays `None` and keys reach the toggle logic directly.
//!
//! `StartTraitMenu`'s instance fields (`confirming`, `confirmYes`, `classId`,
//! `guardianSelected`, `bounceUp`, `bounceOffset`) are per-INSTANCE (the Menu base
//! fields likewise), so it contributes no `java/reconstruction/ownership.tsv`
//! static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bk.<init>:(Lby;B)V => []`
//! (constructor — pure stores), `bk.a:(II)Z => [iadd,i2b,iinc]` (handleKey — the
//! `(byte)(selectedCount+1)` accumulate + the `for i` loop counter),
//! `bk.d:()V => []` (startGame — the deferred `GameState.newGame` call has no
//! arithmetic), and `bk.a:(…Graphics;II)V => [iadd,ishr,iadd×…,i2b,…]` (paint — the
//! ported subset uses the `(originX+155)>>1` heading centre + the `originY+…`
//! offsets; the rest live in the DEFERRED guardian art + bob animation).

use crate::font_manager;
use crate::game::Game;
use crate::main_menu;
use crate::menu::{self, MenuNode};
use j2me_jvm::ishr;

/// Java `bk` / `StartTraitMenu` instance state — the `Menu` (`cb`) base fields plus
/// the guardian-selection + confirmation state.
#[derive(Debug, Default, Clone)]
pub struct StartTraitMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private boolean confirming;` — true once two guardians are picked and the
    /// yes/no confirmation is shown.
    pub confirming: bool,
    /// `private boolean confirmYes;` — in the confirmation, whether "start" (vs.
    /// "cancel") is highlighted.
    pub confirm_yes: bool,
    /// `private byte classId;` — chosen character class id, forwarded to the
    /// deferred `GameState.newGame`.
    pub class_id: i8,
    /// `private boolean[] guardianSelected;` — per-guardian selection flags (a
    /// `boolean[3]`; exactly two may be set).
    pub guardian_selected: Vec<bool>,
    /// `private boolean bounceUp;` — bob-animation direction (true while rising).
    pub bounce_up: bool,
    /// `private byte bounceOffset;` — vertical bob offset (0..3) of the highlighted
    /// guardian icon.
    pub bounce_offset: i8,
    /// **DEFERRED SEAM** — not a Java field. Records the `startGame` launch request
    /// `GameState.newGame(false, classId, guardianSelected)` for the integrator to
    /// wire (a parallel lane owns `GameState`). `(false, classId, guardianMask)`
    /// where `guardianMask` has bit `i` set iff `guardianSelected[i]` — the seam's
    /// lossless encoding of the `boolean[3]`. `None` until the confirm "Yes" fires.
    pub pending_new_game: Option<(bool, i8, i8)>,
}

/// `public StartTraitMenu(ClassConfirmMenu parentMenu, byte classId)`
/// (`bk.<init>:(Lby;B)V => []`): `super(parentMenu, (byte) 3); guardianSelected =
/// new boolean[3]; bounceUp = true; bounceOffset = 0; confirming = false; confirmYes
/// = false; classId = classId; showMessage(...);`.
pub fn construct(g: &mut Game, class_id: i8) {
    // super(parentMenu, (byte) 3);   (parent is the ClassConfirmMenu → non-null → present)
    g.start_trait_menu.base = menu::construct(true, 3);
    // this.guardianSelected = new boolean[3];
    g.start_trait_menu.guardian_selected = vec![false; 3];
    // this.bounceUp = true;
    g.start_trait_menu.bounce_up = true;
    // this.bounceOffset = (byte) 0;
    g.start_trait_menu.bounce_offset = 0;
    // this.confirming = false;
    g.start_trait_menu.confirming = false;
    // this.confirmYes = false;
    g.start_trait_menu.confirm_yes = false;
    // this.classId = classId;
    g.start_trait_menu.class_id = class_id;
    // Not a Java field — reset the deferred-launch record on (re)construction.
    g.start_trait_menu.pending_new_game = None;
    // showMessage(new Object[]{AssetCache.commonText.get(16), AssetCache.commonText.get(13)});
    // (DEFERRED: showMessage intro popup — PopupMenu / AssetCache.commonText unported;
    // the flat model leaves child == MenuChild::None, so keys reach the toggle logic.)
}

/// `public final boolean handleKey(int action, int keyCode)`
/// (`bk.a:(II)Z => [iadd,i2b,iinc]`): the guardian-toggle state machine. While not
/// `confirming`: horizontal nav (resetting the bob), and FIRE toggles the current
/// guardian — reaching two selections flips into `confirming`. While `confirming`:
/// LEFT/RIGHT toggle `confirmYes`, FIRE either [`start_game`]s (Yes) or resets the
/// selection (No). Always returns true.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::StartTrait, action, key_code) {
        return true;
    }
    // if (!this.confirming) { ... }
    if !g.start_trait_menu.confirming {
        // if (moveCursorHorizontal(action, keyCode)) { this.bounceOffset = (byte) 0; return true; }
        if menu::move_cursor_horizontal(&mut g.start_trait_menu.base, action, key_code) {
            // this.bounceOffset = (byte) 0;
            g.start_trait_menu.bounce_offset = 0;
            return true;
        }
        // if (keyCode == 53 || action == 8) { ... }
        if key_code == 53 || action == 8 {
            // this.guardianSelected[cursorIndex] = !this.guardianSelected[cursorIndex];
            let idx = g.start_trait_menu.base.cursor_index as usize;
            g.start_trait_menu.guardian_selected[idx] = !g.start_trait_menu.guardian_selected[idx];
            // byte selectedCount = 0;
            let mut selected_count: i8 = 0;
            // for (int i = 0; i < 3; i++) { if (guardianSelected[i]) selectedCount = (byte)(selectedCount+1); }
            let mut i: i32 = 0;
            while i < 3 {
                if g.start_trait_menu.guardian_selected[i as usize] {
                    selected_count = (selected_count as i32).wrapping_add(1) as i8;
                }
                i = i.wrapping_add(1);
            }
            // if (selectedCount == 2) { this.confirming = true; this.confirmYes = false; }
            if (selected_count as i32) == 2 {
                g.start_trait_menu.confirming = true;
                g.start_trait_menu.confirm_yes = false;
            }
        }
        // if (keyCode != -8) return true;
        if key_code != -8 {
            return true;
        }
        // ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
        // (DEFERRED: Menu.onPopupResult — PopupMenu tear-down not ported; Back is not
        // on the wired FIRE path.)
        // return true;
        return true;
    }
    // switch (action) {
    match action {
        // case 2: case 5: this.confirmYes = !this.confirmYes; break;
        2 | 5 => {
            g.start_trait_menu.confirm_yes = !g.start_trait_menu.confirm_yes;
        }
        // case 8: if (confirmYes) startGame(); else { guardianSelected = new boolean[3]; confirming = false; }
        8 => {
            if g.start_trait_menu.confirm_yes {
                start_game(g);
            } else {
                g.start_trait_menu.guardian_selected = vec![false; 3];
                g.start_trait_menu.confirming = false;
            }
        }
        // default:
        _ => match key_code {
            // case -8: guardianSelected = new boolean[3]; confirming = false; break;
            -8 => {
                g.start_trait_menu.guardian_selected = vec![false; 3];
                g.start_trait_menu.confirming = false;
            }
            // case 52: case 54: this.confirmYes = !this.confirmYes; break;
            52 | 54 => {
                g.start_trait_menu.confirm_yes = !g.start_trait_menu.confirm_yes;
            }
            // case 53: if (confirmYes) startGame(); else { guardianSelected = new boolean[3]; confirming = false; }
            53 => {
                if g.start_trait_menu.confirm_yes {
                    start_game(g);
                } else {
                    g.start_trait_menu.guardian_selected = vec![false; 3];
                    g.start_trait_menu.confirming = false;
                }
            }
            _ => {}
        },
    }
    // return true;
    true
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`bk.a:(…Graphics;II)V`): **PARTIAL** — the parchment fill + the shared title
/// plate + the "trait" heading + the menu panel. The guardian previews
/// (`AssetCache.menuGuardianPreview`), the guardian name/description
/// (`AssetCache.guardianText`), the confirm Yes/No labels (`AssetCache.commonText` /
/// `AssetCache.menuFrames`), the icon-bob animation, and the soft keys
/// (`FontManager.labelBack`) are DEFERRED (those statics are unported and the
/// start-trait art is not oracle-captured).
pub fn paint(g: &mut Game, origin_x: i32, origin_y: i32) {
    let Game {
        screen,
        asset_cache,
        base_canvas,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // graphics.setColor(4136767);
    graphics.set_color(4136767);
    // graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.fill_rect(0, 0, base_canvas.width, base_canvas.height);
    // MainMenu.drawTitlePlate(graphics, originX, originY);
    main_menu::draw_title_plate(asset_cache, &mut graphics, origin_x, origin_y);
    // FontManager.drawMenuItem(graphics, 1, (originX + 155) >> 1, originY + 5);
    font_manager::draw_menu_item(
        font_manager,
        &mut graphics,
        base_canvas,
        1,
        ishr(origin_x.wrapping_add(155), 1),
        origin_y.wrapping_add(5),
    );
    // MainMenu.drawMenuPanel(graphics, originX, originY + 24, 3);
    main_menu::draw_menu_panel(
        asset_cache,
        &mut graphics,
        origin_x,
        origin_y.wrapping_add(24),
        3,
    );

    // (DEFERRED — the guardian previews, the guardian name/description, the confirm
    // Yes/No labels, the icon-bob animation, and the soft keys cross into
    // as-yet-unported statics. Faithful full form:
    //   int panelX = originX + 15; int panelY = originY + 10;
    //   graphics.drawImage(AssetCache.menuFrames[19], panelX+11, panelY+82, 20);
    //   for (byte g = 0; g < 3; g++) {
    //     if (guardianSelected[g]) drawImage(menuGuardianPreview[g][1], panelX+22+(g*34), (panelY+66)-5, 3);
    //     else drawImage(menuGuardianPreview[g][0], panelX+22+(g*34),
    //                    ((panelY+59)-5) + (cursorIndex == g ? bounceOffset : 0), 3);
    //   }
    //   if (!confirming) drawImage(menuFrames[20], panelX+19+(cursorIndex*34), panelY+73, 20);
    //   setColor(0);
    //   if (confirming) { ...menuFrames[17] selector + commonText 17/14/15 Yes/No labels... }
    //   else { drawChars(guardianText.get(cursorIndex)); drawWrappedText(guardianText.get(12+cursorIndex)); }
    //   // icon-bob triangle-wave over bounceOffset/bounceUp (0..3)
    //   if (bounceOffset == 0) { bounceOffset++; bounceUp = true; }
    //   else if (bounceOffset == 3) { bounceOffset--; bounceUp = false; }
    //   else if (bounceUp) bounceOffset++; else bounceOffset--;
    //   if (child == null) needsRepaint = true;
    //   FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    // AssetCache.menuGuardianPreview / commonText / guardianText / FontManager.labelBack are unported.)
}

/// `private void startGame()` (`bk.d:()V => []`): launches a new game with the chosen
/// class and guardian selection mask.
///
/// **DEFERRED SEAM: `GameState.newGame(false, this.classId, this.guardianSelected)`
/// — wired at integration.** A parallel lane owns `GameState`; instead of calling
/// in, the launch request is recorded on
/// [`pending_new_game`](StartTraitMenuState::pending_new_game) so the build stays
/// runnable and the integrator can wire the real launch. The `boolean[3]`
/// `guardianSelected` is packed losslessly into a 3-bit mask (bit `i` set iff
/// guardian `i` is chosen) — the seam's own encoding, NOT a transliterated Java op.
fn start_game(g: &mut Game) {
    // GameState.newGame(false, this.classId, this.guardianSelected);
    // DEFERRED SEAM: GameState.newGame(...) — wired at integration.
    let class_id = g.start_trait_menu.class_id;
    let mut guardian_mask: i8 = 0;
    for i in 0..3usize {
        if g.start_trait_menu.guardian_selected[i] {
            guardian_mask |= 1i8 << i;
        }
    }
    g.start_trait_menu.pending_new_game = Some((false, class_id, guardian_mask));
}
