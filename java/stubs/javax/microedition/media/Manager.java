package javax.microedition.media;

import java.io.IOException;
import java.io.InputStream;

// Minimal compile-only stub of the MMAPI Manager (JSR-135).
// Both createPlayer overloads referenced by the baseline (ci) declare the real
// `throws IOException, MediaException`; the call sites catch plain Exception.
public final class Manager {

    private Manager() {
    }

    public static Player createPlayer(InputStream stream, String type)
            throws IOException, MediaException {
        return null;
    }

    public static Player createPlayer(String locator)
            throws IOException, MediaException {
        return null;
    }

    public static String[] getSupportedContentTypes(String protocol) {
        return new String[0];
    }

    public static String[] getSupportedProtocols(String contentType) {
        return new String[0];
    }
}
