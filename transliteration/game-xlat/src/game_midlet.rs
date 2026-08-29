//! Transliterated from `java/src/main/java/rpg/GameMIDlet.java`
//! (original `GameMIDlet.class`, package `rpg`, in
//! `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! MIDlet-1 entry point — the only class outside the default package. On the
//! first `startApp()` it acquires the LCDUI [`Display`](j2me_me::Display), builds
//! the [`GameLoop`](crate::game_loop) singleton on it, and starts it; teardown
//! calls `notifyDestroyed()`.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): every method here is
//! arithmetic-free — `rpg.GameMIDlet.{rpg.GameMIDlet,startApp,pauseApp,destroyApp,a}`
//! all `=> []`.

use crate::game::Game;
use crate::game_loop;

/// Java `rpg.GameMIDlet` state. `instance` / `display` are references (the
/// Display object is owned by [`Game`]); each is a presence flag, `false` while
/// the Java reference is null. Field order preserves the reviewed declaration
/// order. No static-field initializers ⇒ no `<clinit>` ⇒ derived [`Default`].
#[derive(Debug, Default)]
pub struct ApplicationState {
    /// `public static GameMIDlet instance;` — the singleton self-reference,
    /// `true` once a GameMIDlet has been constructed (JVM default null → false).
    pub instance: bool,
    /// `private Display display;` — the LCDUI display acquired on first start;
    /// the object itself lives on [`Game::display`] (JVM default null → false).
    pub display: bool,
    /// `public boolean started = false;` — guards re-running startup on resume.
    pub started: bool,
}

/// `public GameMIDlet()` — the MIDlet constructor. The field initializer
/// `started = false` runs first (`iconst_0 putfield a:Z`), then the body assigns
/// the singleton `instance = this` (`aload_0 putstatic a:Lrpg/GameMIDlet;`).
pub fn construct(g: &mut Game) {
    // started = false;   (field initializer)
    g.application.started = false;
    // instance = this;
    g.application.instance = true;
}

/// `public final void startApp()` — MIDlet lifecycle. Idempotent via `started`;
/// on the first call it acquires the display and creates + starts the loop.
pub fn start_app(g: &mut Game) {
    // System.out.println("startApp");
    println!("startApp");
    // if (this.started) return;
    if g.application.started {
        return;
    }
    // this.started = true;
    g.application.started = true;
    // this.display = Display.getDisplay(this);   (reference to Game's Display)
    g.application.display = true;
    // GameLoop.create(this.display);
    game_loop::create(g);
    // GameLoop.instance.start();
    game_loop::start(g);
}

/// `public final void pauseApp()`.
pub fn pause_app(_g: &mut Game) {
    // System.out.println("pauseApp");
    println!("pauseApp");
}

/// `public final void destroyApp(boolean unconditional)` — delegates to `exit()`.
pub fn destroy_app(g: &mut Game, _unconditional: bool) {
    // exit();
    exit(g);
}

/// `public final void exit()` — ends the MIDlet via `notifyDestroyed()` (a
/// lifecycle terminator with no observable state in this runtime).
pub fn exit(_g: &mut Game) {
    // notifyDestroyed();
}
