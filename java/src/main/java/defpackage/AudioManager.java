package defpackage;

/* renamed from: bw */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bw.class */
/**
 * The global sound mixer. It owns the {@code snd/} clip pool — 32 lazily-created
 * {@link SoundPlayer} channels indexed by clip id, mapped to filenames through
 * {@link #fileTable} — plus three named roles reused from that pool: {@link #bgm}
 * and {@link #bgm2} (looping background tracks) and {@link #sfx} (the last
 * one-shot effect). Volume is a 0..{@link #maxVolume} level scaled x10 into
 * {@link #scaledVolume} (the 0..100 MMAPI level pushed to every channel). All
 * methods are static — there is one mixer for the whole game.
 */
public final class AudioManager {

    /* renamed from: a, reason: collision with other field name */
    /** Primary looping background-music channel. */
    private static SoundPlayer bgm;

    /* renamed from: b, reason: collision with other field name */
    /** Secondary looping background-music channel. */
    private static SoundPlayer bgm2;

    /** The most recently triggered one-shot sound-effect channel. */
    private static SoundPlayer sfx;

    /** Maximum volume level (the 0..{@code maxVolume} scale exposed to the UI). */
    public static int maxVolume = 10;

    /* renamed from: b, reason: collision with other field name */
    /** Current volume as the 0..100 MMAPI level (= level x10); 0 means muted. */
    private static int scaledVolume = 0;

    /* renamed from: a, reason: collision with other field name */
    /** The 32-entry clip pool, one {@link SoundPlayer} per {@code snd/} id (lazily loaded). */
    private static SoundPlayer[] clips = new SoundPlayer[32];

    /* renamed from: a, reason: collision with other field name */
    /** Clip id -> {@code snd/} filename ("def.mid" is the silent/placeholder default). */
    private static final String[] fileTable = {"00.mid", "01.mid", "02.mid", "03.mid", "04.mid", "05.mid", "06.mid", "07.mid", "08.wav", "def.mid", "def.mid", "def.mid", "12.mid", "13.wav", "14.wav", "15.wav", "16.wav", "17.wav", "18.wav", "def.mid", "20.wav", "21.wav", "22.mid", "23.mid", "24.mid", "25.mid", "26.mid", "27.mid", "28.mid", "29.mid", "30.mid", "31.mid"};

    /* renamed from: a */
    /** Pauses background music: stops {@link #bgm}, or {@link #bgm2} if no primary. */
    public static final void pause() {
        if (bgm != null) {
            bgm.stop();
        } else if (bgm2 != null) {
            bgm2.stop();
        }
    }

    /* renamed from: b */
    /** Resumes background music: plays {@link #bgm}, or {@link #bgm2} if no primary. */
    public static final void resume() {
        if (bgm != null) {
            bgm.play();
        } else if (bgm2 != null) {
            bgm2.play();
        }
    }

    /* renamed from: c */
    /** Stops the secondary background track {@link #bgm2}. */
    public static final void stopBgm2() {
        if (bgm2 != null) {
            bgm2.stop();
        }
    }

    /* renamed from: d */
    /** Stops the current sound effect {@link #sfx}. */
    public static final void stopSfx() {
        if (sfx != null) {
            sfx.stop();
        }
    }

    /* renamed from: e */
    /** Stops the primary background track {@link #bgm}. */
    public static final void stopBgm1() {
        if (bgm != null) {
            bgm.stop();
        }
    }

    /* renamed from: f */
    /** Releases both background channels ({@link #bgm}, {@link #bgm2}). */
    public static final void stopBgm() {
        if (bgm != null) {
            bgm.dispose();
            bgm = null;
        }
        if (bgm2 != null) {
            bgm2.dispose();
            bgm2 = null;
        }
    }

    /* renamed from: a */
    /**
     * Plays clip {@code clipId} as the one-shot effect: routes it to {@link #sfx},
     * sets its volume and starts it. The {@code unused} flag is ignored (it is not
     * read in the original bytecode).
     */
    public static final void playSfx(byte clipId, boolean unused) {
        if (clips[clipId] != null) {
            sfx = clips[clipId];
            sfx.setVolume(scaledVolume);
            sfx.play();
        }
    }

    /* renamed from: a */
    /**
     * Sets the master volume from a 0..{@link #maxVolume} level, resuming or
     * pausing background music at the mute boundary and pushing the scaled
     * 0..100 level ({@link #scaledVolume}) to every loaded clip.
     */
    public static final void setVolume(int level) {
        if (level <= 0) {
            level = 0;
        } else if (level > maxVolume) {
            level = maxVolume;
        }
        if (scaledVolume == 0 && level != 0) {
            resume();
        }
        scaledVolume = level * 10;
        if (scaledVolume == 0) {
            pause();
        }
        for (int i = 0; i < clips.length; i++) {
            if (clips[i] != null) {
                clips[i].setVolume(scaledVolume);
            }
        }
    }

    /* renamed from: g */
    /** Initialises sound at startup: loads options then applies the stored volume. */
    public static final void readySound() {
        System.out.println("readySound");
        try {
            GameLoop.instance.loadOptions();
        } catch (Exception e) {
            e.printStackTrace();
        }
        setVolume(GameLoop.instance.volume);
    }

    /* renamed from: a */
    /** Lazily creates clip {@code clipId} from {@code snd/<file>} and applies the current volume. */
    public static final void loadClip(byte clipId) {
        if (clips[clipId] == null) {
            clips[clipId] = new SoundPlayer(new StringBuffer().append("resource:/snd/").append(fileTable[clipId]).toString());
            clips[clipId].setVolume(scaledVolume);
        }
    }

    /* renamed from: b */
    /** Disposes and forgets clip {@code clipId}. */
    public static final void unloadClip(byte clipId) {
        if (clips[clipId] != null) {
            clips[clipId].dispose();
            clips[clipId] = null;
        }
    }

    /* renamed from: b */
    /**
     * Makes clip {@code clipId} the primary background track and starts it looping
     * (loop count -1). No-op if it is missing or already playing.
     */
    public static final void playBgm(int clipId) {
        bgm = clips[clipId];
        if (bgm == null || bgm.isPlaying()) {
            return;
        }
        bgm.setVolume(scaledVolume);
        bgm.setLoopCount(-1);
        bgm.play();
    }

    /* renamed from: c */
    /**
     * Makes clip {@code clipId} the secondary background track and starts it
     * looping (loop count -1). No-op if it is missing or already playing.
     */
    public static final void playBgm2(int clipId) {
        bgm2 = clips[clipId];
        if (bgm2 == null || bgm2.isPlaying()) {
            return;
        }
        bgm2.setVolume(scaledVolume);
        bgm2.setLoopCount(-1);
        bgm2.play();
    }
}
