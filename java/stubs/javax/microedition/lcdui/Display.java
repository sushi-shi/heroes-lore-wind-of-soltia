package javax.microedition.lcdui;

// Minimal compile-only stub of the MIDP Display (JSR-118 lcdui).
// Referenced by the baseline via getDisplay (static), setCurrent, callSerially.
public class Display {

    Display() {
    }

    public static Display getDisplay(javax.microedition.midlet.MIDlet m) {
        return new Display();
    }

    public void setCurrent(Displayable next) {
    }

    public Displayable getCurrent() {
        return null;
    }

    public void callSerially(Runnable r) {
    }

    public boolean isColor() {
        return true;
    }

    public int numColors() {
        return 0;
    }
}
