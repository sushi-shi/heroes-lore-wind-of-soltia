//! Transliterated from `java/src/main/java/defpackage/ClassConfirmMenu.java`
//! (original `by.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Confirmation screen for the class chosen in [`ClassSelectMenu`](crate::class_select_menu)
//! (`ClassConfirmMenu extends Menu`). It shows the class name/description and
//! portrait with a Yes/No selector; choosing "Yes" (`cursorIndex == 0`) pushes
//! [`StartTraitMenu`](crate::start_trait_menu) to begin a new game as
//! [`class_id`](ClassConfirmMenuState::class_id), "No"/Back returns to the class
//! list. Reached from [`class_select_menu::handle_key`](crate::class_select_menu)
//! FIRE.
//!
//! ## ANTI-BOG boundary
//!
//! This increment ports the constructor, `handleKey` (the `child = new
//! StartTraitMenu(this, classId)` transition made **real** — the flat model
//! materialises the [`StartTraitMenu`](crate::start_trait_menu) child state and
//! links it via the [`MenuChild`] discriminant), and a **PARTIAL** `paint` (the
//! shared parchment fill + title plate + heading + menu panel only — the class
//! name/description text, the portrait, the Yes/No labels, and the soft keys cross
//! into as-yet-unported statics (`AssetCache.heroText` / `AssetCache.classFaces` /
//! `AssetCache.commonText` / `FontManager.drawWrappedText` / `FontManager.labelBack`)
//! and the class-confirm art is not oracle-captured, so they are DEFERRED). The
//! `parent.close()` back transitions are DEFERRED too (`Menu.close` not ported;
//! Back is not on the wired FIRE path).
//!
//! `ClassConfirmMenu`'s one instance field `classId` (`c`) is per-INSTANCE (the
//! Menu base fields likewise), so it contributes no
//! `java/reconstruction/ownership.tsv` static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `by.<init>:(Lc;B)V => []`
//! (constructor — pure stores), `by.a:(II)Z => []` (handleKey — pure branches), and
//! `by.a:(…Graphics;II)V => [iadd×24]` (paint — the ported subset uses the `x+77`,
//! `y+5`, `y+24` iadds; the remaining iadds live in the DEFERRED text/portrait art).

use crate::font_manager;
use crate::game::Game;
use crate::main_menu;
use crate::menu::{self, MenuChild, MenuNode};
use crate::start_trait_menu;

/// Java `by` / `ClassConfirmMenu` instance state — the `Menu` (`cb`) base fields
/// plus the chosen `classId`.
#[derive(Debug, Default, Clone)]
pub struct ClassConfirmMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private byte classId;` — selected starting class id
    /// (`6 + (2 - cursorIndex)` from the class list), forwarded to `StartTraitMenu`.
    pub class_id: i8,
}

/// `public ClassConfirmMenu(ClassSelectMenu parent, byte classId)`
/// (`by.<init>:(Lc;B)V => []`): `super(parent, (byte) 2); cursorIndex = (byte) 1;
/// classId = classId;`.
pub fn construct(g: &mut Game, class_id: i8) {
    // super(parent, (byte) 2);   (parent is the ClassSelectMenu → non-null → present)
    g.class_confirm_menu.base = menu::construct(true, 2);
    // ((Menu) this).cursorIndex = (byte) 1;
    g.class_confirm_menu.base.cursor_index = 1;
    // this.classId = classId;
    g.class_confirm_menu.class_id = class_id;
}

/// `public final boolean handleKey(int action, int keyCode)` (`by.a:(II)Z => []`):
/// horizontal Yes/No navigation, then FIRE → confirm. "Yes" (`cursorIndex == 0`)
/// pushes `StartTraitMenu(this, classId)` — the transition is **real** (the child
/// state is materialised, then linked via the [`MenuChild`] discriminant); "No"
/// (`cursorIndex != 0`) / Back does `parent.close()` (DEFERRED — `Menu.close` not
/// ported; Back is not on the wired FIRE path). Always returns true.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorHorizontal(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::ClassConfirm, action, key_code)
        || menu::move_cursor_horizontal(&mut g.class_confirm_menu.base, action, key_code)
    {
        return true;
    }
    // switch (action) {
    match action {
        // case 8:
        8 => {
            // if (cursorIndex != 0) { parent.close(); } else { child = new StartTraitMenu(this, classId); }
            if (g.class_confirm_menu.base.cursor_index as i32) != 0 {
                // ((Menu) this).parent.close();
                // (DEFERRED: Menu.close — drops the child + invalidateUp; not ported.
                // Back is not on the wired FIRE path.)
            } else {
                push_start_trait(g);
            }
        }
        // default:
        _ => match key_code {
            // case -8: parent.close();
            -8 => {
                // (DEFERRED: Menu.close — see above.)
            }
            // case 53:
            53 => {
                // if (cursorIndex != 0) { parent.close(); } else { child = new StartTraitMenu(this, classId); }
                if (g.class_confirm_menu.base.cursor_index as i32) != 0 {
                    // (DEFERRED: Menu.close — see above.)
                } else {
                    push_start_trait(g);
                }
            }
            _ => {}
        },
    }
    // return true;
    true
}

/// `((Menu) this).child = new StartTraitMenu(this, this.classId);` — materialise the
/// child `StartTraitMenu` state (with the chosen `classId`), then link it via the
/// [`MenuChild`] discriminant. The flat model of Java's `child = new …` assignment.
fn push_start_trait(g: &mut Game) {
    // new StartTraitMenu(this, this.classId)
    let class_id = g.class_confirm_menu.class_id;
    start_trait_menu::construct(g, class_id);
    // ((Menu) this).child = <the StartTraitMenu>;
    g.class_confirm_menu.base.child = MenuChild::StartTrait;
}

/// `public final void paint(Graphics graphics, int x, int y)` (`by.a:(…Graphics;II)V`):
/// **PARTIAL** — the parchment fill, the shared title plate, the "confirm" heading,
/// and the menu panel. The class name/description (`AssetCache.heroText`), the
/// portrait (`AssetCache.classFaces` / `AssetCache.menuFrames[19]`), the Yes/No labels
/// (`AssetCache.commonText` / `AssetCache.menuFrames[17]`), and the soft keys
/// (`FontManager.labelBack`) are DEFERRED (those statics are unported and the
/// class-confirm art is not oracle-captured).
pub fn paint(g: &mut Game, x: i32, y: i32) {
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
    // MainMenu.drawTitlePlate(graphics, x, y);
    main_menu::draw_title_plate(asset_cache, &mut graphics, x, y);
    // FontManager.drawMenuItem(graphics, 1, x + 77, y + 5);
    font_manager::draw_menu_item(
        font_manager,
        &mut graphics,
        base_canvas,
        1,
        x.wrapping_add(77),
        y.wrapping_add(5),
    );
    // MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
    main_menu::draw_menu_panel(asset_cache, &mut graphics, x, y.wrapping_add(24), 3);

    // (DEFERRED — the class name/description text, the class portrait, the Yes/No
    // labels, and the soft keys cross into as-yet-unported statics. Faithful full form:
    //   int baseY = y + 5; int baseX = x + 10;
    //   graphics.setColor(0);
    //   FontManager.drawWrappedText(graphics, baseX+11, baseY+34, 133, 1,
    //       AssetCache.heroText.get((15 + this.classId) - 6));
    //   graphics.drawImage(AssetCache.menuFrames[19], baseX+7, baseY+80, 20);
    //   FontManager.drawChars(graphics, baseX+11, baseY+84, AssetCache.heroText.get(this.classId - 6), 1);
    //   graphics.drawImage(AssetCache.classFaces[this.classId - 6], baseX+125, baseY+137, 40);
    //   graphics.drawImage(AssetCache.menuFrames[17], baseX+5 + (cursorIndex == 0 ? 0 : 28), baseY+118, 20);
    //   setColor(cursorIndex == 0 ? 16777215 : 0);
    //   FontManager.drawChars(graphics, baseX+9, baseY+121, AssetCache.commonText.get(14), 1);
    //   setColor(cursorIndex == 1 ? 16777215 : 0);
    //   FontManager.drawChars(graphics, baseX+37, baseY+121, AssetCache.commonText.get(15), 1);
    //   FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    // AssetCache.heroText / classFaces / commonText / FontManager.labelBack are unported.)
}
