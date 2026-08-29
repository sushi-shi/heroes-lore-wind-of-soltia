//! Oracle for the transliterated `AudioManager` (`bw`) — the `snd/` sound-bank +
//! volume/mixer state, driven over REAL `snd/` blobs from `_originals/…v207.jar`.
//!
//! It pins the four behaviours the port must preserve:
//!   1. `scaledVolume` scaling + clamp (`level * 10`, the R8 `imul`, bounded to
//!      `0..=maxVolume`);
//!   2. the sound-bank load path fetches the EXACT `snd/` filename from `fileTable`;
//!   3. the device-play policy that lives in the game — muting the volume
//!      (`setVolume(0) -> pause()`) STOPS the previously-active background track —
//!      with a proven-red negative control (a non-zero volume change does not);
//!   4. `playBgm` loops (`setLoopCount(-1)`) and never restarts an already-playing
//!      track, and `setVolume` pushes the scaled level to every loaded clip.
//!
//! The observable oracle is `j2me-me`'s ordered `HostAudioOp` sink. Liveness: a
//! corpus floor loads all 32 clips from the JAR (fails loudly if `_originals/` is
//! absent — GATES.md R4).

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::audio_manager::{self, FILE_TABLE};
use heroes_lore_wind_of_soltia_game_xlat::{sound_player, Game};
use j2me_me::HostAudioOp;

/// A `Game` whose resource bank holds the given clips' `snd/` files from the JAR.
fn game_with_snd(clip_ids: &[usize]) -> Game {
    let jar = jar();
    let mut g = Game::new();
    for &id in clip_ids {
        let zip = format!("snd/{}", FILE_TABLE[id]);
        let bytes = jar
            .get(&zip)
            .unwrap_or_else(|| panic!("baseline JAR missing {zip} — corpus broken"));
        assert!(!bytes.is_empty(), "{zip} empty — corpus broken");
        g.resources.insert(zip, to_i8(bytes));
    }
    g
}

#[test]
fn set_volume_scales_by_ten_and_clamps_to_max_volume() {
    let mut g = Game::new();
    assert_eq!(g.audio.max_volume, 10, "bw.<clinit>: maxVolume = 10");
    assert_eq!(g.audio.scaled_volume, 0);

    audio_manager::set_volume(&mut g, 7);
    assert_eq!(g.audio.scaled_volume, 70, "level * 10 (imul)");

    audio_manager::set_volume(&mut g, -3); // level <= 0 -> 0
    assert_eq!(g.audio.scaled_volume, 0);

    audio_manager::set_volume(&mut g, 99); // level > maxVolume -> 10 -> 100
    assert_eq!(g.audio.scaled_volume, 100);

    audio_manager::set_volume(&mut g, 10); // exactly max
    assert_eq!(g.audio.scaled_volume, 100);
}

#[test]
fn load_clip_fetches_the_exact_snd_filename_from_the_table() {
    // clip 0 -> "00.mid" present in the bank; clip 1 -> "01.mid" ABSENT.
    let mut g = game_with_snd(&[0]);
    g.media.clear_ops();

    audio_manager::load_clip(&mut g, 0);
    let h0 = g.audio.clips[0].expect("clip 0 slot filled");
    assert!(
        g.audio.pool[h0].player.is_some(),
        "clip 0's resource (00.mid) was found -> real player"
    );
    assert!(
        g.media.ops().iter().any(|o| matches!(o,
            HostAudioOp::Create { mime, track: 0, .. } if mime == "audio/midi")),
        "clip 0 created with audio/midi from 00.mid"
    );

    // clip 1's file (01.mid) is not in the bank -> the exact name missed -> null.
    audio_manager::load_clip(&mut g, 1);
    let h1 = g.audio.clips[1].expect("clip 1 slot filled (wrapper created)");
    assert!(
        g.audio.pool[h1].player.is_none(),
        "clip 1's resource (01.mid) is absent -> null player"
    );
}

#[test]
fn load_clip_is_idempotent() {
    let mut g = game_with_snd(&[0]);
    audio_manager::load_clip(&mut g, 0);
    let pool_len = g.audio.pool.len();
    g.media.clear_ops();
    audio_manager::load_clip(&mut g, 0); // already loaded
    assert_eq!(g.audio.pool.len(), pool_len, "no second wrapper");
    assert!(
        g.media.ops().is_empty(),
        "no device ops on a redundant load"
    );
}

#[test]
fn play_sfx_routes_sets_volume_and_starts_the_sfx_channel() {
    let mut g = game_with_snd(&[8]); // 08.wav
    g.game_loop.volume = 10; // play() gate: volume > 0
    audio_manager::set_volume(&mut g, 10); // scaledVolume = 100
    audio_manager::load_clip(&mut g, 8);
    let h = g.audio.clips[8].unwrap();
    let p = g.audio.pool[h].player.unwrap();
    g.media.clear_ops();

    audio_manager::play_sfx(&mut g, 8, false);
    assert_eq!(g.audio.sfx, Some(h), "sfx routed to clip 8");
    assert_eq!(
        g.media.drain_ops(),
        vec![
            HostAudioOp::SetVolume {
                player: p,
                level: 100
            },
            HostAudioOp::Realize { player: p },
            HostAudioOp::Prefetch { player: p },
            HostAudioOp::Start {
                player: p,
                track: 8
            },
        ],
        "playSfx: setVolume(scaledVolume) then play() -> start()"
    );
}

#[test]
fn muting_the_volume_stops_the_active_background_track() {
    // The device-play policy that lives in AudioManager: when the volume drops to
    // zero, setVolume -> pause() stops the currently-playing background track.
    let mut g = game_with_snd(&[22]); // 22.mid
    g.game_loop.volume = 10;
    audio_manager::set_volume(&mut g, 10); // scaledVolume = 100
    audio_manager::load_clip(&mut g, 22);
    audio_manager::play_bgm(&mut g, 22);
    let h = g.audio.bgm.expect("bgm routed to clip 22");
    let p = g.audio.pool[h].player.unwrap();
    assert!(
        sound_player::is_playing(&g.media, &g.audio.pool[h]),
        "bgm playing before the mute"
    );
    g.media.clear_ops();

    audio_manager::set_volume(&mut g, 0); // mute -> scaledVolume 0 -> pause()
    assert!(
        !sound_player::is_playing(&g.media, &g.audio.pool[h]),
        "the previously-active bgm was stopped at the mute boundary"
    );
    assert!(
        g.media.ops().iter().any(|o| matches!(o,
            HostAudioOp::Stop { player, .. } if *player == p)),
        "pause() emitted Stop on the active bgm player"
    );
}

#[test]
fn negative_control_a_nonmute_volume_change_does_not_stop_the_bgm() {
    let mut g = game_with_snd(&[22]);
    g.game_loop.volume = 10;
    audio_manager::set_volume(&mut g, 10);
    audio_manager::load_clip(&mut g, 22);
    audio_manager::play_bgm(&mut g, 22);
    let h = g.audio.bgm.unwrap();
    let p = g.audio.pool[h].player.unwrap();
    g.media.clear_ops();

    // Lower the volume but NOT to zero -> no pause -> bgm keeps playing.
    audio_manager::set_volume(&mut g, 5);
    assert!(
        sound_player::is_playing(&g.media, &g.audio.pool[h]),
        "a non-zero volume change must NOT stop the bgm"
    );
    assert!(
        !g.media.ops().iter().any(|o| matches!(o,
            HostAudioOp::Stop { player, .. } if *player == p)),
        "no Stop when volume stays above zero — proves the mute-Stop assertion bites"
    );
    assert!(
        g.media.ops().iter().any(|o| matches!(o,
            HostAudioOp::SetVolume { player, level: 50 } if *player == p)),
        "the new scaled level (50) is still pushed to the bgm channel"
    );
}

#[test]
fn play_bgm_loops_and_does_not_restart_an_already_playing_track() {
    let mut g = game_with_snd(&[22]);
    g.game_loop.volume = 10;
    audio_manager::set_volume(&mut g, 10);
    audio_manager::load_clip(&mut g, 22);
    g.media.clear_ops();

    audio_manager::play_bgm(&mut g, 22);
    let h = g.audio.bgm.expect("bgm routed to clip 22");
    let p = g.audio.pool[h].player.unwrap();
    let first = g.media.drain_ops();
    assert!(
        first.contains(&HostAudioOp::SetLoopCount {
            player: p,
            count: -1
        }),
        "playBgm loops with setLoopCount(-1)"
    );
    assert!(
        first.contains(&HostAudioOp::Start {
            player: p,
            track: 22
        }),
        "playBgm starts the track"
    );
    assert!(sound_player::is_playing(&g.media, &g.audio.pool[h]));

    audio_manager::play_bgm(&mut g, 22); // already playing -> early return
    assert!(
        !g.media
            .drain_ops()
            .iter()
            .any(|o| matches!(o, HostAudioOp::Start { .. })),
        "an already-playing bgm is not restarted"
    );
}

#[test]
fn set_volume_pushes_the_scaled_level_to_every_loaded_clip() {
    let mut g = game_with_snd(&[0, 8, 22]);
    audio_manager::load_clip(&mut g, 0);
    audio_manager::load_clip(&mut g, 8);
    audio_manager::load_clip(&mut g, 22);
    let players: Vec<_> = [0usize, 8, 22]
        .iter()
        .map(|&i| {
            let h = g.audio.clips[i].unwrap();
            g.audio.pool[h].player.unwrap()
        })
        .collect();
    g.media.clear_ops();

    audio_manager::set_volume(&mut g, 6); // scaledVolume = 60
    let ops = g.media.drain_ops();
    for p in &players {
        assert!(
            ops.contains(&HostAudioOp::SetVolume {
                player: *p,
                level: 60
            }),
            "scaledVolume pushed to loaded clip {p:?}"
        );
    }
    assert_eq!(g.audio.scaled_volume, 60);
}

#[test]
fn ready_sound_applies_the_stored_gameloop_volume() {
    let mut g = Game::new();
    g.game_loop.volume = 4; // as loadOptions (deferred) would have restored it
    audio_manager::ready_sound(&mut g);
    assert_eq!(
        g.audio.scaled_volume, 40,
        "readySound -> setVolume(GameLoop.volume=4) -> 40"
    );
}

#[test]
fn every_snd_clip_in_the_table_loads_from_the_baseline_jar() {
    // Liveness + count floor: every fileTable entry maps to a real snd resource in
    // the JAR, so all 32 clips create a player. Fails loudly via jar() if absent.
    let jar = jar();
    let mut g = Game::new();
    for name in FILE_TABLE.iter().collect::<std::collections::BTreeSet<_>>() {
        let zip = format!("snd/{name}");
        let bytes = jar
            .get(&zip)
            .unwrap_or_else(|| panic!("baseline JAR missing {zip}"));
        assert!(!bytes.is_empty());
        g.resources.insert(zip, to_i8(bytes));
    }

    let mut created = 0usize;
    for id in 0..32u8 {
        audio_manager::load_clip(&mut g, id as i8);
        let h = g.audio.clips[id as usize].expect("clip slot filled");
        if g.audio.pool[h].player.is_some() {
            created += 1;
        }
    }
    assert_eq!(created, 32, "all 32 clips created a player from the JAR");
    assert!(g.audio.pool.len() >= 32, "sound-bank count floor");
}
