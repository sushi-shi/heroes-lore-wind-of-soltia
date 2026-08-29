package defpackage;

import javax.microedition.midlet.MIDlet;
import rpg.GameMIDlet;

/* renamed from: w */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:w.class */
/**
 * Reads the game's build/deployment configuration out of the JAD/manifest app
 * properties and exposes it as static flags. Determines whether this is the full
 * or a demo build (the {@code HO-Demo} unlock token), which locales are offered
 * ({@code HO-LangList}), and whether an in-game "buy the full game" link should
 * appear ({@code HO-BuySetup} plus a per-locale {@code HO-URL-xx}). It also pushes
 * the resolved title/version strings into {@link FontManager} and mirrors the
 * full-version flag into {@link Debug#fullVersion}.
 *
 * <p>Not related to the settings RMS store; this is read-only launch config.
 */
public final class AppConfig {
    /* renamed from: a */
    /** The running MIDlet, source of all {@code getAppProperty} lookups. */
    public static MIDlet midlet;

    /* renamed from: a */
    /** Supported locale codes (from {@link StringTable}), indexed by locale. */
    public static String[] locales = StringTable.instance.locales;

    /* renamed from: a */
    /** Resolved per-locale "buy the full game" URL, or null if none. */
    public static String buyUrl;

    /* renamed from: a */
    /** True when a buy link is offered from the main menu ({@code HO-BuySetup} "menu"). */
    public static boolean menuBuyEnabled;
    /** True when a buy link is offered on exit ({@code HO-BuySetup} "exit"). */
    public static boolean exitBuyEnabled;

    /* renamed from: c */
    /** True for the full build; false when the {@code HO-Demo} unlock token is set. */
    public static boolean fullVersion;

    /* renamed from: a */
    /** Per-locale availability mask filled by {@link #initAvailableLocales}. */
    public static boolean[] localeEnabled;

    /* renamed from: a */
    /** Captures the MIDlet and resolves the full-version flag at startup. */
    public static final void init(MIDlet midlet) {
        AppConfig.midlet = midlet;
        fullVersion = isFullVersion();
        Debug.fullVersion = fullVersion;
    }

    /* renamed from: a */
    /** Applies buy-link setup and pushes the title/version strings to fonts. */
    public static final void apply() {
        if (isBuySetupEnabled(true)) {
            menuBuyEnabled = true;
        }
        if (isBuySetupEnabled(false)) {
            exitBuyEnabled = true;
        }
        if (menuBuyEnabled) {
            FontManager.mainMenuLabels[5] = resolveBuyLabel().toCharArray();
        } else {
            FontManager.mainMenuLabels[5] = FontManager.mainMenuLabels[6];
        }
        buyUrl = resolveBuyUrl();
        try {
            String appVersion = GameMIDlet.instance.getAppProperty("MIDlet-Version");
            String versionLine = appVersion;
            if (appVersion != null) {
                if (fullVersion) {
                    versionLine = new StringBuffer().append(versionLine).append(" ").append(FontManager.getString(3917)).toString();
                }
                FontManager.versionText = versionLine.toCharArray();
            }
        } catch (Exception unused) {
        }
    }

    /* renamed from: c */
    /** True unless the {@code HO-Demo} property holds the demo-unlock token. */
    private static boolean isFullVersion() {
        String demoToken = midlet.getAppProperty("HO-Demo");
        boolean full = true;
        if (demoToken != null && demoToken.trim().equals("BEJ8K52N7A")) {
            full = false;
        }
        return full;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** True when the full-version menu link should be shown. */
    public static final boolean showFullVersionLink() {
        return fullVersion && menuBuyEnabled && buyUrl != null;
    }

    /* renamed from: b */
    /** True when the demo "buy the full game" link should be shown. */
    public static final boolean showDemoBuyLink() {
        return (fullVersion || !menuBuyEnabled || buyUrl == null) ? false : true;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /**
     * Builds {@link #localeEnabled} from {@code HO-LangList}. Returns the single
     * forced locale index when exactly one is listed, otherwise {@code -1} (and,
     * when none are listed, enables every locale).
     */
    public static final int initAvailableLocales() {
        int localeCount = locales.length;
        localeEnabled = new boolean[localeCount];
        int matched = 0;
        int lastMatch = -1;
        String langList = midlet.getAppProperty("HO-LangList");
        if (langList != null) {
            for (int i = 0; i < localeCount; i++) {
                if (langList.indexOf(locales[i]) >= 0) {
                    System.out.println(locales[i]);
                    localeEnabled[i] = true;
                    lastMatch = i;
                    matched++;
                }
            }
        }
        if (matched == 1) {
            return lastMatch;
        }
        if (matched != 0) {
            return -1;
        }
        for (int i = 0; i < localeCount; i++) {
            localeEnabled[i] = true;
        }
        return -1;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /**
     * Resolves the localized buy-link label ({@code HO-Label-xx}), decoding any
     * {@code \\uXXXX} escapes, falling back to a string-table default and capping
     * the result at 16 characters.
     */
    public static final String resolveBuyLabel() {
        String label = midlet.getAppProperty(new StringBuffer().append("HO-Label-").append(locales[StringTable.instance.localeIndex]).toString());
        if (label == null || label.length() == 0) {
            return fullVersion ? StringTable.instance.get(3931) : StringTable.instance.get(3925);
        }
        if (label.indexOf("\\u") < 0) {
            int length = label.length();
            return label.substring(0, length < 16 ? length : 16);
        }
        StringBuffer buffer = new StringBuffer(label);
        int i = 0;
        char[] hex = new char[4];
        do {
            int at = i;
            i++;
            if (buffer.charAt(at) == '\\' && buffer.charAt(i) == 'u') {
                buffer.getChars(i + 1, i + 5, hex, 0);
                buffer.setCharAt(i - 1, (char) Integer.parseInt(FontManager.charsToString(hex), 16));
                buffer.delete(i, i + 5);
            }
        } while (i < buffer.length());
        String decoded = buffer.toString();
        int length = decoded.length();
        return decoded.substring(0, length < 16 ? length : 16);
    }

    /* JADX INFO: renamed from: b, reason: collision with other method in class */
    /** Resolves the per-locale buy URL ({@code HO-URL-xx}), or null if unset. */
    private static String resolveBuyUrl() {
        String url = midlet.getAppProperty(new StringBuffer().append("HO-URL-").append(locales[StringTable.instance.localeIndex]).toString());
        if (url == null || url.length() == 0) {
            return null;
        }
        return url;
    }

    /* renamed from: a */
    /**
     * True when {@code HO-BuySetup} enables a buy link (containing "menu" when
     * {@code menu} is set, else "exit") and a locale buy URL is configured.
     */
    private static boolean isBuySetupEnabled(boolean menu) {
        String buySetup = midlet.getAppProperty("HO-BuySetup");
        String url = midlet.getAppProperty(new StringBuffer().append("HO-URL-").append(locales[StringTable.instance.localeIndex]).toString());
        if (buySetup == null || buySetup.length() == 0 || url == null || url.length() == 0) {
            return false;
        }
        if (menu) {
            return buySetup.indexOf("menu") > -1;
        }
        return buySetup.indexOf("exit") > -1;
    }
}
