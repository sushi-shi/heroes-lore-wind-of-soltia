package javax.microedition.media;

// Minimal compile-only stub of the MMAPI PlayerListener interface (JSR-135).
// Implemented by the baseline (ci). The string event-type constants are part of
// the real API surface; included for completeness though the baseline does not
// read them.
public interface PlayerListener {

    String STARTED = "started";
    String STOPPED = "stopped";
    String END_OF_MEDIA = "endOfMedia";
    String DURATION_UPDATED = "durationUpdated";
    String DEVICE_UNAVAILABLE = "deviceUnavailable";
    String DEVICE_AVAILABLE = "deviceAvailable";
    String VOLUME_CHANGED = "volumeChanged";
    String ERROR = "error";
    String CLOSED = "closed";

    void playerUpdate(Player player, String event, Object eventData);
}
