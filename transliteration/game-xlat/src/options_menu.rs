//! Transliterated from `java/src/main/java/defpackage/OptionsMenu.java`
//! (original `be.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Options screen (`OptionsMenu extends Menu`) — volume, difficulty,
//! auto-advance-text and camera-follow rows. Reached both from the in-game
//! `SystemTab` ([`in_game`](OptionsMenuState::in_game)` == true`, drawn as an inset
//! panel) and from the title [`MainMenu`](crate::main_menu) (`in_game == false`,
//! full screen). Left/right on a row mutates that setting on the shared
//! [`GameLoop`](crate::game_loop) singleton; Back persists the settings
//! (`GameLoop.saveOptions`) and closes.
//!
//! ## ANTI-BOG boundary
//!
//! This increment ports the constructor and `handleKey` fully. `handleKey`'s option
//! mutations drive the ported [`game_loop`](crate::game_loop) fields
//! (`volume`/`difficulty`/`autoTextAdvance`/`cameraFollow`) and the ported
//! [`audio_manager`](crate::audio_manager) (`setVolume`/`stopSfx`); Back's
//! `parent.close()` is made **real** via [`menu::close`], and the camera snap
//! writes the ported `GameState.cam*` fields. Two cross-class calls are DEFERRED:
//! `GameLoop.setDifficulty` (unported in `game_loop.rs`) and `GameLoop.saveOptions`
//! (unported RMS write, guarded by a swallowing `catch`). The `paint` is
//! **PARTIAL** (the branch's structural frame only); the option-row labels/values
//! and the soft keys cross into unported statics (`AssetCache.commonText`,
//! `StringTable`-backed value glyphs, `AssetCache.slotFrame`/`cursorArrow`,
//! `FontManager.blankLabel`/`labelBack`) and are DEFERRED.
//!
//! `OptionsMenu`'s fields are all per-INSTANCE (the `Menu` base fields likewise) —
//! `gameLoop` aliases the `GameLoop` singleton (`Game.game_loop`), modelled as a
//! presence flag — so it contributes no `java/reconstruction/ownership.tsv` static
//! rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `be.<init>:(Lcb;Z)V => []` (constructor — pure stores),
//! `be.a:(II)Z => [isub,i2b,iadd,i2b]` (handleKey — the two `difficulty ∓ 1`
//! steps), and `be.a:(…Graphics;II)V => [iinc,iinc,…]` (paint — the ported prefix;
//! the rest is DEFERRED).

use crate::audio_manager;
use crate::font_manager;
use crate::game::Game;
use crate::main_menu;
use crate::menu::{self, MenuNode};
use j2me_jvm::ishr;

/// Java `be` / `OptionsMenu` instance state — the `Menu` (`cb`) base fields plus the
/// screen's own instance fields.
#[derive(Debug, Default, Clone)]
pub struct OptionsMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private boolean inGame;` (obf `c`) — inset panel (in-game) vs full screen
    /// (title).
    pub in_game: bool,
    /// `private GameLoop gameLoop;` (obf `a`) — a reference to `GameLoop.instance`;
    /// a presence flag (the singleton itself is `Game.game_loop`).
    pub game_loop: bool,
}

/// `public OptionsMenu(Menu parent, boolean inGame)` (`be.<init>:(Lcb;Z)V => []`):
/// `super(parent, (byte) 4); inGame = inGame; gameLoop = GameLoop.instance;`.
pub fn construct(g: &mut Game, in_game: bool) {
    // super(parent, (byte) 4);   (parent is the calling menu → non-null → present)
    g.options_menu.base = menu::construct(true, 4);
    // this.inGame = inGame;
    g.options_menu.in_game = in_game;
    // this.gameLoop = GameLoop.instance;   (the singleton is Game.game_loop)
    g.options_menu.game_loop = g.game_loop.instance;
}

/// `public final boolean handleKey(int action, int keyCode)`
/// (`be.a:(II)Z => [isub,i2b,iadd,i2b]`): forwards to any child / vertical no-wrap
/// navigation, then left/right mutates the current row's setting on the shared
/// `GameLoop`, and Back persists + closes. Always returns true.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorVertical(action, keyCode, false)) return true;
    if menu::pass_key_to_child(g, MenuNode::Options, action, key_code)
        || menu::move_cursor_vertical(&mut g.options_menu.base, action, key_code, false)
    {
        return true;
    }
    // if (keyCode == 52 || action == 2 || keyCode == 54 || action == 5) { switch (cursorIndex) {...} }
    if key_code == 52 || action == 2 || key_code == 54 || action == 5 {
        match g.options_menu.base.cursor_index as i32 {
            // case 0: volume
            0 => {
                // if (left) volume = 0; else if (right) volume = AudioManager.maxVolume;
                if key_code == 52 || action == 2 {
                    g.game_loop.volume = 0;
                } else if key_code == 54 || action == 5 {
                    g.game_loop.volume = g.audio.max_volume;
                }
                // AudioManager.setVolume(this.gameLoop.volume);
                let vol = g.game_loop.volume;
                audio_manager::set_volume(g, vol);
                // if (this.gameLoop.volume == 0) AudioManager.stopSfx();
                if g.game_loop.volume == 0 {
                    audio_manager::stop_sfx(g);
                }
            }
            // case 1: difficulty
            1 => {
                // if (left) { difficulty = (byte)(difficulty - 1); if (difficulty < 0) difficulty = 2; }
                if key_code == 52 || action == 2 {
                    g.game_loop.difficulty = (g.game_loop.difficulty as i32).wrapping_sub(1) as i8;
                    if (g.game_loop.difficulty as i32) < 0 {
                        g.game_loop.difficulty = 2;
                    }
                }
                // if (right) { difficulty = (byte)(difficulty + 1); if (difficulty > 2) difficulty = 0; }
                if key_code == 54 || action == 5 {
                    g.game_loop.difficulty = (g.game_loop.difficulty as i32).wrapping_add(1) as i8;
                    if (g.game_loop.difficulty as i32) > 2 {
                        g.game_loop.difficulty = 0;
                    }
                }
                // this.gameLoop.setDifficulty(this.gameLoop.difficulty);
                // (DEFERRED: GameLoop.setDifficulty — unported in game_loop.rs (out of
                //  scope this lane). It re-sets `difficulty` (already set above) and
                //  `frameDelay = frameDelayTable[difficulty]`; frameDelay is not observable
                //  on the front-menu path.)
            }
            // case 2: autoTextAdvance = !autoTextAdvance;
            2 => {
                g.game_loop.auto_text_advance = !g.game_loop.auto_text_advance;
            }
            // case 3: cameraFollow = !cameraFollow;
            3 => {
                g.game_loop.camera_follow = !g.game_loop.camera_follow;
            }
            _ => {}
        }
    }
    // if (keyCode != -8) return true;
    if key_code != -8 {
        return true;
    }
    // boolean follow = this.gameLoop.cameraFollow;
    // if (follow) { GameState.camX = GameState.camTargetX; int camTargetY = GameState.camTargetY; GameState.camY = camTargetY; }
    if g.game_loop.camera_follow {
        g.game_state.cam_x = g.game_state.cam_target_x;
        let cam_target_y = g.game_state.cam_target_y;
        g.game_state.cam_y = cam_target_y;
    }
    // try { GameLoop.instance.saveOptions(); } catch (Exception e) { e.printStackTrace(); }
    // (DEFERRED: GameLoop.saveOptions — unported RMS write; the original's catch swallows
    //  any failure, leaving the settings applied in memory.)
    // ((Menu) this).parent.close();
    let parent =
        menu::parent_of(g, MenuNode::Options).expect("NullPointerException: OptionsMenu.parent");
    menu::close(g, parent);
    // return true;
    true
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`be.a:(…Graphics;II)V`): **PARTIAL** — the branch's structural frame. The
/// in-game branch draws the inset panel; the title branch draws the parchment fill +
/// title plate + "Options" heading + menu panel + the three row separators. The
/// shared option-row labels/values and the soft keys are DEFERRED (unported
/// statics).
pub fn paint(g: &mut Game, x: i32, y: i32) {
    let in_game = g.options_menu.in_game;
    let Game {
        screen,
        asset_cache,
        base_canvas,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    if in_game {
        // int panelX = x + 6; int panelY = y + 25;
        let panel_x = x.wrapping_add(6);
        let panel_y = y.wrapping_add(25);
        // Menu.drawPanelFrame(graphics, panelX, panelY, 143, 139);
        menu::draw_panel_frame(&mut graphics, panel_x, panel_y, 143, 139);
        // Menu.fillPanelInterior(graphics, panelX, panelY, 143, 139);
        menu::fill_panel_interior(&mut graphics, panel_x, panel_y, 143, 139);
        // dimColor = 10452799; valueColor = 16777215;
        // FontManager.drawSoftKeys(graphics, FontManager.blankLabel, FontManager.labelBack);
        //   (DEFERRED: FontManager.blankLabel / labelBack unported.)
        // labelX = panelX + 5 + 15; firstRowY = panelY + 15 + 10;
    } else {
        // graphics.setColor(4136767);
        graphics.set_color(4136767);
        // graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
        graphics.fill_rect(0, 0, base_canvas.width, base_canvas.height);
        // MainMenu.drawTitlePlate(graphics, x, y);
        main_menu::draw_title_plate(asset_cache, &mut graphics, x, y);
        // FontManager.drawMenuItem(graphics, 5, (x + 155) >> 1, y + 5);
        font_manager::draw_menu_item(
            font_manager,
            &mut graphics,
            base_canvas,
            5,
            ishr(x.wrapping_add(155), 1),
            y.wrapping_add(5),
        );
        // MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
        main_menu::draw_menu_panel(asset_cache, &mut graphics, x, y.wrapping_add(24), 3);
        // int labelX = x + 15 + 12; int firstRowY = y + 10 + 46;
        let label_x = x.wrapping_add(15).wrapping_add(12);
        let first_row_y = y.wrapping_add(10).wrapping_add(46);
        // graphics.drawImage(menuFrames[19], labelX+1, firstRowY+16/36/56, 20);
        let mf = asset_cache
            .menu_frames
            .as_ref()
            .expect("AssetCache.menuFrames null");
        graphics
            .draw_image(
                &mf[19],
                label_x.wrapping_add(1),
                first_row_y.wrapping_add(16),
                20,
            )
            .expect("drawImage(menuFrames[19])");
        graphics
            .draw_image(
                &mf[19],
                label_x.wrapping_add(1),
                first_row_y.wrapping_add(36),
                20,
            )
            .expect("drawImage(menuFrames[19])");
        graphics
            .draw_image(
                &mf[19],
                label_x.wrapping_add(1),
                first_row_y.wrapping_add(56),
                20,
            )
            .expect("drawImage(menuFrames[19])");
        // FontManager.drawSoftKeys(graphics, (char[]) null, FontManager.labelBack);
        //   (DEFERRED: FontManager.labelBack unported.)
    }

    // (DEFERRED — the shared option rows (volume/difficulty/autoText/camera) and the
    //  per-row slot markers. Faithful full form: for each of the four rows draw its
    //  label (AssetCache.commonText.get(18..21)) dim/white by cursorIndex, then its
    //  value centred (StringTable.instance.get(...) / AssetCache.commonText.get(60+diff)),
    //  and finally, for each of the itemCount rows, drawImage(AssetCache.slotFrame) +
    //  drawImage(AssetCache.cursorArrow). AssetCache.commonText/slotFrame/cursorArrow and
    //  the StringTable value glyphs are unported.)
}
