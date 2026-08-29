package defpackage;

import javax.microedition.media.Manager;
import javax.microedition.media.MediaException;
import javax.microedition.media.Player;
import javax.microedition.media.PlayerListener;
import javax.microedition.media.control.VolumeControl;

/* renamed from: ci */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ci.class */
/**
 * A wrapper around one MMAPI {@link Player} — a single sound or music track.
 * {@link AudioManager} owns a pool of these (one per {@code snd/} entry plus a
 * few named channels). The wrapper hides realize/prefetch/start/stop lifecycle
 * behind simple verbs, guesses the MMAPI content type from the file extension
 * (see {@link #contentTypeOf}), and no-ops safely when the underlying player
 * failed to create. It registers itself as a {@link PlayerListener} but ignores
 * the events.
 */
public final class SoundPlayer implements PlayerListener {
    /* renamed from: a */
    /** The wrapped MMAPI player, or null if creation failed. */
    private Player player;

    public SoundPlayer(String url) {
        create(url);
    }

    /* renamed from: a */
    /** Sets the loop count on the player (see {@link Player#setLoopCount}). */
    public final void setLoopCount(int loops) {
        if (this.player != null) {
            this.player.setLoopCount(loops);
        }
    }

    /* renamed from: a */
    /** Starts playback, but only while the global sound volume is above zero. */
    public final void play() {
        if (GameLoop.instance.volume > 0) {
            start();
        }
    }

    /* renamed from: b */
    /** Stops playback, ignoring MMAPI errors. */
    public final void stop() {
        Player p;
        try {
            if (this.player != null) {
                p = this.player;
                p.stop();
            }
        } catch (MediaException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: c */
    /** Closes and releases the player. */
    public final void dispose() {
        if (this.player != null) {
            this.player.close();
            this.player = null;
        }
    }

    /* renamed from: a, reason: collision with other method in class */
    /** True while the player is in the STARTED state (MMAPI state code >= 400). */
    public final boolean isPlaying() {
        return this.player != null && this.player.getState() >= 400;
    }

    /* renamed from: a */
    /** Guesses the MMAPI content type from the URL's extension (wav/jts/mid). */
    private static String contentTypeOf(String url) throws Exception {
        String contentType;
        if (url.endsWith("wav")) {
            contentType = "audio/x-wav";
        } else if (url.endsWith("jts")) {
            contentType = "audio/x-tone-seq";
        } else {
            if (!url.endsWith("mid")) {
                throw new Exception(new StringBuffer().append("Cannot guess content type from URL: ").append(url).toString());
            }
            contentType = "audio/midi";
        }
        return contentType;
    }

    /* renamed from: a, reason: collision with other method in class */
    /**
     * Creates the underlying player from {@code url}: an {@code http:} URL is
     * handed straight to {@link Manager#createPlayer(String)}, a
     * {@code resource:} URL is opened as a resource stream with the content type
     * from {@link #contentTypeOf} and realized. On any failure the player is
     * closed and left null.
     */
    private void create(String url) {
        if (this.player == null) {
            try {
                if (url.startsWith("http:")) {
                    this.player = Manager.createPlayer(url);
                } else if (url.startsWith("resource")) {
                    this.player = Manager.createPlayer(getClass().getResourceAsStream(url.substring(url.indexOf(58) + 1)), contentTypeOf(url));
                    this.player.realize();
                }
                this.player.addPlayerListener(this);
            } catch (Exception unused) {
                if (this.player != null) {
                    this.player.close();
                    this.player = null;
                }
            }
        }
    }

    /* renamed from: b */
    /** Sets the player's absolute volume level via its VolumeControl, if present. */
    public final void setVolume(int level) {
        VolumeControl control;
        if (this.player == null || (control = (VolumeControl) this.player.getControl("VolumeControl")) == null) {
            return;
        }
        control.setLevel(level);
    }

    /* renamed from: d */
    /** Realizes, prefetches and starts the player, ignoring MMAPI errors. */
    public final void start() {
        Player p = this.player;
        if (p != null) {
            try {
                this.player.realize();
                this.player.prefetch();
                p = this.player;
                p.start();
            } catch (MediaException e) {
                e.printStackTrace();
            }
        }
    }

    public final void playerUpdate(Player player, String event, Object data) {
    }
}
