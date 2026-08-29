package javax.microedition.lcdui;

// Minimal compile-only stub of the MIDP Displayable base class (JSR-118 lcdui).
// The baseline references it only as the parameter type of Display.setCurrent
// and as the superclass of Canvas; no methods are called on it directly.
public abstract class Displayable {

    Displayable() {
    }

    public int getWidth() {
        return 0;
    }

    public int getHeight() {
        return 0;
    }
}
