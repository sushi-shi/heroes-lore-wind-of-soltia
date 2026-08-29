//! Transliterated from `java/src/main/java/defpackage/BitmapFont.java`
//! (original `az.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The `.mf` bitmap-font engine: the loader (`load` + `patchPng` + `crc32`) and
//! the glyph metrics/blitter this title-screen increment needs — `charWidth`,
//! `stringWidth`, `lineHeightOf`, `isControl`, and the core `drawChars`. Each font
//! instance owns its decoded glyph sheet (a paletted PNG whose ink palette entry
//! `patchPng` rewrites to the requested colour before `Image.createImage` decodes
//! it) and its cumulative glyph-offset table.
//!
//! ANTI-BOG: `drawString`/`drawLines`/`blockHeight` (the wrapped-block draws) are
//! **not** reached by the title paint (which calls `FontManager.drawChars` /
//! `drawCharsCentered` → `drawChars(chars,0,len,x,y,anchor)`), and are DEFERRED.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `az.a:(C)I => [iadd,i2c,isub,iadd,isub,isub,isub,i2c,isub,iadd,isub,isub]` (charWidth),
//! `az.a:([CII)I => [iadd,isub,iadd,iinc]` (stringWidth),
//! `az.b:([CII)I => [iadd,iinc,iadd]` (lineHeightOf),
//! `az.a:([BII)I => [iinc,iinc,ixor,iand,iushr,ixor,ixor]` (crc32),
//! `az.<clinit>:()V => [iand,iushr,ixor,iushr,iinc,iinc]` (crcTable),
//! `az.a:(Ljava/io/InputStream;II)V` (load) and
//! `az.a:([BIIIII)V` (patchPng) and
//! `az.a:(Ljavax/microedition/lcdui/Graphics;[CIIIII)I` (drawChars) — their full
//! opcode multisets are reproduced by the transliteration below.

use crate::game::Game;
use j2me_jvm::{ishl, ishr, iushr, java_div};

/// Java `az` / `BitmapFont` instance state, in reviewed declaration order. The
/// `static final int[] crcTable` is a class constant reproduced by [`crc_table`].
#[derive(Debug, Clone)]
pub struct BitmapFontState {
    /// `public boolean forceUppercase;`
    pub force_uppercase: bool,
    /// `public boolean hideControls;`
    pub hide_controls: bool,
    /// `private int bulletColor;`
    pub bullet_color: i32,
    /// `private Image glyphSheet;` — the decoded (palette-patched) glyph sheet.
    pub glyph_sheet: Option<j2me_me::Image>,
    /// `public int lineHeight;`
    pub line_height: i32,
    /// `private int ascent;`
    pub ascent: i32,
    /// `private int spaceWidth;`
    pub space_width: i32,
    /// `private int kerning;`
    pub kerning: i32,
    /// `private short[] glyphOffsets;`
    pub glyph_offsets: Vec<i16>,
    /// `public int lineSpacing = 2;`
    pub line_spacing: i32,
    /// `private boolean hasAccents = true;`
    pub has_accents: bool,
}

impl Default for BitmapFontState {
    fn default() -> Self {
        // Field initializers: lineSpacing = 2, hasAccents = true; the rest at their
        // JVM defaults until `load`.
        BitmapFontState {
            force_uppercase: false,
            hide_controls: false,
            bullet_color: 0,
            glyph_sheet: None,
            line_height: 0,
            ascent: 0,
            space_width: 0,
            kerning: 0,
            glyph_offsets: Vec::new(),
            line_spacing: 2,
            has_accents: true,
        }
    }
}

/// `static final int[] crcTable = new int[256]` filled by `az.<clinit>`
/// (`=> [iand,iushr,ixor,iushr,iinc,iinc]`). The original assigns `crcTable[i]`
/// on every inner iteration (the final value is what matters); reproduced.
fn crc_table() -> [i32; 256] {
    let mut table = [0i32; 256];
    // for (int i = 0; i < 256; i++)
    let mut i: i32 = 0;
    while i < 256 {
        // int c = i;
        let mut c: i32 = i;
        // for (int bit = 0; bit < 8; bit++)
        let mut bit: i32 = 0;
        while bit < 8 {
            // c = (c & 1) != 0 ? (-306674912) ^ (c >>> 1) : c >>> 1;
            c = if (c & 1) != 0 {
                (-306674912) ^ iushr(c, 1)
            } else {
                iushr(c, 1)
            };
            // crcTable[i] = c;
            table[i as usize] = c;
            bit = bit.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    table
}

/// `public BitmapFont(String name, int primaryColor, int secondaryColor, boolean forceUppercase)`
/// — `forceUppercase = forceUppercase; load(getResourceAsStream("/" + name + ".mf"), primary, secondary)`.
/// The resource read goes through [`Game::resources`] (the classpath seam).
pub fn construct(
    g: &mut Game,
    name: &str,
    primary_color: i32,
    secondary_color: i32,
    force_uppercase: bool,
) -> BitmapFontState {
    let mut s = BitmapFontState {
        force_uppercase,
        ..BitmapFontState::default()
    };
    // getResourceAsStream("/" + name + ".mf")
    let path = format!("/{name}.mf");
    let bytes: Vec<i8> = g
        .resources
        .get(&path)
        .unwrap_or_else(|| panic!("BitmapFont: resource not found: {path}"))
        .to_vec();
    load(&mut s, &bytes, primary_color, secondary_color);
    s
}

/// `public final void load(InputStream stream, int primaryColor, int secondaryColor)`.
/// Reads the metrics + glyph-offset table off the head of the `.mf` blob, patches
/// the ink colour(s) into the embedded PNG palette, and decodes the glyph sheet.
/// The `try/catch(Exception) { throw RuntimeException("MFont:") }` wrap: any
/// out-of-range read here is faithfully a panic (an uncaught MIDlet-fatal).
pub fn load(s: &mut BitmapFontState, blob: &[i8], primary_color: i32, secondary_color: i32) {
    // this.bulletColor = primaryColor;
    s.bullet_color = primary_color;
    // A byte cursor over the InputStream; `read()` returns 0..255 (or would be -1
    // at EOF; the header reads never hit EOF on a well-formed .mf).
    let mut pos: usize = 0;
    let read = |pos: &mut usize| -> i32 {
        let v = (blob[*pos] as i32) & 255;
        *pos = pos.wrapping_add(1);
        v
    };
    // stream.read(); x4  (the 4-byte header, discarded)
    read(&mut pos);
    read(&mut pos);
    read(&mut pos);
    read(&mut pos);
    // this.lineHeight = stream.read();
    s.line_height = read(&mut pos);
    // this.hasAccents = false;
    s.has_accents = false;
    // if (this.lineHeight - 100 > 0) { this.lineHeight -= 100; this.hasAccents = true; }
    if s.line_height.wrapping_sub(100) > 0 {
        s.line_height = s.line_height.wrapping_sub(100);
        s.has_accents = true;
    }
    // this.ascent = stream.read(); this.spaceWidth = stream.read(); this.kerning = stream.read();
    s.ascent = read(&mut pos);
    s.space_width = read(&mut pos);
    s.kerning = read(&mut pos);
    // this.glyphOffsets = new short[95 + (hasAccents ? 9 : 0)];
    let count: i32 = 95i32.wrapping_add(if s.has_accents { 9 } else { 0 });
    s.glyph_offsets = vec![0i16; count as usize];
    // for (g = 0; g < 95 + (hasAccents?9:0); g++)
    let mut gi: i32 = 0;
    while gi < 95i32.wrapping_add(if s.has_accents { 9 } else { 0 }) {
        // this.glyphOffsets[g] = (short) (((read() & 255) << 8) | (read() & 255));
        let hi = ishl(read(&mut pos) & 255, 8);
        let lo = read(&mut pos) & 255;
        s.glyph_offsets[gi as usize] = (hi | lo) as i16;
        gi = gi.wrapping_add(1);
    }
    // int pngLength = ((read()&255)<<8) | (read()&255);
    let png_length: i32 = ishl(read(&mut pos) & 255, 8) | (read(&mut pos) & 255);
    // int chunkOffset = ((read()&255)<<8) | (read()&255);
    let chunk_offset: i32 = ishl(read(&mut pos) & 255, 8) | (read(&mut pos) & 255);
    // int hi = read(); int lo = read();
    let hi_b: i32 = read(&mut pos);
    let lo_b: i32 = read(&mut pos);
    // int primaryColorOffset = (hi==255 && lo==255) ? -1 : ((hi&255)<<8)|(lo&255);
    let primary_color_offset: i32 = if hi_b == 255 && lo_b == 255 {
        -1
    } else {
        ishl(hi_b & 255, 8) | (lo_b & 255)
    };
    // int secondaryColorOffset = ((read()&255)<<8)|(read()&255);
    let secondary_color_offset: i32 = ishl(read(&mut pos) & 255, 8) | (read(&mut pos) & 255);
    // DataInputStream in = new DataInputStream(stream);
    // byte[] png = new byte[pngLength]; in.readFully(png);
    let mut png: Vec<i8> = blob[pos..pos.wrapping_add(png_length as usize)].to_vec();
    // if (primaryColorOffset > 0) patchPng(png, chunkOffset, primaryColorOffset, primaryColor, secondaryColorOffset, secondaryColor);
    if primary_color_offset > 0 {
        patch_png(
            &mut png,
            chunk_offset,
            primary_color_offset,
            primary_color,
            secondary_color_offset,
            secondary_color,
        );
    }
    // this.glyphSheet = Image.createImage(png, 0, pngLength);
    s.glyph_sheet =
        Some(j2me_me::create_image_region(&png, 0, png_length).expect("BitmapFont glyph sheet"));
}

/// `private void patchPng(byte[] png, int chunkOffset, int primaryColorOffset, int primaryColor, int secondaryColorOffset, int secondaryColor)`
/// (`=> [iand,ishl,iadd,...,i2b,...]`). Writes the ink colour(s) into the PNG's
/// PLTE data and recomputes that chunk's CRC in place.
pub fn patch_png(
    png: &mut [i8],
    chunk_offset: i32,
    primary_color_offset: i32,
    primary_color: i32,
    secondary_color_offset: i32,
    secondary_color: i32,
) {
    // int chunkLen = ((png[o]&255)<<24)|((png[o+1]&255)<<16)|((png[o+2]&255)<<8)|(png[o+3]&255);
    let o = chunk_offset;
    let chunk_len: i32 = ishl((png[o as usize] as i32) & 255, 24)
        | ishl((png[o.wrapping_add(1) as usize] as i32) & 255, 16)
        | ishl((png[o.wrapping_add(2) as usize] as i32) & 255, 8)
        | ((png[o.wrapping_add(3) as usize] as i32) & 255);
    // png[primaryColorOffset]   = (byte)(primaryColor >> 16);
    // png[primaryColorOffset+1] = (byte)(primaryColor >> 8);
    // png[primaryColorOffset+2] = (byte) primaryColor;
    let p = primary_color_offset;
    png[p as usize] = ishr(primary_color, 16) as i8;
    png[p.wrapping_add(1) as usize] = ishr(primary_color, 8) as i8;
    png[p.wrapping_add(2) as usize] = primary_color as i8;
    // if (secondaryColorOffset > 0 && secondaryColor >= 0) { ... }
    if secondary_color_offset > 0 && secondary_color >= 0 {
        let q = secondary_color_offset;
        png[q as usize] = ishr(secondary_color, 16) as i8;
        png[q.wrapping_add(1) as usize] = ishr(secondary_color, 8) as i8;
        png[q.wrapping_add(2) as usize] = secondary_color as i8;
    }
    // int crc = crc32(png, chunkOffset + 4, chunkLen + 4);
    let crc: i32 = crc32(png, chunk_offset.wrapping_add(4), chunk_len.wrapping_add(4));
    // int crcPos = chunkOffset + 8 + chunkLen;
    let crc_pos: i32 = chunk_offset.wrapping_add(8).wrapping_add(chunk_len);
    // png[crcPos..+3] = big-endian crc
    png[crc_pos as usize] = ishr(crc, 24) as i8;
    png[crc_pos.wrapping_add(1) as usize] = ishr(crc, 16) as i8;
    png[crc_pos.wrapping_add(2) as usize] = ishr(crc, 8) as i8;
    png[crc_pos.wrapping_add(3) as usize] = crc as i8;
}

/// `private int crc32(byte[] data, int offset, int length)`
/// (`=> [iinc,iinc,ixor,iand,iushr,ixor,ixor]`) — standard CRC-32 (poly
/// 0xEDB88320) over `length` bytes from `offset`.
pub fn crc32(data: &[i8], offset: i32, length: i32) -> i32 {
    let table = crc_table();
    // int crc = -1;
    let mut crc: i32 = -1;
    let mut offset = offset;
    let mut length = length;
    loop {
        // int prev = crc;
        let prev: i32 = crc;
        // length--;
        length = length.wrapping_sub(1);
        // if (length < 0) return prev ^ (-1);
        if length < 0 {
            return prev ^ (-1);
        }
        // int pos = offset; offset++;
        let pos: i32 = offset;
        offset = offset.wrapping_add(1);
        // crc = crcTable[(prev ^ data[pos]) & 255] ^ (prev >>> 8);
        crc = table[((prev ^ (data[pos as usize] as i32)) & 255) as usize] ^ iushr(prev, 8);
    }
}

/// `public final int stringWidth(String str)` — null-safe; a char[] round-trip.
pub fn string_width(s: &BitmapFontState, str: Option<&[u16]>) -> i32 {
    // if (str == null) return 0;
    match str {
        None => 0,
        // char[] chars = str.toCharArray(); return stringWidth(chars, 0, chars.length);
        Some(chars) => string_width_range(s, chars, 0, chars.len() as i32),
    }
}

/// `public final int stringWidth(char[] chars, int start, int count)`
/// (`=> [iadd,isub,iadd,iinc]`).
pub fn string_width_range(s: &BitmapFontState, chars: &[u16], start: i32, count: i32) -> i32 {
    // int width = 0; int end = start + count;
    let mut width: i32 = 0;
    let end: i32 = start.wrapping_add(count);
    // for (int i = start; i < end; i++)
    let mut i: i32 = start;
    while i < end {
        // if (!isControl(chars[i])) width += charWidth(chars[i]) - this.kerning;
        if !is_control(s, chars[i as usize]) {
            width = width.wrapping_add(char_width(s, chars[i as usize]).wrapping_sub(s.kerning));
        }
        i = i.wrapping_add(1);
    }
    width
}

/// `public final int charWidth(char c)`
/// (`=> [iadd,i2c,isub,iadd,isub,isub,isub,i2c,isub,iadd,isub,isub]`).
// The OR-patterns (`224 | 225 | …`) transliterate the Java `switch`'s grouped
// `case` labels one-for-one; kept as such rather than collapsed to `x..=y` ranges.
#[allow(clippy::manual_range_patterns)]
pub fn char_width(s: &BitmapFontState, c: u16) -> i32 {
    let mut c: u16 = c;
    // if (c == 339) return 7;
    if (c as i32) == 339 {
        return 7;
    }
    // if (c <= ' ' || c >= 256) { if (c == ' ') return spaceWidth; return 0; }
    if (c as i32) <= 32 || (c as i32) >= 256 {
        if (c as i32) == 32 {
            return s.space_width;
        }
        return 0;
    }
    // boolean z = 192 <= c && 223 > c; boolean z2 = z;
    let z: bool = 192 <= (c as i32) && 223 > (c as i32);
    let z2: bool = z;
    // if (z) c = (char) (c + ' ');
    if z {
        c = (c as i32).wrapping_add(32) as u16;
    }
    // if (c > 127) switch (c) { ... }
    if (c as i32) > 127 {
        c = match c as i32 {
            161 => 129,
            176 => 176,
            191 => 130,
            223 => 127,
            224 | 225 | 226 | 227 | 228 | 229 => b'a' as u16,
            230 => {
                if z2 {
                    136
                } else {
                    138
                }
            }
            231 => {
                if z2 {
                    128
                } else {
                    137
                }
            }
            232 | 233 | 234 | 235 => b'e' as u16,
            236 | 237 | 238 | 239 => b'i' as u16,
            241 => b'n' as u16,
            242 | 243 | 244 | 245 | 246 => b'o' as u16,
            249 | 250 | 251 | 252 => b'u' as u16,
            _ => b'.' as u16,
        };
    }
    // if (c >= 136) switch (c) { 136->8, 137->5, 138->7, 176->4 }
    if (c as i32) >= 136 {
        match c as i32 {
            136 => return 8,
            137 => return 5,
            138 => return 7,
            176 => return 4,
            _ => {}
        }
    }
    // int width = glyphOffsets[(c - '!') + 1] - glyphOffsets[c - '!'];
    let mut width: i32 = (s.glyph_offsets[(c as i32).wrapping_sub(33).wrapping_add(1) as usize]
        as i32)
        .wrapping_sub(s.glyph_offsets[(c as i32).wrapping_sub(33) as usize] as i32);
    // if ((forceUppercase || z2) && c >= 'a' && c <= 'z') { char upper = (char)(c - ' '); width = glyphOffsets[(upper-'!')+1] - glyphOffsets[upper-'!']; }
    if (s.force_uppercase || z2) && (c as i32) >= (b'a' as i32) && (c as i32) <= (b'z' as i32) {
        let upper: u16 = (c as i32).wrapping_sub(32) as u16;
        width = (s.glyph_offsets[(upper as i32).wrapping_sub(33).wrapping_add(1) as usize] as i32)
            .wrapping_sub(s.glyph_offsets[(upper as i32).wrapping_sub(33) as usize] as i32);
    }
    width
}

/// `public final int lineHeightOf(char[] chars, int start, int count)`
/// (`=> [iadd,iinc,iadd]`).
pub fn line_height_of(s: &BitmapFontState, chars: &[u16], start: i32, count: i32) -> i32 {
    // int end = start + count;
    let end: i32 = start.wrapping_add(count);
    // for (int i = start; i < end; i++) if ("gjpqy,;_|ç¡¿".indexOf(chars[i]) != -1) return lineHeight;
    const DESCENDERS: [u16; 12] = [
        b'g' as u16,
        b'j' as u16,
        b'p' as u16,
        b'q' as u16,
        b'y' as u16,
        b',' as u16,
        b';' as u16,
        b'_' as u16,
        b'|' as u16,
        0x00E7, // ç
        0x00A1, // ¡
        0x00BF, // ¿
    ];
    let mut i: i32 = start;
    while i < end {
        if DESCENDERS.contains(&chars[i as usize]) {
            return s.line_height;
        }
        i = i.wrapping_add(1);
    }
    // return this.ascent + 1;
    s.ascent.wrapping_add(1)
}

/// `private boolean isControl(char c)` — the markup control chars `$ @ |` while
/// `hideControls`.
pub fn is_control(s: &BitmapFontState, c: u16) -> bool {
    // if (!this.hideControls) return false;
    if !s.hide_controls {
        return false;
    }
    // switch (c) { case '$': case '@': case '|': return true; default: return false; }
    matches!(c as i32, 36 | 64 | 124)
}

/// `public final int drawChars(Graphics graphics, char[] chars, int x, int y, int anchor)`
/// — the convenience overload over the whole array.
pub fn draw_chars(
    s: &BitmapFontState,
    graphics: &mut j2me_me::Graphics,
    chars: &[u16],
    x: i32,
    y: i32,
    anchor: i32,
) -> i32 {
    // return drawChars(graphics, chars, 0, chars.length, x, y, anchor);
    draw_chars_range(s, graphics, chars, 0, chars.len() as i32, x, y, anchor)
}

/// `public final int drawChars(Graphics graphics, char[] chars, int start, int count, int x, int y, int anchor)`
/// (`az.a:(…Graphics;[CIIIII)I`) — the core glyph blitter, transliterated
/// verbatim (accent/special-char resolution included, though the title's ASCII
/// strings never take those branches). Returns the total advance.
#[allow(clippy::too_many_arguments)]
pub fn draw_chars_range(
    s: &BitmapFontState,
    graphics: &mut j2me_me::Graphics,
    chars: &[u16],
    start: i32,
    count: i32,
    x: i32,
    y: i32,
    anchor: i32,
) -> i32 {
    let sheet = s.glyph_sheet.as_ref().expect("BitmapFont glyphSheet null");
    // int clipX = getClipX(); ...
    let clip_x = graphics.clip_x();
    let clip_y = graphics.clip_y();
    let clip_width = graphics.clip_width();
    let clip_height = graphics.clip_height();
    let mut x: i32 = x;
    let mut y: i32 = y;
    // if ((anchor & 1) != 0) x -= stringWidth(chars,start,count)/2;
    // else if ((anchor & 8) != 0) x -= stringWidth(chars,start,count);
    if (anchor & 1) != 0 {
        x = x.wrapping_sub(java_div(string_width_range(s, chars, start, count), 2).expect("sw/2"));
    } else if (anchor & 8) != 0 {
        x = x.wrapping_sub(string_width_range(s, chars, start, count));
    }
    // if ((anchor & 32) != 0) y -= lineHeightOf(chars,start,count);
    // else if ((anchor & 64) != 0) y -= this.ascent;
    if (anchor & 32) != 0 {
        y = y.wrapping_sub(line_height_of(s, chars, start, count));
    } else if (anchor & 64) != 0 {
        y = y.wrapping_sub(s.ascent);
    }
    // int advance = 0; int end = start + count;
    let mut advance: i32 = 0;
    let end: i32 = start.wrapping_add(count);
    // for (int idx = start; idx < end; idx++)
    let mut idx: i32 = start;
    while idx < end {
        // if (x > clipX + clipWidth) { setClip(...); return advance; }
        if x > clip_x.wrapping_add(clip_width) {
            graphics.set_clip(clip_x, clip_y, clip_width, clip_height);
            return advance;
        }
        // char c = chars[idx];
        let mut c: u16 = chars[idx as usize];
        // if (!isControl(c)) {
        if !is_control(s, c) {
            // int accentIndex = -1; int accentDx = 0;
            let mut accent_index: i32 = -1;
            let mut accent_dx: i32 = 0;
            // if (c == ' ') { x += spaceWidth; advance += spaceWidth; }
            if (c as i32) == 32 {
                x = x.wrapping_add(s.space_width);
                advance = advance.wrapping_add(s.space_width);
            } else {
                // boolean z = false;
                let mut z: bool = false;
                // if (c == 339) c = forceUppercase ? 139 : 140;
                if (c as i32) == 339 {
                    c = if s.force_uppercase { 139 } else { 140 };
                } else if (c as i32) > 32 && (c as i32) < 256 {
                    // boolean z2 = 192 <= c && 223 > c; z = z2;
                    let z2: bool = 192 <= (c as i32) && 223 > (c as i32);
                    z = z2;
                    // if (z2) c = (char)(c + ' ');
                    if z2 {
                        c = (c as i32).wrapping_add(32) as u16;
                    }
                    // if (c > 127) switch(c) { ...accent resolution... }
                    if (c as i32) > 127 {
                        match c as i32 {
                            161 => c = 129,
                            176 => {}
                            191 => c = 130,
                            223 => c = 127,
                            224 => {
                                c = b'a' as u16;
                                accent_index = 0;
                                accent_dx = 1;
                            }
                            225 => {
                                c = b'a' as u16;
                                accent_index = 1;
                                accent_dx = 1;
                            }
                            226 => {
                                c = b'a' as u16;
                                accent_index = 2;
                                accent_dx = 1;
                            }
                            227 => {
                                c = b'a' as u16;
                                accent_index = 3;
                                accent_dx = 1;
                            }
                            228 => {
                                c = b'a' as u16;
                                accent_index = 4;
                                accent_dx = 1;
                            }
                            229 => c = b'a' as u16,
                            230 => c = if s.force_uppercase || z { 136 } else { 138 },
                            231 => c = if s.force_uppercase || z { 128 } else { 137 },
                            232 => {
                                c = b'e' as u16;
                                accent_index = 0;
                                accent_dx = 1;
                            }
                            233 => {
                                c = b'e' as u16;
                                accent_index = 1;
                                accent_dx = 1;
                            }
                            234 => {
                                c = b'e' as u16;
                                accent_index = 2;
                                accent_dx = 1;
                            }
                            235 => {
                                c = b'e' as u16;
                                accent_index = 3;
                                accent_dx = 1;
                            }
                            236 => {
                                c = b'i' as u16;
                                accent_index = 0;
                                accent_dx = -1;
                            }
                            237 => {
                                c = b'i' as u16;
                                accent_index = 1;
                                accent_dx = 0;
                            }
                            238 => {
                                c = b'i' as u16;
                                accent_index = 2;
                                accent_dx = -1;
                            }
                            239 => {
                                c = b'i' as u16;
                                accent_index = 4;
                                accent_dx = -1;
                            }
                            241 => {
                                c = b'n' as u16;
                                accent_index = 3;
                                accent_dx = 1;
                            }
                            242 => {
                                c = b'o' as u16;
                                accent_index = 0;
                                accent_dx = 1;
                            }
                            243 => {
                                c = b'o' as u16;
                                accent_index = 1;
                                accent_dx = 1;
                            }
                            244 => {
                                c = b'o' as u16;
                                accent_index = 2;
                                accent_dx = 1;
                            }
                            245 => {
                                c = b'o' as u16;
                                accent_index = 3;
                                accent_dx = 1;
                            }
                            246 => {
                                c = b'o' as u16;
                                accent_index = 4;
                                accent_dx = 1;
                            }
                            249 => {
                                c = b'u' as u16;
                                accent_index = 0;
                                accent_dx = 1;
                            }
                            250 => {
                                c = b'u' as u16;
                                accent_index = 1;
                                accent_dx = 1;
                            }
                            251 => {
                                c = b'u' as u16;
                                accent_index = 2;
                                accent_dx = 1;
                            }
                            252 => {
                                c = b'u' as u16;
                                accent_index = 4;
                                accent_dx = 1;
                            }
                            _ => c = b'.' as u16,
                        }
                    }
                    // if ((forceUppercase || z) && c >= 'a' && c <= 'z') c = (char)(c - ' ');
                    if (s.force_uppercase || z)
                        && (c as i32) >= (b'a' as i32)
                        && (c as i32) <= (b'z' as i32)
                    {
                        c = (c as i32).wrapping_sub(32) as u16;
                    }
                }
                // short glyphX = 0; int glyphWidth = 0;
                let mut glyph_x: i16 = 0;
                let mut glyph_width: i32 = 0;
                let last: i32 = s.glyph_offsets.len() as i32 - 1;
                // switch (c) { special sheet-tail glyphs / bullet / default }
                let mut is_bullet = false;
                match c as i32 {
                    136 => {
                        glyph_x = s.glyph_offsets[last as usize];
                        glyph_width = 8;
                    }
                    137 => {
                        glyph_x = (s.glyph_offsets[last as usize] as i32).wrapping_add(8) as i16;
                        glyph_width = 5;
                    }
                    138 => {
                        glyph_x =
                            (s.glyph_offsets[last as usize] as i32).wrapping_add(8 + 5) as i16;
                        glyph_width = 7;
                    }
                    139 => {
                        glyph_x =
                            (s.glyph_offsets[last as usize] as i32).wrapping_add(8 + 5 + 7) as i16;
                        glyph_width = 7;
                    }
                    140 => {
                        glyph_x = (s.glyph_offsets[last as usize] as i32)
                            .wrapping_add(8 + 5 + 7 + 7) as i16;
                        glyph_width = 7;
                    }
                    176 => {
                        // graphics.setColor(bulletColor); setClip(x,y,4,4); drawArc(x,y,2,2,0,360); x+=4; advance+=4; continue;
                        graphics.set_color(s.bullet_color);
                        graphics.set_clip(x, y, 4, 4);
                        graphics.draw_arc(x, y, 2, 2, 0, 360);
                        x = x.wrapping_add(4);
                        advance = advance.wrapping_add(4);
                        is_bullet = true;
                    }
                    _ => {
                        // int glyphIndex = c - '!';
                        let glyph_index: i32 = (c as i32).wrapping_sub(33);
                        // if (glyphIndex >= 0 && glyphIndex < glyphOffsets.length - 1)
                        if glyph_index >= 0 && glyph_index < (s.glyph_offsets.len() as i32 - 1) {
                            glyph_x = s.glyph_offsets[glyph_index as usize];
                            glyph_width = (s.glyph_offsets[glyph_index.wrapping_add(1) as usize]
                                as i32)
                                .wrapping_sub(glyph_x as i32);
                        }
                    }
                }
                if !is_bullet {
                    // short accentX = hasAccents ? glyphOffsets[98 + accentIndex] : 0;
                    let accent_x: i16 = if s.has_accents {
                        s.glyph_offsets[(98i32.wrapping_add(accent_index)) as usize]
                    } else {
                        0
                    };
                    // int accentWidth = hasAccents ? glyphOffsets[(98+accentIndex)+1] - accentX : 0;
                    let accent_width: i32 = if s.has_accents {
                        (s.glyph_offsets
                            [(98i32.wrapping_add(accent_index).wrapping_add(1)) as usize]
                            as i32)
                            .wrapping_sub(accent_x as i32)
                    } else {
                        0
                    };
                    // if ((x + glyphWidth) - kerning < clipX) { x += glyphWidth - kerning; advance += glyphWidth - kerning; }
                    if x.wrapping_add(glyph_width).wrapping_sub(s.kerning) < clip_x {
                        x = x.wrapping_add(glyph_width.wrapping_sub(s.kerning));
                        advance = advance.wrapping_add(glyph_width.wrapping_sub(s.kerning));
                    } else {
                        // setClip(clipX,clipY,clipWidth,clipHeight);
                        graphics.set_clip(clip_x, clip_y, clip_width, clip_height);
                        // if (c != 'i' || accentIndex < 0) clipRect(x,y,glyphWidth,lineHeight); else clipRect(x,y+1,glyphWidth,lineHeight);
                        if (c as i32) != (b'i' as i32) || accent_index < 0 {
                            graphics.clip_rect(x, y, glyph_width, s.line_height);
                        } else {
                            graphics.clip_rect(x, y.wrapping_add(1), glyph_width, s.line_height);
                        }
                        // graphics.drawImage(glyphSheet, x - glyphX, y, 20);
                        graphics
                            .draw_image(sheet, x.wrapping_sub(glyph_x as i32), y, 20)
                            .expect("drawImage(glyphSheet)");
                        // if (accentIndex >= 0) { ...accent mark blit... }
                        if accent_index >= 0 {
                            graphics.set_clip(clip_x, clip_y, clip_width, clip_height);
                            let mark_width: i32 = if accent_index != 4 {
                                accent_width
                            } else {
                                accent_width.wrapping_add(1)
                            };
                            let raise: i32 = if s.force_uppercase || z { 2 } else { 0 };
                            if (c as i32) != (b'i' as i32) || accent_index < 0 {
                                graphics.clip_rect(
                                    x,
                                    y.wrapping_sub(1).wrapping_sub(raise),
                                    mark_width.wrapping_add(accent_dx),
                                    s.line_height.wrapping_add(raise),
                                );
                            } else {
                                graphics.clip_rect(
                                    x.wrapping_sub(1),
                                    y.wrapping_sub(1).wrapping_sub(raise),
                                    mark_width.wrapping_add(accent_dx),
                                    s.line_height.wrapping_add(raise),
                                );
                            }
                            graphics
                                .draw_image(
                                    sheet,
                                    x.wrapping_sub(accent_x as i32).wrapping_add(accent_dx),
                                    y.wrapping_sub(1).wrapping_sub(raise),
                                    20,
                                )
                                .expect("drawImage(accent)");
                        }
                        // x += glyphWidth - kerning; advance += glyphWidth - kerning;
                        x = x.wrapping_add(glyph_width.wrapping_sub(s.kerning));
                        advance = advance.wrapping_add(glyph_width.wrapping_sub(s.kerning));
                    }
                }
            }
        }
        idx = idx.wrapping_add(1);
    }
    // graphics.setClip(clipX, clipY, clipWidth, clipHeight);
    graphics.set_clip(clip_x, clip_y, clip_width, clip_height);
    advance
}
