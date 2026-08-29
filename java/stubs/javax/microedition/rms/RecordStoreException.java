package javax.microedition.rms;

// Minimal compile-only stub of the MIDP RMS checked exception (JSR-118 rms).
// The single RMS user (au) catches plain Exception / declares `throws
// Exception`, so a single checked type suffices for the tree to compile.
public class RecordStoreException extends Exception {

    public RecordStoreException(String message) {
        super(message);
    }

    public RecordStoreException() {
        super();
    }
}
