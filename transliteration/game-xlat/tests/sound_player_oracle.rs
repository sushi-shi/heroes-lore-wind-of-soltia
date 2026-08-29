//! Oracle for the transliterated `SoundPlayer` (`ci`) — its MMAPI 5-state
//! lifecycle driven through `j2me-me`'s `MediaRuntime`, over REAL `snd/` blobs
//! pulled from `_originals/…v207.jar` (never into the repo tree).
//!
//! The observable oracle is the ordered `HostAudioOp` sink: each verb of the
//! wrapper must emit exactly the device operations the Java calls would, in order.
//! A proven-red negative control (an un-realized player is state 100, not 200)
//! shows the lifecycle assertions bite; the helpers panic loudly if `_originals/`
//! is absent (GATES.md R4 — a corpus oracle never skips to a false green).

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::resources::ResourceBank;
use heroes_lore_wind_of_soltia_game_xlat::sound_player;
use j2me_me::{HostAudioOp, MediaRuntime};

const SND_MIDI: &str = "snd/00.mid";
const SND_WAV: &str = "snd/08.wav";

/// A `ResourceBank` holding the named baseline-JAR `snd/` entries, verified live.
fn bank_with(names: &[&str]) -> ResourceBank {
    let jar = jar();
    let mut bank = ResourceBank::new();
    for name in names {
        let bytes = jar
            .get(name)
            .unwrap_or_else(|| panic!("baseline JAR missing {name} — corpus broken"));
        assert!(
            !bytes.is_empty(),
            "snd resource {name} is empty — corpus broken"
        );
        bank.insert(*name, to_i8(bytes));
    }
    bank
}

#[test]
fn sound_player_walks_the_mmapi_lifecycle_over_real_snd_bytes() {
    let bank = bank_with(&[SND_MIDI]);
    let mut rt = MediaRuntime::new();

    // new SoundPlayer("resource:/snd/00.mid") -> create() -> createPlayer + realize.
    let sp = sound_player::new_player(&mut rt, &bank, "resource:/snd/00.mid", 0);
    let p = sp.player.expect("a present resource yields a real player");
    assert_eq!(
        rt.get_state(p).unwrap(),
        200,
        "the constructor realized the player (REALIZED)"
    );
    assert!(!sound_player::is_playing(&rt, &sp), "not started yet");
    assert_eq!(
        rt.drain_ops(),
        vec![
            HostAudioOp::Create {
                player: p,
                track: 0,
                mime: "audio/midi".to_string(),
            },
            HostAudioOp::Realize { player: p },
        ],
        "createPlayer(stream, audio/midi) then realize()"
    );

    // play() with volume > 0 -> start(): realize -> prefetch -> start.
    sound_player::play(&mut rt, &sp, 10);
    assert_eq!(rt.get_state(p).unwrap(), 400, "STARTED");
    assert!(
        sound_player::is_playing(&rt, &sp),
        "isPlaying == getState() >= 400"
    );
    assert_eq!(
        rt.drain_ops(),
        vec![
            HostAudioOp::Realize { player: p },
            HostAudioOp::Prefetch { player: p },
            HostAudioOp::Start {
                player: p,
                track: 0
            },
        ],
        "start(): realize -> prefetch -> start"
    );

    // stop() -> PREFETCHED.
    sound_player::stop(&mut rt, &sp);
    assert_eq!(rt.get_state(p).unwrap(), 300, "PREFETCHED after stop");
    assert_eq!(
        rt.drain_ops(),
        vec![HostAudioOp::Stop {
            player: p,
            track: 0
        }]
    );
}

#[test]
fn start_and_stop_are_idempotent() {
    let bank = bank_with(&[SND_MIDI]);
    let mut rt = MediaRuntime::new();
    let sp = sound_player::new_player(&mut rt, &bank, "resource:/snd/00.mid", 0);
    rt.clear_ops();

    sound_player::start(&mut rt, &sp);
    let first = rt.drain_ops();
    assert!(
        first.iter().any(|o| matches!(o, HostAudioOp::Start { .. })),
        "first start plays"
    );

    sound_player::start(&mut rt, &sp); // already STARTED
    let second = rt.drain_ops();
    assert!(
        !second
            .iter()
            .any(|o| matches!(o, HostAudioOp::Start { .. })),
        "starting an already-STARTED player emits no Start op"
    );

    sound_player::stop(&mut rt, &sp);
    assert!(
        rt.drain_ops()
            .iter()
            .any(|o| matches!(o, HostAudioOp::Stop { .. })),
        "first stop halts"
    );
    sound_player::stop(&mut rt, &sp); // already stopped
    assert!(
        rt.drain_ops().is_empty(),
        "stopping a non-started player is a silent no-op"
    );
}

#[test]
fn set_loop_count_emits_the_op_before_start() {
    let bank = bank_with(&[SND_MIDI]);
    let mut rt = MediaRuntime::new();
    let sp = sound_player::new_player(&mut rt, &bank, "resource:/snd/00.mid", 0);
    let p = sp.player.unwrap();
    rt.clear_ops();
    sound_player::set_loop_count(&mut rt, &sp, -1);
    assert_eq!(
        rt.drain_ops(),
        vec![HostAudioOp::SetLoopCount {
            player: p,
            count: -1
        }]
    );
}

#[test]
fn set_volume_clamps_to_the_mmapi_0_100_range() {
    let bank = bank_with(&[SND_WAV]);
    let mut rt = MediaRuntime::new();
    let sp = sound_player::new_player(&mut rt, &bank, "resource:/snd/08.wav", 8);
    let p = sp.player.unwrap();
    rt.clear_ops();

    sound_player::set_volume(&mut rt, &sp, 70);
    sound_player::set_volume(&mut rt, &sp, 150); // clamps to 100
    sound_player::set_volume(&mut rt, &sp, -5); // clamps to 0
    assert_eq!(
        rt.drain_ops(),
        vec![
            HostAudioOp::SetVolume {
                player: p,
                level: 70
            },
            HostAudioOp::SetVolume {
                player: p,
                level: 100
            },
            HostAudioOp::SetVolume {
                player: p,
                level: 0
            },
        ]
    );
}

#[test]
fn content_type_is_guessed_from_the_extension() {
    let bank = bank_with(&[SND_MIDI, SND_WAV]);
    let mut rt = MediaRuntime::new();
    let midi = sound_player::new_player(&mut rt, &bank, "resource:/snd/00.mid", 0);
    let wav = sound_player::new_player(&mut rt, &bank, "resource:/snd/08.wav", 8);
    let pm = midi.player.unwrap();
    let pw = wav.player.unwrap();
    let ops = rt.ops();
    assert!(
        ops.contains(&HostAudioOp::Create {
            player: pm,
            track: 0,
            mime: "audio/midi".to_string(),
        }),
        ".mid -> audio/midi"
    );
    assert!(
        ops.contains(&HostAudioOp::Create {
            player: pw,
            track: 8,
            mime: "audio/x-wav".to_string(),
        }),
        ".wav -> audio/x-wav"
    );
}

#[test]
fn a_missing_resource_leaves_the_player_null() {
    // getResourceAsStream -> null -> createPlayer(null,...) throws -> player null.
    let bank = bank_with(&[SND_MIDI]); // "01.mid" deliberately absent
    let mut rt = MediaRuntime::new();
    let mut sp = sound_player::new_player(&mut rt, &bank, "resource:/snd/01.mid", 1);
    assert!(sp.player.is_none(), "absent resource -> null player");
    assert!(!sound_player::is_playing(&rt, &sp));
    assert!(
        rt.ops()
            .iter()
            .all(|o| !matches!(o, HostAudioOp::Create { .. })),
        "no player is created for an absent resource"
    );

    // Every verb is a safe no-op on a null-player wrapper.
    sound_player::play(&mut rt, &sp, 10);
    sound_player::set_volume(&mut rt, &sp, 50);
    sound_player::set_loop_count(&mut rt, &sp, -1);
    sound_player::stop(&mut rt, &sp);
    sound_player::dispose(&mut rt, &mut sp);
    assert!(
        rt.ops().is_empty(),
        "all verbs no-op on a null-player wrapper"
    );
}

#[test]
fn play_does_not_start_when_volume_is_zero() {
    let bank = bank_with(&[SND_MIDI]);
    let mut rt = MediaRuntime::new();
    let sp = sound_player::new_player(&mut rt, &bank, "resource:/snd/00.mid", 0);
    rt.clear_ops();

    sound_player::play(&mut rt, &sp, 0); // GameLoop.volume == 0
    assert!(!sound_player::is_playing(&rt, &sp));
    assert!(
        rt.drain_ops().is_empty(),
        "play() is a no-op while volume == 0"
    );

    // ...and it DOES start once the volume rises (the proven contrast).
    sound_player::play(&mut rt, &sp, 1);
    assert!(sound_player::is_playing(&rt, &sp));
}

#[test]
fn dispose_closes_and_nulls_the_player() {
    let bank = bank_with(&[SND_MIDI]);
    let mut rt = MediaRuntime::new();
    let mut sp = sound_player::new_player(&mut rt, &bank, "resource:/snd/00.mid", 0);
    let p = sp.player.unwrap();
    rt.clear_ops();

    sound_player::dispose(&mut rt, &mut sp);
    assert!(sp.player.is_none(), "dispose nulls the wrapper's player");
    assert_eq!(rt.get_state(p).unwrap(), 0, "underlying player CLOSED");
    assert_eq!(rt.drain_ops(), vec![HostAudioOp::Close { player: p }]);
}

#[test]
fn negative_control_an_unrealized_player_is_state_100_not_200() {
    // Proves the lifecycle assertions bite: the constructor's realize() is what
    // moves a fresh player from UNREALIZED(100) to REALIZED(200). Drop that call
    // and the "== 200" assertions above would read 100.
    let mut rt = MediaRuntime::new();
    let p = rt.create_player(0, "audio/midi"); // created, NOT realized
    assert_eq!(rt.get_state(p).unwrap(), 100, "UNREALIZED");
    assert_ne!(
        rt.get_state(p).unwrap(),
        200,
        "an un-realized player must differ from the realized state the ctor reaches"
    );
}
