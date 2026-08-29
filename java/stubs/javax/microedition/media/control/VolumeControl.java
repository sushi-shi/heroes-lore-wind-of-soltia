package javax.microedition.media.control;

import javax.microedition.media.Control;

// Minimal compile-only stub of the MMAPI VolumeControl (JSR-135).
// setLevel returns int, matching the baseline reference (ci.b -> setLevel:(I)I).
public interface VolumeControl extends Control {

    int setLevel(int level);

    int getLevel();

    void setMute(boolean mute);

    boolean isMuted();
}
