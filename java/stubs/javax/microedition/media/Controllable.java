package javax.microedition.media;

// Minimal compile-only stub of the MMAPI Controllable interface (JSR-135).
// getControl is referenced by the baseline (ci) and returns a Control; the
// call site is not guarded, so it declares no checked exception.
public interface Controllable {

    Control getControl(String controlType);

    Control[] getControls();
}
