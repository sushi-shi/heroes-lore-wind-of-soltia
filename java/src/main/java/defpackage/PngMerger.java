package defpackage;

import java.io.IOException;
import javax.microedition.lcdui.Image;

/* renamed from: br */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:br.class */
/**
 * The atlas engine — the "PNGMerger" that reassembles individual sprite frames
 * from the game's headerless {@code .mpd} / {@code .mph} texture atlases
 * (Phase-1 spec §3.1–§3.2). It reconstructs a valid, decodable PNG on demand:
 * <ul>
 *   <li>an {@code .mph} index ({@link #mphData}) holds a header (flags + frame
 *       count) and, per frame, which {@code _<k>.mpd} file it lives in, its byte
 *       offset there, and a bitmask of which optional PNG chunks it carries;</li>
 *   <li>each {@code .mpd} is a back-to-back run of sub-PNGs stripped of the
 *       8-byte signature and IEND chunk;</li>
 *   <li>{@link #assembleFrame} stitches a full PNG for a frame: signature +
 *       IHDR + the optional chunks flagged for it + a shared PLTE/tRNS palette
 *       appended in the {@code .mph} + IDAT + IEND, then {@link Image#createImage}
 *       decodes it.</li>
 * </ul>
 * On top of assembly it offers three runtime transforms that edit the raw
 * (uncompressed, filter-0) IDAT and fix the zlib Adler-32 and chunk CRC-32:
 * {@link #imageMirrored} horizontally flips a frame at the sub-byte pixel level
 * (used for facing-left/right sprite banks — the spec's "variant" bank),
 * {@link #imageGray} / {@link #applyEffect} recolor it (grayscale, brightness,
 * invert, channel-swap, color-replace), and {@link #remapPalette} rewrites two
 * PLTE entries in place.
 */
public final class PngMerger {

    /* renamed from: a */
    /** Base resource path (without extension) of the atlas pair. */
    private String basePath;

    /* renamed from: b */
    /** {@code .mph} flags bit 0x08: frames carry an appended shared PLTE/tRNS palette. */
    private boolean mergePalette;

    /** {@code .mph} flags bit 0x04: frames need runtime palette-remap (see {@link #imageMirrored}). */
    private boolean paletteRemap;

    /* renamed from: a */
    /** Number of distinct {@code _<k>.mpd} files this atlas references. */
    private int mpdCount;

    /* renamed from: a */
    /** Frame count per {@code .mpd} file (indexed by mpd number). */
    private int[] framesPerMpd;

    /* renamed from: c */
    /** The whole {@code .mph} index blob. */
    private byte[] mphData;

    /* renamed from: a */
    /** Lazily-loaded {@code .mpd} payloads (byte[] per mpd number, else null). */
    private Object[] mpdData;

    /* renamed from: a */
    /** Per-frame optional-chunk bitmask (bit k-1 set = frame carries CHUNK_TYPES[k]). */
    private char[] chunkMasks;

    /* renamed from: b */
    /** Byte offset of the shared PLTE chunk within {@link #mphData}, or -1. */
    private int pltePos;

    /* renamed from: c */
    /** Byte offset of the shared tRNS chunk within {@link #mphData}, or -1. */
    private int trnsPos;

    /* renamed from: a */
    /** True once every frame has been extracted (allows {@code .mpd} bytes to be dropped). */
    public boolean preloadAll = false;

    /** PNG chunk-type names, indexed the way {@link #findChunk}/{@link #locateChunk} use them. */
    private static final String[] CHUNK_TYPES = {"IHDR", "cHRM", "gAMA", "iCCP", "sBIT", "sRGB", "tEXt", "zTXt", "iTXt", "pHYs", "sPLT", "tIME", "PLTE", "tRNS", "hIST", "bKGD", "IDAT", "IEND"};

    /* renamed from: a */
    /** The 8-byte PNG signature prepended to every reassembled frame. */
    private static final byte[] PNG_SIGNATURE = {-119, 80, 78, 71, 13, 10, 26, 10};

    /** A complete, pre-CRC'd IEND chunk appended to every reassembled frame. */
    private static final byte[] IEND_CHUNK = {0, 0, 0, 0, 73, 69, 78, 68, -82, 66, 96, -126};

    /* renamed from: a */
    /** Shared CRC-32 engine for fixing chunk checksums after a transform. */
    private static Crc32 crc = new Crc32();

    /* renamed from: a */
    /** Shared Adler-32 engine for fixing the IDAT zlib checksum after a transform. */
    private static Adler32 adler = new Adler32();

    public PngMerger() {
    }

    public PngMerger(String str) throws IOException {
        load(str);
    }

    /* renamed from: a */
    /** Loads the atlas: resets state, records the base path, reads the {@code .mph} index. */
    public final void load(String str) throws IOException {
        this.framesPerMpd = null;
        this.mphData = null;
        this.mpdData = null;
        this.chunkMasks = null;
        this.basePath = str;
        readIndex();
    }

    /* renamed from: b */
    /** Reads the {@code .mph} blob into {@link #mphData} and parses its header. */
    private void readIndex() throws IOException {
        this.mphData = AssetCache.readResource(new StringBuffer().append(this.basePath).append(".mph").toString());
        parseHeader();
    }

    /* renamed from: a */
    /** Loads {@code _<i>.mpd} into {@link #mpdData}. */
    public final void loadMpd(int i) throws IOException {
        this.mpdData[i] = AssetCache.readResource(new StringBuffer().append(this.basePath).append("_").append(i).append(".mpd").toString());
    }

    /* renamed from: b */
    /** Drops the cached bytes of {@code .mpd} number {@code i}. */
    public final void unloadMpd(int i) {
        this.mpdData[i] = null;
    }

    /* renamed from: a */
    /** Drops every cached {@code .mpd} and runs a GC. */
    public final void unloadAllMpd() {
        for (int i = 0; i < this.mpdCount; i++) {
            unloadMpd(i);
        }
        System.gc();
    }

    /* renamed from: c */
    /** Parses the {@code .mph} header: flags ({@link #mergePalette}/{@link #paletteRemap}), per-mpd frame counts, per-frame chunk masks, and the shared PLTE/tRNS offsets. */
    private void parseHeader() {
        int iA = readU32(this.mphData, 0);
        this.mergePalette = (iA >> 27) % 2 == 1;
        this.paletteRemap = (iA >> 26) % 2 == 1;
        int iM45a = frameCount();
        this.mpdCount = 0;
        for (int i = 0; i < iM45a; i++) {
            if (this.mpdCount < readU16(this.mphData, 8 + (8 * i)) + 1) {
                this.mpdCount = readU16(this.mphData, 8 + (8 * i)) + 1;
            }
        }
        this.framesPerMpd = new int[this.mpdCount];
        for (int i2 = 0; i2 < iM45a; i2++) {
            int[] iArr = this.framesPerMpd;
            char cM54a = readU16(this.mphData, 8 + (8 * i2));
            iArr[cM54a] = iArr[cM54a] + 1;
        }
        this.mpdData = new Object[this.mpdCount];
        this.chunkMasks = new char[iM45a];
        for (int i3 = 0; i3 < iM45a; i3++) {
            this.chunkMasks[i3] = readU16(this.mphData, 8 + (8 * i3) + 6);
        }
        this.pltePos = locateChunk(this.mphData, 12);
        this.trnsPos = locateChunk(this.mphData, 13);
    }

    /* renamed from: a */
    /** Number of frames in the atlas (u32 at mph offset 4). */
    public final int frameCount() {
        return readU32(this.mphData, 4);
    }

    /* renamed from: a */
    /** Assembles and decodes frame {@code i} (base bank). */
    public final Image image(int i) {
        byte[] bArrM51b = assembleFrame(i);
        return Image.createImage(bArrM51b, 0, bArrM51b.length);
    }

    /* renamed from: a */
    /** Extracts every frame to an Image[], enabling {@link #preloadAll}, then frees the mpd bytes. */
    public final Image[] allImages() {
        this.preloadAll = true;
        int iM45a = frameCount();
        Image[] imageArr = new Image[iM45a];
        for (int i = 0; i < iM45a; i++) {
            imageArr[i] = image(i);
            BaseCanvas.yieldTick();
        }
        unloadAllMpd();
        return imageArr;
    }

    /* renamed from: b */
    /** Frame {@code i} from the horizontally-mirrored bank (used for opposite-facing sprites); falls back to {@link #image} when the atlas needs no remap. */
    public final Image imageMirrored(int i) {
        if (!this.paletteRemap) {
            return image(i);
        }
        byte[] bArrM51b = assembleFrame(i);
        mirror(bArrM51b);
        return Image.createImage(bArrM51b, 0, bArrM51b.length);
    }

    /* renamed from: c */
    /** Assembles frame {@code i} and returns it grayscaled (effect mode 1). */
    public final Image imageGray(int i) {
        byte[] bArrM51b = assembleFrame(i);
        applyEffect(bArrM51b, 1);
        return Image.createImage(bArrM51b, 0, bArrM51b.length);
    }

    /* renamed from: a */
    /** In merge-palette atlases, rewrites two entries of the shared PLTE with colors {@code i}/{@code i2}. */
    public final void remapPalette(int i, int i2) {
        if (this.mergePalette) {
            transformPixels(this.mphData, this.pltePos, 4, i, i2);
        }
    }

    /* renamed from: a */
    /** Returns the {@code .mpd} bytes holding frame {@code i}, lazily reloading if they were freed. */
    private byte[] mpdBytes(int i) {
        int iM50a = mpdIndexOf(i);
        if (this.preloadAll && this.mpdData[iM50a] == null) {
            unloadAllMpd();
            try {
                loadMpd(iM50a);
            } catch (IOException e) {
                System.out.println(new StringBuffer().append("[PNGMerger ERROR] cannot load mpd '").append(this.basePath).append("' no.").append(iM50a).toString());
                e.printStackTrace();
            }
        }
        return (byte[]) this.mpdData[iM50a];
    }

    /* renamed from: a */
    /** Which {@code .mpd} file frame {@code i} lives in (mph record field). */
    private int mpdIndexOf(int i) {
        return readU16(this.mphData, 8 + (8 * i));
    }

    /* renamed from: b */
    /** Reassembles a full PNG for frame {@code i}, choosing the merged- or simple-palette path. */
    private byte[] assembleFrame(int i) {
        return this.mergePalette ? assembleMerged(i) : assembleSimple(i);
    }

    /* renamed from: c */
    /** Assembles frame {@code i} for a non-merge atlas: signature + the frame's IDAT slice + IEND. */
    private byte[] assembleSimple(int i) {
        byte[] bArrM49a = mpdBytes(i);
        int iA = readU32(this.mphData, 8 + (i * 8) + 2);
        int iM53b = frameLength(i);
        byte[] bArr = new byte[8 + iM53b + 12];
        System.arraycopy(PNG_SIGNATURE, 0, bArr, 0, 8);
        System.arraycopy(bArrM49a, iA, bArr, 8, iM53b);
        System.arraycopy(IEND_CHUNK, 0, bArr, 8 + iM53b, 12);
        return bArr;
    }

    /* renamed from: d */
    /** Assembles frame {@code i} for a merge-palette atlas: signature + IHDR + the optional chunks flagged for it + the shared PLTE/tRNS + IDAT + IEND. */
    private byte[] assembleMerged(int i) {
        int iA;
        int iA2;
        byte[] bArrM49a = mpdBytes(i);
        int iA3 = readU32(this.mphData, 8 + (i * 8) + 2);
        int iM53b = frameLength(i);
        byte[] bArr = new byte[8 + (this.mphData.length - ((readU32(this.mphData, 4) * 8) + 8)) + iM53b + 12];
        System.arraycopy(PNG_SIGNATURE, 0, bArr, 0, 8);
        int iA4 = findChunk(bArrM49a, 0, iA3, iM53b);
        if (iA4 == -1) {
            return null;
        }
        int iA5 = readU32(bArrM49a, iA4) + 12;
        System.arraycopy(bArrM49a, iA4, bArr, 8, iA5);
        int i2 = 8 + iA5;
        for (int i3 = 0; i3 < 18; i3++) {
            if (frameHasChunk(i, i3)) {
                switch (i3) {
                    case 1:
                    case 2:
                    case 3:
                    case 4:
                    case 5:
                    case 9:
                    case 10:
                        int iA6 = findChunk(bArrM49a, i3, iA3, iM53b);
                        if (iA6 != -1) {
                            int iA7 = readU32(bArrM49a, iA6) + 12;
                            System.arraycopy(bArrM49a, iA6, bArr, i2, iA7);
                            i2 += iA7;
                        }
                        break;
                }
            }
        }
        int i4 = this.pltePos;
        int iA8 = readU32(this.mphData, i4) + 12;
        System.arraycopy(this.mphData, i4, bArr, i2, iA8);
        int i5 = i2 + iA8;
        int i6 = this.trnsPos;
        if (i6 != -1) {
            int iA9 = readU32(this.mphData, i6) + 12;
            System.arraycopy(this.mphData, i6, bArr, i5, iA9);
            i5 += iA9;
        }
        if (frameHasChunk(i, 14) && (iA2 = findChunk(bArrM49a, 14, iA3, iM53b)) != -1) {
            int iA10 = readU32(bArrM49a, iA2) + 12;
            System.arraycopy(bArrM49a, iA2, bArr, i5, iA10);
            i5 += iA10;
        }
        if (frameHasChunk(i, 15) && (iA = findChunk(bArrM49a, 15, iA3, iM53b)) != -1) {
            int iA11 = readU32(bArrM49a, iA) + 12;
            System.arraycopy(bArrM49a, iA, bArr, i5, iA11);
            i5 += iA11;
        }
        int iA12 = findChunk(bArrM49a, 16, iA3, iM53b);
        int iA13 = readU32(bArrM49a, iA12) + 12;
        System.arraycopy(bArrM49a, iA12, bArr, i5, iA13);
        System.arraycopy(IEND_CHUNK, 0, bArr, i5 + iA13, 12);
        return bArr;
    }

    /* renamed from: b */
    /** Byte length of frame {@code i}'s data inside its {@code .mpd}. */
    private int frameLength(int i) {
        byte[] bArrM49a = mpdBytes(i);
        return ((i == frameCount() - 1 || readU16(this.mphData, 8 + (i * 8)) != readU16(this.mphData, 8 + ((i + 1) * 8))) ? bArrM49a.length : readU32(this.mphData, (8 + ((i + 1) * 8)) + 2)) - readU32(this.mphData, (8 + (i * 8)) + 2);
    }

    /* renamed from: a */
    /** Finds the byte offset of the CHUNK_TYPES[i] chunk within {@code [i2, i2+i3)} (or the whole buffer when {@code i3==-1}); -1 if absent. */
    private static int findChunk(byte[] bArr, int i, int i2, int i3) {
        String str = CHUNK_TYPES[i];
        int length = i3 == -1 ? bArr.length : i2 + i3;
        int iA = i2;
        while (true) {
            int i4 = iA;
            if (i4 >= length) {
                return -1;
            }
            if (bArr[i4 + 4] == str.charAt(0) && bArr[i4 + 5] == str.charAt(1) && bArr[i4 + 6] == str.charAt(2) && bArr[i4 + 7] == str.charAt(3)) {
                return i4;
            }
            iA = i4 + readU32(bArr, i4) + 12;
        }
    }

    /* renamed from: a */
    /** Reads a big-endian unsigned 32-bit value at {@code i}. */
    private static int readU32(byte[] bArr, int i) {
        if (bArr.length - 4 < i) {
            throw new ArrayIndexOutOfBoundsException();
        }
        return 0 + ((bArr[i] & 255) * 16777216) + ((bArr[i + 1] & 255) * 65536) + ((bArr[i + 2] & 255) * 256) + (bArr[i + 3] & 255);
    }

    /* renamed from: a */
    /** Reads a big-endian unsigned 16-bit value at {@code i}. */
    private static char readU16(byte[] bArr, int i) {
        if (bArr.length - 2 < i) {
            throw new ArrayIndexOutOfBoundsException();
        }
        return (char) (((char) (0 + ((bArr[i] & 255) * 256))) + (bArr[i + 1] & 255));
    }

    /* renamed from: a */
    /** True if frame {@code i}'s chunk bitmask has optional chunk {@code i2} set. */
    private boolean frameHasChunk(int i, int i2) {
        return i2 >= 1 && i2 <= 16 && ((this.chunkMasks[i] >> (i2 - 1)) & 1) == 1;
    }

    /* renamed from: b */
    /** Scans the whole buffer for the CHUNK_TYPES[i] chunk and returns the offset of its length field (-1 if absent). */
    private static int locateChunk(byte[] bArr, int i) {
        String str = CHUNK_TYPES[i];
        int length = bArr.length;
        for (int i2 = 0; i2 < length - 3; i2++) {
            if (bArr[i2] == str.charAt(0) && bArr[i2 + 1] == str.charAt(1) && bArr[i2 + 2] == str.charAt(2) && bArr[i2 + 3] == str.charAt(3)) {
                return i2 - 4;
            }
        }
        return -1;
    }

    /* renamed from: a */
    /** Horizontally mirrors a decoded frame in place by reversing pixels within each raw scanline of the IDAT, then fixes the Adler-32 and CRC-32. */
    public static final void mirror(byte[] bArr) {
        int iA = findChunk(bArr, 16, 8, bArr.length);
        int iA2 = findChunk(bArr, 0, 8, bArr.length);
        mirrorScanlines(bArr, iA, readU32(bArr, iA2 + 8), readU32(bArr, iA2 + 12), bArr[iA2 + 16]);
    }

    /* renamed from: a */
    /** The raw-scanline pixel-mirror worker for {@link #mirror} ({@code i}=IDAT offset, {@code i2}=width, {@code i3}=height, {@code i4}=bit depth); aborts if any scanline uses a non-zero PNG filter. */
    private static void mirrorScanlines(byte[] bArr, int i, int i2, int i3, int i4) {
        int i5 = 8 / i4;
        int i6 = ((i2 - 1) / i5) + 1;
        byte b2 = (byte) (255 >> (8 - i4));
        int i7 = i + 15;
        int i8 = (i6 + 1) * i3;
        int i9 = i2 / 2;
        int i10 = i7 + i8;
        int i11 = i10 + 4;
        int i12 = i + 4;
        for (int i13 = 0; i13 < i3; i13++) {
            if (bArr[i7 + ((i6 + 1) * i13)] != 0) {
                return;
            }
        }
        for (int i14 = 0; i14 < i3; i14++) {
            int i15 = i7 + ((i6 + 1) * i14) + 1;
            for (int i16 = 0; i16 < i9; i16++) {
                int i17 = (i2 - 1) - i16;
                int i18 = i15 + (i16 / i5);
                int i19 = i15 + (i17 / i5);
                int i20 = i16 % i5;
                int i21 = i17 % i5;
                byte b3 = (byte) (((i5 - i20) - 1) * i4);
                byte b4 = (byte) (((i5 - i21) - 1) * i4);
                byte b5 = (byte) ((bArr[i18] >> b3) & b2);
                bArr[i18] = (byte) ((bArr[i18] & ((b2 << b3) ^ (-1))) | (((byte) ((bArr[i19] >> b4) & b2)) << b3));
                bArr[i19] = (byte) ((bArr[i19] & ((b2 << b4) ^ (-1))) | (b5 << b4));
            }
        }
        adler.reset();
        adler.update(bArr, i7, i8);
        System.arraycopy(toBE32((int) adler.getValue()), 0, bArr, i10, 4);
        crc.reset();
        crc.update(bArr, i12, i8 + 15);
        System.arraycopy(toBE32(crc.getValue()), 0, bArr, i11, 4);
    }

    /* renamed from: a */
    public static final void applyEffect(byte[] bArr, int i) {
        applyEffect(bArr, i, 0);
    }

    /* renamed from: a */
    /** Applies recolor effect {@code i} (with arg {@code i2}) to the IDAT pixels and fixes the CRC. */
    public static final void applyEffect(byte[] bArr, int i, int i2) {
        transformPixels(bArr, findChunk(bArr, 12, 8, bArr.length), i, i2, 0);
    }

    /* renamed from: b */
    /** The pixel-recolor worker: mode {@code i2} selects channel-swap (0), grayscale (1), brightness (2), invert (3) or color-replace (4); recomputes the chunk CRC. */
    private static void transformPixels(byte[] bArr, int i, int i2, int i3, int i4) {
        int iA = readU32(bArr, i);
        int i5 = i + 8;
        int i6 = i5 + iA;
        switch (i2) {
            case 0:
                switch (i3) {
                    case 0:
                        for (int i7 = 0; i7 < iA / 3; i7++) {
                            byte b2 = bArr[i5 + (i7 * 3)];
                            bArr[i5 + (i7 * 3)] = bArr[i5 + (i7 * 3) + 1];
                            bArr[i5 + (i7 * 3) + 1] = b2;
                        }
                        break;
                    case 1:
                        for (int i8 = 0; i8 < iA / 3; i8++) {
                            byte b3 = bArr[i5 + (i8 * 3) + 1];
                            bArr[i5 + (i8 * 3) + 1] = bArr[i5 + (i8 * 3) + 2];
                            bArr[i5 + (i8 * 3) + 2] = b3;
                        }
                        break;
                    case 2:
                        for (int i9 = 0; i9 < iA / 3; i9++) {
                            byte b4 = bArr[i5 + (i9 * 3)];
                            bArr[i5 + (i9 * 3)] = bArr[i5 + (i9 * 3) + 2];
                            bArr[i5 + (i9 * 3) + 2] = b4;
                        }
                        break;
                    case 3:
                        for (int i10 = 0; i10 < iA / 3; i10++) {
                            byte b5 = bArr[i5 + (i10 * 3)];
                            bArr[i5 + (i10 * 3)] = bArr[i5 + (i10 * 3) + 2];
                            bArr[i5 + (i10 * 3) + 2] = bArr[i5 + (i10 * 3) + 1];
                            bArr[i5 + (i10 * 3) + 1] = b5;
                        }
                        break;
                    case 4:
                        for (int i11 = 0; i11 < iA / 3; i11++) {
                            byte b6 = bArr[i5 + (i11 * 3)];
                            bArr[i5 + (i11 * 3)] = bArr[i5 + (i11 * 3) + 1];
                            bArr[i5 + (i11 * 3) + 1] = bArr[i5 + (i11 * 3) + 2];
                            bArr[i5 + (i11 * 3) + 2] = b6;
                        }
                        break;
                }
                break;
            case 1:
                for (int i12 = 0; i12 < iA / 3; i12++) {
                    byte b7 = (byte) ((((bArr[i5 + (i12 * 3)] & 255) + (bArr[(i5 + (i12 * 3)) + 1] & 255)) + (bArr[(i5 + (i12 * 3)) + 2] & 255)) / 3);
                    bArr[i5 + (i12 * 3)] = b7;
                    bArr[i5 + (i12 * 3) + 1] = b7;
                    bArr[i5 + (i12 * 3) + 2] = b7;
                }
                break;
            case 2:
                for (int i13 = 0; i13 < iA / 3; i13++) {
                    int i14 = bArr[i5 + (i13 * 3)] & 255;
                    int i15 = bArr[i5 + (i13 * 3) + 1] & 255;
                    int i16 = bArr[i5 + (i13 * 3) + 2] & 255;
                    bArr[i5 + (i13 * 3)] = (byte) ((i14 * (i3 * 10)) / 1000 < 255 ? (i14 * (i3 * 10)) / 1000 : 255);
                    bArr[i5 + (i13 * 3) + 1] = (byte) ((i15 * (i3 * 10)) / 1000 < 255 ? (i15 * (i3 * 10)) / 1000 : 255);
                    bArr[i5 + (i13 * 3) + 2] = (byte) ((i16 * (i3 * 10)) / 1000 < 255 ? (i16 * (i3 * 10)) / 1000 : 255);
                }
                break;
            case 3:
                for (int i17 = 0; i17 < iA / 3; i17++) {
                    bArr[i5 + (i17 * 3)] = (byte) (bArr[i5 + (i17 * 3)] ^ (-1));
                    bArr[i5 + (i17 * 3) + 1] = (byte) (bArr[(i5 + (i17 * 3)) + 1] ^ (-1));
                    bArr[i5 + (i17 * 3) + 2] = (byte) (bArr[(i5 + (i17 * 3)) + 2] ^ (-1));
                }
                break;
            case 4:
                byte b8 = (byte) ((i3 >> 16) & 255);
                byte b9 = (byte) ((i3 >> 8) & 255);
                byte b10 = (byte) (i3 & 255);
                byte b11 = (byte) ((i4 >> 16) & 255);
                byte b12 = (byte) ((i4 >> 8) & 255);
                byte b13 = (byte) (i4 & 255);
                for (int i18 = 0; i18 < iA / 3; i18++) {
                    if (bArr[i5 + (i18 * 3)] == b8 && bArr[i5 + (i18 * 3) + 1] == b9 && bArr[i5 + (i18 * 3) + 2] == b10) {
                        bArr[i5 + (i18 * 3)] = b11;
                        bArr[i5 + (i18 * 3) + 1] = b12;
                        bArr[i5 + (i18 * 3) + 2] = b13;
                    }
                }
                break;
        }
        crc.reset();
        crc.update(bArr, i + 4, iA + 4);
        System.arraycopy(toBE32(crc.getValue()), 0, bArr, i6, 4);
    }

    /* renamed from: e */
    /** Encodes {@code i} as 4 big-endian bytes. */
    private static byte[] toBE32(int i) {
        return new byte[]{(byte) ((i >> 24) & 255), (byte) ((i >> 16) & 255), (byte) ((i >> 8) & 255), (byte) (i & 255)};
    }
}
