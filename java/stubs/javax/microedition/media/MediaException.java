package javax.microedition.media;

// Minimal compile-only stub of the MMAPI checked exception (JSR-135).
// Thrown by Manager.createPlayer and Player.realize/prefetch/start/stop; the
// baseline (ci) catches it directly, so it must be a checked Exception subtype.
public class MediaException extends Exception {

    public MediaException(String message) {
        super(message);
    }

    public MediaException() {
        super();
    }
}
