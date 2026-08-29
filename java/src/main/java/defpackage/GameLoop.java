package defpackage;

import javax.microedition.lcdui.Display;

/* renamed from: bs */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bs.class */
/**
 * The single-threaded game loop and settings owner. Holds the active
 * {@link BaseCanvas} screen, drives one frame per {@link #run()} via
 * {@code callSerially}, throttles to a difficulty-derived FPS, and packs the
 * persisted option/progress blob to the "/c" record store.
 */
public final class GameLoop implements Runnable {

    /* renamed from: a */
    /** MIDlet display this loop renders to. */
    private Display display;

    /* renamed from: a */
    /** Currently shown screen (title or game). */
    private BaseCanvas current;

    /* renamed from: a */
    /** The live {@link GameScreen}, or null while on the title screen. */
    public static GameScreen gameScreen;

    /** Frame delay for the current difficulty (ms, from {@link #frameDelayTable}). */
    private int frameDelay;
    /** Target milliseconds per frame ({@code 1000 / fps}). */
    private int frameTargetMs;

    /* renamed from: a */
    /** Master volume level 0..15. */
    public int volume = AudioManager.maxVolume;

    /* renamed from: a */
    /** Sound-on option (debug builds only). */
    public boolean soundEnabled;

    /** Persisted flag: a character has been created (skips the class warning). */
    public boolean hasCreatedCharacter;

    /* renamed from: c */
    /** Option: dialogue text auto-advances without a keypress. */
    public boolean autoTextAdvance;

    /* renamed from: d */
    /** Option: camera follows the hero. */
    public boolean cameraFollow;

    /* renamed from: a */
    /** Difficulty level 0..3, indexes {@link #frameDelayTable}. */
    public byte difficulty;

    /* renamed from: b */
    /** Packed one-time story/progress flag bits. */
    public byte progressFlags;

    /* renamed from: b */
    /** Obfuscated progress counter (XOR-masked in the save blob). */
    public int progressData;

    /* renamed from: a */
    /** {@code System.currentTimeMillis()} captured at frame start. */
    private long frameStartMs;

    /** Set to stop the loop from scheduling further frames. */
    public boolean stopped;
    /** True until the first {@link #run()} has bootstrapped the title screen. */
    private boolean bootPending;

    /* renamed from: a */
    /** Global singleton. */
    public static GameLoop instance;

    /** Frame delay in ms per difficulty level 0..3. */
    public static final int[] frameDelayTable = {8, 10, 14, 18};

    /* renamed from: a */
    /** Monitor guarding one-frame-at-a-time execution. */
    public static Object lock = new Object();

    /** Creates the singleton loop for {@code display}. */
    public static final void create(Display display) {
        instance = new GameLoop(display);
    }

    private GameLoop(Display display) {
        this.soundEnabled = !Debug.fullVersion;
        this.hasCreatedCharacter = false;
        this.autoTextAdvance = false;
        this.cameraFollow = true;
        this.difficulty = (byte) 2;
        this.progressFlags = (byte) 0;
        this.progressData = 0;
        this.bootPending = true;
        this.display = display;
        this.frameDelay = frameDelayTable[this.difficulty];
        applyDifficultyFps();
        this.progressFlags = (byte) (this.progressFlags | 8);
    }

    @Override // java.lang.Runnable
    public final void run() {
        if (this.bootPending) {
            this.bootPending = false;
            this.current = new TitleScreen();
            this.display.setCurrent(this.current);
            ((TitleScreen) this.current).boot();
            AudioManager.readySound();
            setLoadingFps();
        }
        synchronized (lock) {
            if (this.stopped) {
                return;
            }
            markFrameStart();
            this.current.flushKey();
            this.current.requestRepaint();
            this.display.callSerially(this);
        }
    }

    /** Records the start time of the current frame. */
    public final void markFrameStart() {
        this.frameStartMs = System.currentTimeMillis();
    }

    /** Sleeps until the frame's target duration has elapsed (unless fast-forwarding). */
    public final void throttle() {
        if (EventScript.skip) {
            return;
        }
        sleepFor(this.frameStartMs, this.frameTargetMs);
    }

    /** Sleeps so that {@code targetMs} passes since {@code startMs}; yields if already over. */
    public final void sleepFor(long startMs, long targetMs) {
        long elapsedMs = System.currentTimeMillis() - startMs;
        if (elapsedMs >= targetMs) {
            Thread.yield();
        } else {
            try {
                Thread.sleep(targetMs - elapsedMs);
            } catch (InterruptedException unused) {
            }
        }
    }

    /** Marks a boot and launches the loop thread. */
    public final void start() {
        this.bootPending = true;
        new Thread(this).start();
    }

    /** Switches the display to a fresh {@link GameScreen}. */
    public final void showGameScreen() {
        this.current = new GameScreen();
        gameScreen = (GameScreen) this.current;
        this.display.setCurrent(this.current);
        GameState.buildLoadMenu();
    }

    /** Returns to the title screen in story mode. */
    public final void returnToTitle() {
        this.current = new TitleScreen();
        gameScreen = null;
        ((TitleScreen) this.current).enterStoryMode(false, (byte) 2);
        this.display.setCurrent(this.current);
        instance.setLoadingFps();
    }

    /** Sets the target frame rate to {@code fps} frames per second. */
    public final void setFps(int fps) {
        this.frameTargetMs = 1000 / fps;
    }

    /** Applies the FPS implied by the current difficulty's frame delay. */
    public final void applyDifficultyFps() {
        setFps(this.frameDelay);
    }

    /** Drops to 5 FPS for the asset-loading screen. */
    public final void setLoadingFps() {
        setFps(5);
    }

    /** Runs at 20 FPS for fast sequences. */
    public final void setFastFps() {
        setFps(20);
    }

    /** Sets difficulty {@code level} and its frame delay. */
    public final void setDifficulty(byte level) {
        this.difficulty = level;
        this.frameDelay = frameDelayTable[level];
    }

    /* renamed from: a */
    /** Serializes volume, option bits, difficulty, flags and progress into 6 bytes. */
    public final byte[] packOptions() {
        int optionByte = 0 | ((this.volume & 15) << 4);
        if (Debug.fullVersion && this.soundEnabled) {
            optionByte |= 8;
        }
        if (this.hasCreatedCharacter) {
            optionByte |= 4;
        }
        if (this.autoTextAdvance) {
            optionByte |= 2;
        }
        if (this.cameraFollow) {
            optionByte |= 1;
        }
        byte[] buffer = new byte[6];
        buffer[0] = (byte) optionByte;
        buffer[1] = (byte) (((this.difficulty & 15) << 4) | this.progressFlags);
        ByteUtil.writeI32(this.progressData ^ (-504331042), buffer, 2);
        return buffer;
    }

    /** Restores fields from a 6-byte {@link #packOptions()} blob. */
    public final void unpackOptions(byte[] data) {
        this.volume = (byte) ((data[0] & 240) >> 4);
        if (Debug.fullVersion) {
            this.soundEnabled = (data[0] & 8) != 0;
        }
        this.hasCreatedCharacter = (data[0] & 4) != 0;
        this.autoTextAdvance = (data[0] & 2) != 0;
        this.cameraFollow = (data[0] & 1) != 0;
        this.difficulty = (byte) ((data[1] & 240) >> 4);
        this.progressFlags = (byte) (data[1] & 15);
        AudioManager.setVolume(this.volume);
        setDifficulty(this.difficulty);
        this.progressData = ByteUtil.readS32(data, 2) ^ (-504331042);
    }

    /** Writes the packed options to record store "/c". */
    public final void saveOptions() throws Exception {
        byte[] packed = packOptions();
        RmsFile optionsFile = new RmsFile("/c", 0);
        optionsFile.write(packed, 0, packed.length);
        optionsFile.close();
    }

    /** Reads and applies the packed options from record store "/c". */
    public final void loadOptions() throws Exception {
        byte[] buffer = new byte[6];
        RmsFile optionsFile = new RmsFile("/c", 1);
        optionsFile.read(buffer, 0, buffer.length);
        optionsFile.close();
        unpackOptions(buffer);
    }
}
