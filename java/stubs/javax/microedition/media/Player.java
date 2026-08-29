package javax.microedition.media;

// Minimal compile-only stub of the MMAPI Player interface (JSR-135).
// Extends Controllable (so getControl is inherited, matching the baseline's
// use). realize/prefetch/start/stop declare `throws MediaException`, matching
// the real API and the baseline's try/catch(MediaException) sites; close()
// declares no checked exception (ci calls it unguarded).
public interface Player extends Controllable {

    int UNREALIZED = 100;
    int REALIZED = 200;
    int PREFETCHED = 300;
    int STARTED = 400;
    int CLOSED = 0;
    long TIME_UNKNOWN = -1L;

    void realize() throws MediaException;

    void prefetch() throws MediaException;

    void start() throws MediaException;

    void stop() throws MediaException;

    void deallocate();

    void close();

    int getState();

    void setLoopCount(int count);

    void addPlayerListener(PlayerListener playerListener);

    void removePlayerListener(PlayerListener playerListener);
}
