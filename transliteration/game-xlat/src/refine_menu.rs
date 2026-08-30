//! Transliterated from `java/src/main/java/defpackage/RefineMenu.java`
//! (original `ax.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The item-refinery hub screen (world screen 7), `RefineMenu extends Menu`. Reached
//! from the world via event op 11/1 (`GameState.requestState`, which does
//! `setScreen(7)` then [`open`]). A lazily-created singleton that shows a two-choice
//! popup — enchant or combine — over the `/sgui/refi` label table
//! ([`text`](RefineMenuState::text)); [`on_popup_result`] pushes the chosen child
//! ([`EnchantMenu`](crate::enchant_menu) or [`CombineMenu`](crate::combine_menu)).
//! Both children read their strings from this class's shared [`text`](text_get) table.
//! The screen paints itself at the centered panel origin
//! ([`panel_x`](RefineMenuState::panel_x)/[`panel_y`](RefineMenuState::panel_y)) via
//! [`draw`].
//!
//! ## Statics + singleton
//!
//! `RefineMenu` is a lazily-created singleton (like `ShopMenu`/`CharacterMenu`):
//! [`instance`] creates it on first use and centres the panel. Its `static` fields —
//! `panelX`/`panelY` (the centred origin), `text` (the `/sgui/refi` label table) and
//! `singleton` (the presence flag) — live on [`RefineMenuState`] (`Game.refine_menu`),
//! the sole owner per `java/reconstruction/ownership.tsv`.
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. `instance`/`<init>`/`open`/`closeRefine`/`handleKey`/the
//! overriding `onPopupResult`/`draw` are real (the popup dispatch threads the flat
//! [`menu`](crate::menu) child stack, and `closeRefine` returns to world screen 2 via
//! the ported `GameState.setScreen`). Only two genuinely-unported hops are DEFERRED:
//! `GameLoop.gameScreen.markRedraw()` in `closeRefine` (`GameScreen` not this lane) and
//! the `FontManager.labelBack` soft-key label (unmodeled in the partial `FontManager`
//! → passed `None`, as `character_menu` does). The `paint` clear + soft keys are drawn.
//! **DEFERRED: wire into GameScreen case 7 + GameState** — the world's screen-7
//! dispatch to [`draw`]/[`handle_key`] and the `requestState` op-11/1 case that calls
//! `setScreen(7)` + `open` live in the (unported) `GameScreen`/`GameState` and are wired
//! in the game-state lane.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `ax.a:()Lax;` (instance —
//! the two `halfW - 77` / `halfH - 85` `isub`s), `ax.d:()V => []` (open),
//! `ax.e:()V => []` (closeRefine), `ax.a:(II)Z => []` (handleKey — pure branches),
//! `ax.a:(BB)V => []` (onPopupResult), `ax.a:(…Graphics;)V => []` (draw),
//! `ax.a:(…Graphics;II)V => []` (paint — no arithmetic in the ported clear + soft keys).

use crate::combine_menu;
use crate::enchant_menu;
use crate::font_manager;
use crate::game::Game;
use crate::game_state;
use crate::menu::{self, MenuChild, MenuNode};
use crate::text_table::{self, TextTableState};

/// Java `ax` / `RefineMenu` state — the `Menu` base + the class's four `static` fields
/// (`panelX`, `panelY`, `text`, `singleton`).
#[derive(Debug, Default, Clone)]
pub struct RefineMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `public static int panelX;` (obf `a`) — centred panel origin X.
    pub panel_x: i32,
    /// `public static int panelY;` (obf `b`) — centred panel origin Y.
    pub panel_y: i32,
    /// `public static TextTable text;` (obf `a`, `Lz;`) — the `/sgui/refi` label table
    /// shared with the enchant/combine children (`None` == Java null, until [`open`]).
    pub text: Option<TextTableState>,
    /// `private static RefineMenu singleton;` (obf `a`, `Lax;`) — presence flag.
    pub singleton: bool,
}

/// `RefineMenu.text.get(index)` — resolves a `/sgui/refi` label through the loaded
/// [`StringTable`](crate::string_table). A null `text` (before [`open`]) panics,
/// matching the Java NPE. Shared with the enchant/combine children.
pub(crate) fn text_get(g: &Game, index: i32) -> Vec<u16> {
    // RefineMenu.text.get(index)
    text_table::get(
        g,
        g.refine_menu
            .text
            .as_ref()
            .expect("NullPointerException: RefineMenu.text (open not called)"),
        index,
    )
}

/// `public static final RefineMenu instance()` (`ax.a:()Lax;`): returns (creating on
/// first use) the refinery singleton, centring the panel at `halfW - 77` / `halfH - 85`.
pub fn instance(g: &mut Game) {
    // if (singleton == null) {
    if !g.refine_menu.singleton {
        // singleton = new RefineMenu();
        construct(g);
        g.refine_menu.singleton = true;
        // panelX = BaseCanvas.halfW - 77;
        g.refine_menu.panel_x = g.base_canvas.half_w.wrapping_sub(77);
        // panelY = BaseCanvas.halfH - 85;
        g.refine_menu.panel_y = g.base_canvas.half_h.wrapping_sub(85);
    }
    // return singleton;   (the refine node — identity, not returned in the flat model)
}

/// `public RefineMenu()` (`ax.<init>:()V`): `super(null, (byte) 0)` — a root (null parent).
pub fn construct(g: &mut Game) {
    // super(null, (byte) 0);   (null parent → the refinery is a root)
    g.refine_menu.base = menu::construct(false, 0);
}

/// `public final void open()` (`ax.d:()V => []`): loads the `/sgui/refi` label table
/// and shows the enchant/combine choice popup (style 8 selectable list, itemCount 2).
pub fn open(g: &mut Game) {
    // try { text = new TextTable("/sgui/refi"); } catch (IOException e) { e.printStackTrace(); }
    let table = text_table::construct(g, "/sgui/refi");
    g.refine_menu.text = Some(table);
    // showPopup((byte) 8, (byte) 2, new Object[]{text.get(0), text.get(1), text.get(2)});
    let l0 = text_get(g, 0);
    let l1 = text_get(g, 1);
    let l2 = text_get(g, 2);
    menu::show_popup(g, MenuNode::Refine, 8, 2, vec![l0, l1, l2]);
}

/// `public final void closeRefine()` (`ax.e:()V => []`): tears the refinery down and
/// returns to the world screen.
pub fn close_refine(g: &mut Game) {
    // singleton = null;
    g.refine_menu.singleton = false;
    // text = null;
    g.refine_menu.text = None;
    // ((Menu) this).child = null;
    g.refine_menu.base.child = MenuChild::None;
    // GameState.setScreen(2);
    game_state::set_screen(g, 2);
    // GameLoop.gameScreen.markRedraw();
    // (DEFERRED: GameScreen.markRedraw is unported — game_screen not this lane.)
    // System.gc();   — no-op.
}

/// `public final boolean handleKey(int action, int keyCode)` (`ax.a:(II)Z => []`):
/// child forward; Back (`-8`) closes the refinery. Returns whether consumed (`false`
/// even when closing, exactly as the Java — the close falls through to `return false`).
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Refine, action, key_code) {
        return true;
    }
    // if (keyCode != -8) return false;
    if key_code != -8 {
        return false;
    }
    // closeRefine(); return false;
    close_refine(g);
    false
}

/// `public final void onPopupResult(byte tag, byte result)` (`ax.a:(BB)V => []`): runs
/// the base dismiss (`super`), then pushes the enchant (result 0) or combine (result 1)
/// child for the choice popup (tag 8), else closes the refinery.
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // super.onPopupResult(tag, result);
    menu::on_popup_result_base(g, MenuNode::Refine, tag, result);
    // if (tag == 8 && result == 0) { ((Menu) this).child = new EnchantMenu(this); }
    if tag == 8 && result == 0 {
        enchant_menu::construct(g);
        g.refine_menu.base.child = MenuChild::Enchant;
    // else if (tag == 8 && result == 1) { ((Menu) this).child = new CombineMenu(this); }
    } else if tag == 8 && result == 1 {
        combine_menu::construct(g);
        g.refine_menu.base.child = MenuChild::Combine;
    // else closeRefine();
    } else {
        close_refine(g);
    }
}

/// `public final void draw(Graphics graphics)` (`ax.a:(…Graphics;)V => []`): draws the
/// whole refinery screen tree at the centered panel origin.
pub fn draw(g: &mut Game) {
    // render(graphics, panelX, panelY);
    let (panel_x, panel_y) = (g.refine_menu.panel_x, g.refine_menu.panel_y);
    menu::render_at(g, MenuNode::Refine, panel_x, panel_y);
}

/// `public final void paint(Graphics graphics, int x, int y)` (`ax.a:(…Graphics;II)V =>
/// []`): clears the screen and draws the OK/Back soft keys (`labelBack` unmodeled in
/// the partial `FontManager` → passed `None`, as `character_menu` does).
pub fn paint(g: &mut Game, _x: i32, _y: i32) {
    // FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    let label_ok = g.font_manager.label_ok.clone();
    let Game {
        screen,
        font_manager,
        base_canvas,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // FontManager.clearScreen(graphics);
    font_manager::clear_screen(&mut graphics, base_canvas);
    // FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    // (labelBack is unported → passed as None.)
    font_manager::draw_soft_keys(
        font_manager,
        &mut graphics,
        base_canvas,
        label_ok.as_deref(),
        None,
    );
}
