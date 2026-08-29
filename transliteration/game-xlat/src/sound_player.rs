//! Transliterated from `java/src/main/java/defpackage/SoundPlayer.java`
//! (original `ci.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! `SoundPlayer` (`ci`) wraps one MMAPI `Player` — a single sound or music track.
//! [`AudioManager`](crate::audio_manager) owns a pool of these. The wrapper hides
//! the realize/prefetch/start/stop lifecycle behind simple verbs, guesses the
//! MMAPI content type from the file extension ([`content_type_of`]), and no-ops
//! safely when the underlying player failed to create. It registers itself as a
//! `PlayerListener` but ignores the events ([`player_update`] is empty).
//!
//! The 5-state MMAPI lifecycle is `j2me-me`'s [`PlayerState`](j2me_me::PlayerState)
//! (`UNREALIZED 100 → REALIZED 200 → PREFETCHED 300 → STARTED 400`, plus
//! `CLOSED 0`); `isPlaying` is `getState() >= 400`. Each device call is routed
//! through the runtime's ordered [`HostAudioOp`](j2me_me::HostAudioOp) sink, so a
//! test can assert the exact operation sequence reaching the host.
//!
//! ## Structural notes
//!
//! - The instance field `player` (`ci.a:Ljavax/microedition/media/Player;`) becomes
//!   [`SoundPlayerState::player`], an `Option<PlayerId>` handle into the
//!   [`MediaRuntime`](j2me_me::MediaRuntime) arena (`null` → `None`). `ci` has no
//!   `static` state, so it owns no `ownership.tsv` rows (like `BitmapFont`).
//! - `Manager.createPlayer(InputStream, String)` takes a resource *stream*; the
//!   idiomatic `j2me-me` model identifies the stream by an integer `track` (which
//!   sound the host renders). That `track` is a host-boundary datum, not
//!   Java-observable, so [`new_player`]/[`create`] accept it as an extra parameter;
//!   [`AudioManager`](crate::audio_manager) passes the clip id. The Java-observable
//!   behaviour (which resource is fetched, the lifecycle, the volume, the op order)
//!   is unchanged.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `ci.<init>:(Ljava/lang/String;)V => []`, `ci.a:(Ljava/lang/String;)V => [iadd]`
//! (create — `url.indexOf(58) + 1`), `ci.a:(Ljava/lang/String;)Ljava/lang/String;
//! => []` (contentTypeOf), `ci.a:(I)V => []` (setLoopCount), `ci.a:()V => []`
//! (play), `ci.b:()V => []` (stop), `ci.c:()V => []` (dispose), `ci.a:()Z => []`
//! (isPlaying), `ci.b:(I)V => []` (setVolume), `ci.d:()V => []` (start),
//! `ci.playerUpdate:(...)V => []`.

use crate::resources::ResourceBank;
use j2me_jvm::JavaError;
use j2me_me::{MediaRuntime, PlayerId};

/// Java `ci` / `SoundPlayer` instance state — the single wrapped MMAPI player.
/// One per pool entry; not a singleton (no `*State` on [`Game`](crate::Game)).
#[derive(Debug, Clone)]
pub struct SoundPlayerState {
    /// `private Player player;` — the wrapped MMAPI player handle, or `None` if
    /// creation failed (Java `null`).
    pub player: Option<PlayerId>,
}

/// `public SoundPlayer(String url)` (`ci.<init>:(Ljava/lang/String;)V => []`) —
/// `create(url)`. `track` is the host-boundary sound identity (see the module
/// docs); it is not part of the Java constructor signature.
pub fn new_player(
    rt: &mut MediaRuntime,
    resources: &ResourceBank,
    url: &str,
    track: i32,
) -> SoundPlayerState {
    // this.player defaults to null.
    let mut sp = SoundPlayerState { player: None };
    // create(url);
    create(rt, resources, &mut sp, url, track);
    sp
}

/// `private void create(String url)`
/// (`ci.a:(Ljava/lang/String;)V => [iadd]`). Creates the underlying player from
/// `url`; on any failure the player is closed and left null.
pub fn create(
    rt: &mut MediaRuntime,
    resources: &ResourceBank,
    sp: &mut SoundPlayerState,
    url: &str,
    track: i32,
) {
    // if (this.player == null) {
    if sp.player.is_none() {
        // try { ... } catch (Exception unused) { if (player != null) { close(); player = null; } }
        if create_body(rt, resources, sp, url, track).is_err() {
            if let Some(p) = sp.player {
                // this.player.close();
                rt.close(p).expect("close: valid player handle");
                // this.player = null;
                sp.player = None;
            }
        }
    }
}

/// The `try` block of `create`. A raised `JavaError` is the Java `throw` the
/// surrounding `catch (Exception unused)` swallows (closing the player).
fn create_body(
    rt: &mut MediaRuntime,
    resources: &ResourceBank,
    sp: &mut SoundPlayerState,
    url: &str,
    track: i32,
) -> Result<(), JavaError> {
    // if (url.startsWith("http:")) {
    if url.starts_with("http:") {
        // this.player = Manager.createPlayer(url);
        //
        // The `http:` locator branch is never taken by a shipped asset (every url
        // is `resource:/snd/...`), so `j2me-me` does not model the locator-based
        // `Manager.createPlayer(String)`. Reaching here is an asset/decompilation
        // error; model it as an unsupported-locator throw the `catch` swallows.
        return Err(JavaError::Media(
            "http: locator player not modeled".to_string(),
        ));
    } else if url.starts_with("resource") {
        // Manager.createPlayer(getResourceAsStream(url.substring(url.indexOf(58) + 1)),
        //                      contentTypeOf(url))
        // — argument order: getResourceAsStream FIRST, then contentTypeOf.
        //
        // int cut = url.indexOf(58) + 1;   (iadd)
        let cut = string_index_of(url, ':').wrapping_add(1);
        // String path = url.substring(cut);
        let path = &url[cut as usize..];
        // InputStream stream = getResourceAsStream(path);
        let stream_present = resources.get(path).is_some();
        // String contentType = contentTypeOf(url);   (may throw)
        let content_type = content_type_of(url)?;
        // this.player = Manager.createPlayer(stream, contentType);
        //   a null stream makes createPlayer throw (caught below → player stays null).
        if !stream_present {
            return Err(JavaError::Io("no resource stream".to_string()));
        }
        let p = rt.create_player(track, &content_type);
        sp.player = Some(p);
        // this.player.realize();
        rt.realize(p)?;
    }
    // this.player.addPlayerListener(this);
    //   `this.player` is null iff neither branch ran (never, for a resource url);
    //   a null here is the NPE the `catch` swallows.
    let p = sp.player.ok_or(JavaError::NullPointer)?;
    rt.add_player_listener(p)?;
    Ok(())
}

/// `private static String contentTypeOf(String url) throws Exception`
/// (`ci.a:(Ljava/lang/String;)Ljava/lang/String; => []`). Guesses the MMAPI
/// content type from the URL's extension; an unknown extension throws.
fn content_type_of(url: &str) -> Result<String, JavaError> {
    let content_type;
    // if (url.endsWith("wav")) contentType = "audio/x-wav";
    if url.ends_with("wav") {
        content_type = "audio/x-wav";
    // else if (url.endsWith("jts")) contentType = "audio/x-tone-seq";
    } else if url.ends_with("jts") {
        content_type = "audio/x-tone-seq";
    } else {
        // if (!url.endsWith("mid")) throw new Exception("Cannot guess content type from URL: " + url);
        if !url.ends_with("mid") {
            return Err(JavaError::Media(format!(
                "Cannot guess content type from URL: {url}"
            )));
        }
        // contentType = "audio/midi";
        content_type = "audio/midi";
    }
    Ok(content_type.to_string())
}

/// `public final void setLoopCount(int loops)` (`ci.a:(I)V => []`). Unguarded in
/// Java: an `IllegalStateException` (player STARTED/CLOSED) would be fatal, so a
/// panic here is faithful. The game only calls this before `start()` (legal).
pub fn set_loop_count(rt: &mut MediaRuntime, sp: &SoundPlayerState, loops: i32) {
    // if (this.player != null) this.player.setLoopCount(loops);
    if let Some(p) = sp.player {
        rt.set_loop_count(p, loops)
            .expect("setLoopCount: player not STARTED/CLOSED (unguarded in Java)");
    }
}

/// `public final void play()` (`ci.a:()V => []`). Starts playback, but only while
/// the global sound `volume` (`GameLoop.instance.volume`) is above zero. `volume`
/// is a cross-owner read passed as a snapshot (see `docs/TRANSLITERATION.md`).
pub fn play(rt: &mut MediaRuntime, sp: &SoundPlayerState, volume: i32) {
    // if (GameLoop.instance.volume > 0) start();
    if volume > 0 {
        start(rt, sp);
    }
}

/// `public final void stop()` (`ci.b:()V => []`). Stops playback, ignoring MMAPI
/// errors (the `try/catch (MediaException)` swallows them).
pub fn stop(rt: &mut MediaRuntime, sp: &SoundPlayerState) {
    // try { if (this.player != null) { p = this.player; p.stop(); } }
    // catch (MediaException e) { e.printStackTrace(); }
    if let Some(p) = sp.player {
        let _ = rt.stop(p);
    }
}

/// `public final void dispose()` (`ci.c:()V => []`). Closes and releases the
/// player.
pub fn dispose(rt: &mut MediaRuntime, sp: &mut SoundPlayerState) {
    // if (this.player != null) { this.player.close(); this.player = null; }
    if let Some(p) = sp.player {
        rt.close(p).expect("close: valid player handle");
        sp.player = None;
    }
}

/// `public final boolean isPlaying()` (`ci.a:()Z => []`). True while the player is
/// STARTED (MMAPI state `>= 400`).
pub fn is_playing(rt: &MediaRuntime, sp: &SoundPlayerState) -> bool {
    // return this.player != null && this.player.getState() >= 400;
    match sp.player {
        None => false,
        Some(p) => rt.get_state(p).expect("getState: valid player handle") >= 400,
    }
}

/// `public final void setVolume(int level)` (`ci.b:(I)V => []`). Sets the player's
/// absolute volume via its `VolumeControl`, if present.
pub fn set_volume(rt: &mut MediaRuntime, sp: &SoundPlayerState, level: i32) {
    // if (this.player == null || (control = (VolumeControl) getControl("VolumeControl")) == null) return;
    let p = match sp.player {
        None => return,
        Some(p) => p,
    };
    let has_control = rt
        .get_control(p, "VolumeControl")
        .expect("getControl: valid player handle");
    if !has_control {
        return;
    }
    // control.setLevel(level);   (return value discarded)
    rt.set_level(p, level)
        .expect("setLevel: valid player handle");
}

/// `public final void start()` (`ci.d:()V => []`). Realizes, prefetches and starts
/// the player, ignoring MMAPI errors (the `try/catch (MediaException)` swallows
/// them and short-circuits the remaining calls).
pub fn start(rt: &mut MediaRuntime, sp: &SoundPlayerState) {
    // Player p = this.player; if (p != null) { try { realize(); prefetch(); p.start(); } catch (MediaException e) {...} }
    if let Some(p) = sp.player {
        let _ = start_body(rt, p);
    }
}

/// The `try` block of `start`: `realize` → `prefetch` → `start`, short-circuiting
/// on the first `MediaException` (`?`), which the caller swallows.
fn start_body(rt: &mut MediaRuntime, p: PlayerId) -> Result<(), JavaError> {
    // this.player.realize();
    rt.realize(p)?;
    // this.player.prefetch();
    rt.prefetch(p)?;
    // p.start();
    rt.start(p)?;
    Ok(())
}

/// `public final void playerUpdate(Player, String, Object)`
/// (`ci.playerUpdate:(...)V => []`). The `PlayerListener` callback — empty; the
/// wrapper ignores every event. Nothing in the transliteration invokes it.
pub fn player_update() {}

/// `String.indexOf(int ch)` for an ASCII `ch` (the only urls here are ASCII
/// resource paths). Returns the char index of the first occurrence, or `-1`.
/// `create` calls `url.indexOf(58)` (`58 == ':'`).
fn string_index_of(s: &str, ch: char) -> i32 {
    match s.find(ch) {
        Some(byte_index) => byte_index as i32,
        None => -1,
    }
}
