package javax.microedition.midlet;

// Minimal compile-only stub of the MIDP MIDlet lifecycle base class.
// Behaviour-free: signatures match the JSR-118 API surface the baseline
// bytecode references (rpg.GameMIDlet extends this). No runtime behaviour.
//
// The real startApp/pauseApp/destroyApp are `protected abstract` and
// platformRequest declares `throws ConnectionNotFoundException`; the baseline's
// only platformRequest call site catches plain Exception, so the checked throws
// is dropped here to keep the stub self-contained (a wider catch is always
// legal in Java, so this does not change what compiles).
public abstract class MIDlet {

    protected MIDlet() {
    }

    protected abstract void startApp();

    protected abstract void pauseApp();

    protected abstract void destroyApp(boolean unconditional);

    public final void notifyDestroyed() {
    }

    public final void notifyPaused() {
    }

    public final String getAppProperty(String key) {
        return null;
    }

    public final void resumeRequest() {
    }

    public final boolean platformRequest(String url) {
        return false;
    }

    public final int checkPermission(String permission) {
        return 0;
    }
}
