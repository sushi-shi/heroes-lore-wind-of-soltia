//! Transliterated from `java/src/main/java/defpackage/AudioManager.java`
//! (original `bw.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! `AudioManager` (`bw`) is the global sound mixer — one for the whole game (all
//! methods `static`). It owns the `snd/` clip pool (32 lazily-created
//! [`SoundPlayer`](crate::sound_player) channels indexed by clip id, mapped to
//! filenames through [`FILE_TABLE`]) plus three named roles reused from that pool:
//! `bgm`/`bgm2` (looping background tracks) and `sfx` (the last one-shot effect).
//! Volume is a `0..maxVolume` level scaled x10 into `scaledVolume` (the `0..100`
//! MMAPI level pushed to every channel).
//!
//! ## The device-play policy lives HERE (not in `j2me-me`)
//!
//! Each MMAPI player's lifecycle is independent; the GAME decides when to stop a
//! track. In `bw` that policy is the volume-mute boundary in [`set_volume`]: when
//! the level drops to zero, `scaledVolume` becomes zero and [`pause`] **stops the
//! active background track** (the previously-playing player is halted); when it
//! rises off zero, [`resume`] restarts it. (`playSfx`/`playBgm`/`playBgm2` do NOT
//! stop a prior channel first — preserved exactly per the v207 bytecode; `playBgm`
//! only guards against restarting an already-playing track.)
//!
//! ## Structural notes / ownership
//!
//! - Statics → fields on [`AudioManagerState`] (see `java/reconstruction/ownership.tsv`).
//! - `bgm`/`bgm2`/`sfx` are Java references that ALIAS entries of the `clips[]`
//!   pool. Rust needs a concrete owner for the heap-allocated `SoundPlayer`
//!   objects, so [`AudioManagerState::pool`] is the grow-only arena that owns every
//!   created wrapper (the R10 handle-arena idiom `MediaRuntime` already uses for
//!   `Player`s), and `clips`/`bgm`/`bgm2`/`sfx` hold `usize` handles into it. This
//!   models Java reference aliasing exactly — including the case where a slot is
//!   `unloadClip`'d (its `clips[i]` handle cleared) while `bgm` still references the
//!   now-disposed object: the arena entry persists, so `bgm.stop()` still resolves
//!   to it (a null-player no-op), matching the JVM keeping the object alive. The
//!   arena never shrinks; the unreferenced-but-retained wrappers Java would GC have
//!   no finalizer, so the difference is not observable.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): every method is `[]`
//! except `bw.a:(I)V => [imul, iinc]` ([`set_volume`] — `level * 10` and the
//! push-to-all loop counter). `bw.<clinit>:()V => []`.

use crate::game::Game;
use crate::sound_player::{self, SoundPlayerState};

/// Clip id -> `snd/` filename (`bw.a:[Ljava/lang/String; = fileTable`). "def.mid"
/// is the silent/placeholder default. 32 entries, one per clip id.
pub const FILE_TABLE: [&str; 32] = [
    "00.mid", "01.mid", "02.mid", "03.mid", "04.mid", "05.mid", "06.mid", "07.mid", "08.wav",
    "def.mid", "def.mid", "def.mid", "12.mid", "13.wav", "14.wav", "15.wav", "16.wav", "17.wav",
    "18.wav", "def.mid", "20.wav", "21.wav", "22.mid", "23.mid", "24.mid", "25.mid", "26.mid",
    "27.mid", "28.mid", "29.mid", "30.mid", "31.mid",
];

/// Java `bw` / `AudioManager` static state — the one global mixer.
#[derive(Debug)]
pub struct AudioManagerState {
    /// `public static int maxVolume = 10;` — the `0..maxVolume` UI scale.
    pub max_volume: i32,
    /// `private static int scaledVolume = 0;` — current volume as the `0..100`
    /// MMAPI level (`= level * 10`); `0` means muted.
    pub scaled_volume: i32,
    /// `private static SoundPlayer bgm;` — primary looping bgm (handle into
    /// [`pool`](Self::pool), or `None` for Java `null`).
    pub bgm: Option<usize>,
    /// `private static SoundPlayer bgm2;` — secondary looping bgm.
    pub bgm2: Option<usize>,
    /// `private static SoundPlayer sfx;` — the most recent one-shot effect.
    pub sfx: Option<usize>,
    /// `private static SoundPlayer[] clips = new SoundPlayer[32];` — the 32-entry
    /// clip pool; each slot is a handle into [`pool`](Self::pool), or `None`.
    pub clips: Vec<Option<usize>>,
    /// `private static final String[] fileTable = {...};` — see [`FILE_TABLE`].
    pub file_table: Vec<String>,
    /// The heap arena that owns every created `SoundPlayer` wrapper (see the module
    /// docs). Not a Java static — it is the concrete owner for the objects
    /// `clips`/`bgm`/`bgm2`/`sfx` reference. Grow-only.
    pub pool: Vec<SoundPlayerState>,
}

impl Default for AudioManagerState {
    /// `bw.<clinit>` + field initializers (`bw.<clinit>:()V => []`): all constants
    /// and nulls, in JVM order. `bw`'s class-init has no cross-class effect
    /// (`maxVolume` is the only static another class reads, and it is a constant),
    /// so the eager `Default` is the whole `<clinit>`; no lazy trigger guard is
    /// needed.
    fn default() -> Self {
        AudioManagerState {
            // public static int maxVolume = 10;
            max_volume: 10,
            // private static int scaledVolume = 0;
            scaled_volume: 0,
            // private static SoundPlayer bgm, bgm2, sfx;   (null)
            bgm: None,
            bgm2: None,
            sfx: None,
            // private static SoundPlayer[] clips = new SoundPlayer[32];   (32 nulls)
            clips: vec![None; 32],
            // private static final String[] fileTable = {...};
            file_table: FILE_TABLE.iter().map(|s| s.to_string()).collect(),
            // the SoundPlayer heap arena — empty at class load.
            pool: Vec::new(),
        }
    }
}

/// `public static final void pause()` (`bw.a:()V => []`). Pauses background music:
/// stops `bgm`, or `bgm2` if there is no primary.
pub fn pause(g: &mut Game) {
    // if (bgm != null) bgm.stop(); else if (bgm2 != null) bgm2.stop();
    if let Some(h) = g.audio.bgm {
        sound_player::stop(&mut g.media, &g.audio.pool[h]);
    } else if let Some(h) = g.audio.bgm2 {
        sound_player::stop(&mut g.media, &g.audio.pool[h]);
    }
}

/// `public static final void resume()` (`bw.b:()V => []`). Resumes background
/// music: plays `bgm`, or `bgm2` if there is no primary.
pub fn resume(g: &mut Game) {
    // if (bgm != null) bgm.play(); else if (bgm2 != null) bgm2.play();
    let volume = g.game_loop.volume;
    if let Some(h) = g.audio.bgm {
        sound_player::play(&mut g.media, &g.audio.pool[h], volume);
    } else if let Some(h) = g.audio.bgm2 {
        sound_player::play(&mut g.media, &g.audio.pool[h], volume);
    }
}

/// `public static final void stopBgm2()` (`bw.c:()V => []`). Stops the secondary
/// background track `bgm2`.
pub fn stop_bgm2(g: &mut Game) {
    // if (bgm2 != null) bgm2.stop();
    if let Some(h) = g.audio.bgm2 {
        sound_player::stop(&mut g.media, &g.audio.pool[h]);
    }
}

/// `public static final void stopSfx()` (`bw.d:()V => []`). Stops the current
/// sound effect `sfx`.
pub fn stop_sfx(g: &mut Game) {
    // if (sfx != null) sfx.stop();
    if let Some(h) = g.audio.sfx {
        sound_player::stop(&mut g.media, &g.audio.pool[h]);
    }
}

/// `public static final void stopBgm1()` (`bw.e:()V => []`). Stops the primary
/// background track `bgm`.
pub fn stop_bgm1(g: &mut Game) {
    // if (bgm != null) bgm.stop();
    if let Some(h) = g.audio.bgm {
        sound_player::stop(&mut g.media, &g.audio.pool[h]);
    }
}

/// `public static final void stopBgm()` (`bw.f:()V => []`). Releases both
/// background channels.
pub fn stop_bgm(g: &mut Game) {
    // if (bgm != null) { bgm.dispose(); bgm = null; }
    if let Some(h) = g.audio.bgm {
        sound_player::dispose(&mut g.media, &mut g.audio.pool[h]);
        g.audio.bgm = None;
    }
    // if (bgm2 != null) { bgm2.dispose(); bgm2 = null; }
    if let Some(h) = g.audio.bgm2 {
        sound_player::dispose(&mut g.media, &mut g.audio.pool[h]);
        g.audio.bgm2 = None;
    }
}

/// `public static final void playSfx(byte clipId, boolean unused)`
/// (`bw.a:(BZ)V => []`). Plays clip `clipId` as the one-shot effect: routes it to
/// `sfx`, sets its volume and starts it. `unused` is ignored (not read in the
/// original bytecode). Does NOT stop any prior `sfx` first (preserved per bytecode).
pub fn play_sfx(g: &mut Game, clip_id: i8, _unused: bool) {
    // if (clips[clipId] != null) {
    let idx = clip_id as usize;
    if let Some(h) = g.audio.clips[idx] {
        // sfx = clips[clipId];
        g.audio.sfx = Some(h);
        // sfx.setVolume(scaledVolume);
        let scaled_volume = g.audio.scaled_volume;
        sound_player::set_volume(&mut g.media, &g.audio.pool[h], scaled_volume);
        // sfx.play();
        let volume = g.game_loop.volume;
        sound_player::play(&mut g.media, &g.audio.pool[h], volume);
    }
}

/// `public static final void setVolume(int level)` (`bw.a:(I)V => [imul, iinc]`).
/// Sets the master volume from a `0..maxVolume` level, resuming or pausing
/// background music at the mute boundary and pushing the scaled `0..100` level to
/// every loaded clip.
pub fn set_volume(g: &mut Game, level: i32) {
    let mut level = level;
    // if (level <= 0) level = 0; else if (level > maxVolume) level = maxVolume;
    if level <= 0 {
        level = 0;
    } else if level > g.audio.max_volume {
        level = g.audio.max_volume;
    }
    // if (scaledVolume == 0 && level != 0) resume();
    if g.audio.scaled_volume == 0 && level != 0 {
        resume(g);
    }
    // scaledVolume = level * 10;   (imul)
    g.audio.scaled_volume = level.wrapping_mul(10);
    // if (scaledVolume == 0) pause();
    if g.audio.scaled_volume == 0 {
        pause(g);
    }
    // for (int i = 0; i < clips.length; i++) if (clips[i] != null) clips[i].setVolume(scaledVolume);
    let scaled_volume = g.audio.scaled_volume;
    let mut i: i32 = 0;
    while i < g.audio.clips.len() as i32 {
        if let Some(h) = g.audio.clips[i as usize] {
            sound_player::set_volume(&mut g.media, &g.audio.pool[h], scaled_volume);
        }
        // i++   (iinc)
        i = i.wrapping_add(1);
    }
}

/// `public static final void readySound()` (`bw.g:()V => []`). Initialises sound
/// at startup: loads options then applies the stored volume.
pub fn ready_sound(g: &mut Game) {
    // System.out.println("readySound");   — diagnostic output, a no-op here.
    // try { GameLoop.instance.loadOptions(); } catch (Exception e) { e.printStackTrace(); }
    //   loadOptions (bs.j) — the RMS option unpack — is DEFERRED (not ported in the
    //   boot increment); its `catch` swallows any failure, leaving the fields at
    //   their construction defaults. Modeled as a no-op.
    // setVolume(GameLoop.instance.volume);
    let volume = g.game_loop.volume;
    set_volume(g, volume);
}

/// `public static final void loadClip(byte clipId)` (`bw.a:(B)V => []`). Lazily
/// creates clip `clipId` from `snd/<file>` and applies the current volume.
pub fn load_clip(g: &mut Game, clip_id: i8) {
    let idx = clip_id as usize;
    // if (clips[clipId] == null) {
    if g.audio.clips[idx].is_none() {
        // clips[clipId] = new SoundPlayer("resource:/snd/" + fileTable[clipId]);
        let mut url = String::from("resource:/snd/");
        url.push_str(&g.audio.file_table[idx]);
        let sp = sound_player::new_player(&mut g.media, &g.resources, &url, clip_id as i32);
        let h = g.audio.pool.len();
        g.audio.pool.push(sp);
        g.audio.clips[idx] = Some(h);
        // clips[clipId].setVolume(scaledVolume);
        let scaled_volume = g.audio.scaled_volume;
        sound_player::set_volume(&mut g.media, &g.audio.pool[h], scaled_volume);
    }
}

/// `public static final void unloadClip(byte clipId)` (`bw.b:(B)V => []`). Disposes
/// and forgets clip `clipId`.
pub fn unload_clip(g: &mut Game, clip_id: i8) {
    let idx = clip_id as usize;
    // if (clips[clipId] != null) { clips[clipId].dispose(); clips[clipId] = null; }
    if let Some(h) = g.audio.clips[idx] {
        sound_player::dispose(&mut g.media, &mut g.audio.pool[h]);
        g.audio.clips[idx] = None;
    }
}

/// `public static final void playBgm(int clipId)` (`bw.b:(I)V => []`). Makes clip
/// `clipId` the primary background track and starts it looping (loop count -1).
/// No-op if it is missing or already playing.
pub fn play_bgm(g: &mut Game, clip_id: i32) {
    let idx = clip_id as usize;
    // bgm = clips[clipId];
    g.audio.bgm = g.audio.clips[idx];
    // if (bgm == null || bgm.isPlaying()) return;
    let h = match g.audio.bgm {
        None => return,
        Some(h) => h,
    };
    if sound_player::is_playing(&g.media, &g.audio.pool[h]) {
        return;
    }
    // bgm.setVolume(scaledVolume);
    let scaled_volume = g.audio.scaled_volume;
    sound_player::set_volume(&mut g.media, &g.audio.pool[h], scaled_volume);
    // bgm.setLoopCount(-1);
    sound_player::set_loop_count(&mut g.media, &g.audio.pool[h], -1);
    // bgm.play();
    let volume = g.game_loop.volume;
    sound_player::play(&mut g.media, &g.audio.pool[h], volume);
}

/// `public static final void playBgm2(int clipId)` (`bw.c:(I)V => []`). Makes clip
/// `clipId` the secondary background track and starts it looping (loop count -1).
/// No-op if it is missing or already playing.
pub fn play_bgm2(g: &mut Game, clip_id: i32) {
    let idx = clip_id as usize;
    // bgm2 = clips[clipId];
    g.audio.bgm2 = g.audio.clips[idx];
    // if (bgm2 == null || bgm2.isPlaying()) return;
    let h = match g.audio.bgm2 {
        None => return,
        Some(h) => h,
    };
    if sound_player::is_playing(&g.media, &g.audio.pool[h]) {
        return;
    }
    // bgm2.setVolume(scaledVolume);
    let scaled_volume = g.audio.scaled_volume;
    sound_player::set_volume(&mut g.media, &g.audio.pool[h], scaled_volume);
    // bgm2.setLoopCount(-1);
    sound_player::set_loop_count(&mut g.media, &g.audio.pool[h], -1);
    // bgm2.play();
    let volume = g.game_loop.volume;
    sound_player::play(&mut g.media, &g.audio.pool[h], volume);
}
