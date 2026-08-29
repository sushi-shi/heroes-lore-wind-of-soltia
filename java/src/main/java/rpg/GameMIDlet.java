package rpg;

import defpackage.GameLoop;
import javax.microedition.lcdui.Display;
import javax.microedition.midlet.MIDlet;

/* renamed from: rpg.GameMIDlet */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:rpg/GameMIDlet.class */
/**
 * MIDlet-1 entry point (the only class outside the default package). On the
 * first {@code startApp()} it builds the {@link GameLoop} on the LCDUI display
 * and starts it; teardown just calls {@code notifyDestroyed()}.
 */
public class GameMIDlet extends MIDlet {
    public static GameMIDlet instance;

    /* renamed from: a */
    /** LCDUI display acquired on first start. */
    private Display display;

    /* renamed from: a */
    /** Guards against re-running startup on resume. */
    public boolean started = false;

    public GameMIDlet() {
        instance = this;
    }

    public final void startApp() {
        System.out.println("startApp");
        if (this.started) {
            return;
        }
        this.started = true;
        this.display = Display.getDisplay(this);
        GameLoop.create(this.display);
        GameLoop.instance.start();
    }

    public final void pauseApp() {
        System.out.println("pauseApp");
    }

    public final void destroyApp(boolean unconditional) {
        exit();
    }

    /** Ends the MIDlet. */
    public final void exit() {
        notifyDestroyed();
    }
}
