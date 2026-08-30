//! Transliterated from `java/src/main/java/defpackage/WrapFont.java`
//! (original `b.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! `WrapFont extends BitmapFont`, adding only greedy word-wrapping (`wrap` /
//! `wrapInto`) and the `create` factories `FontManager` builds its fonts through.
//! The subclass declares **no fields** — a `WrapFont` instance is structurally a
//! [`BitmapFontState`], so the factories construct one directly (and the two
//! wrapping methods take that `BitmapFontState` as `&self`).
//!
//! [`wrap`] / [`wrap_into`] are the greedy, space-aware word-wrapping core the
//! whole `FontManager` wrapped-text family (`wrapLines` → every `drawWrapped*` /
//! `lineCount` / `measureBlockHeight`) is built on: fill a line char-by-char until
//! its pixel width reaches the budget, then back up to the last space (or break
//! mid-word only when a single word overflows), honouring embedded `'\n'`.
//!
//! Opcode shapes (R8): `b.<init>:(Ljava/lang/String;IIZ)V => []`,
//! `b.a:(Ljava/lang/String;IIZ)Laz; => []` (create),
//! `b.a:(Ljava/lang/String;IZ)Laz; => []` (create single-colour),
//! `b.a:(Ljava/lang/String;II)I => [iadd,isub,iadd,iadd,iadd,iinc,iinc,iinc,iinc,iadd,iinc,iadd,isub,iinc,iadd]` (wrap),
//! `b.a:(Ljava/util/Vector;Ljava/lang/String;I)Ljava/util/Vector; => [iadd,isub,isub,iadd,iadd,iinc,isub,iinc,iinc,iadd,isub,iadd,isub,isub]` (wrapInto)
//! — their full multisets are reproduced by the transliteration below.

use crate::bitmap_font::{self, BitmapFontState};
use crate::game::Game;

/// `public static final BitmapFont create(String name, int primaryColor, int secondaryColor, boolean forceUppercase)`
/// — `return new WrapFont(name, primaryColor, secondaryColor, forceUppercase);`.
/// The private `WrapFont(...)` constructor is just `super(...)` (BitmapFont.load).
pub fn create(
    g: &mut Game,
    name: &str,
    primary_color: i32,
    secondary_color: i32,
    force_uppercase: bool,
) -> BitmapFontState {
    bitmap_font::construct(g, name, primary_color, secondary_color, force_uppercase)
}

/// `public static final BitmapFont create(String name, int color, boolean forceUppercase)`
/// — `return create(name, color, -1, forceUppercase);`.
pub fn create_single(
    g: &mut Game,
    name: &str,
    color: i32,
    force_uppercase: bool,
) -> BitmapFontState {
    create(g, name, color, -1, force_uppercase)
}

/// `String.indexOf(int ch)` restricted to the single `str.indexOf(10)` call in
/// `wrap` / `wrapInto`: the index of the first `ch` in `str`, or `-1`.
fn index_of_char(str: &[u16], ch: i32) -> i32 {
    let mut i: i32 = 0;
    while (i as usize) < str.len() {
        if (str[i as usize] as i32) == ch {
            return i;
        }
        i = i.wrapping_add(1);
    }
    -1
}

/// `new String(char[] value, int offset, int count)` — the substring copy the
/// wrap emits per line. Panics on a bad range exactly as the JVM's `StringIndex-
/// OutOfBoundsException` would (the reached call sites — `wrapInto` — are not
/// guarded, so an uncaught throw is faithful).
fn substring(chars: &[u16], offset: i32, count: i32) -> Vec<u16> {
    chars[offset as usize..(offset.wrapping_add(count)) as usize].to_vec()
}

/// `public final int wrap(String str, int width, int maxLines)`
/// (`b.a:(Ljava/lang/String;II)I`) — how many characters of `str` fit within pixel
/// `width` over at most `maxLines` lines (a fast path returns the whole length for
/// a short single-line string). Breaks at the last space before an overflow, or
/// mid-word when a word is itself too wide. `str` is never null on the reached
/// paths (callers build it via `new String(...)`), so the `str == null ? 0`
/// branch degenerates to `str.length()`.
pub fn wrap(font: &BitmapFontState, str: &[u16], width: i32, max_lines: i32) -> i32 {
    // int length = str == null ? 0 : str.length(); int total = length;
    let length: i32 = str.len() as i32;
    let total: i32 = length;
    // if (length == 0) return 0;
    if length == 0 {
        return 0;
    }
    // char[] chars = str.toCharArray();
    let chars: &[u16] = str;
    // if (total < 64 && str.indexOf(10) == -1 && stringWidth(chars,0,total) <= width) return str.length();
    if total < 64
        && index_of_char(str, 10) == -1
        && bitmap_font::string_width_range(font, chars, 0, total) <= width
    {
        return str.len() as i32;
    }
    // int lineStart = 0; int pos = 0;
    let mut line_start: i32 = 0;
    let mut pos: i32 = 0;
    let mut max_lines: i32 = max_lines;
    // while (maxLines > 0)
    while max_lines > 0 {
        // int rowStart = lineStart; int rowWidth = 0; int span = 1;
        let row_start: i32 = line_start;
        let mut row_width: i32 = 0;
        let mut span: i32 = 1;
        // while (maxLines > 0)
        while max_lines > 0 {
            // int probe = (rowStart + span) - 1; pos = probe;
            let probe: i32 = row_start.wrapping_add(span).wrapping_sub(1);
            pos = probe;
            // if (probe < total)
            if probe < total {
                // char c = chars[pos]; rowWidth += charWidth(c);
                let c: u16 = chars[pos as usize];
                row_width = row_width.wrapping_add(bitmap_font::char_width(font, c));
                // if (c != '\n')
                if (c as i32) != 10 {
                    // if (rowWidth >= width)
                    if row_width >= width {
                        // while (pos > rowStart && chars[pos] != ' ') pos--;
                        while pos > row_start && (chars[pos as usize] as i32) != 32 {
                            pos = pos.wrapping_sub(1);
                        }
                        // if (chars[pos] != ' ') { lineStart = (lineStart + span) - 1; maxLines--; break; }
                        if (chars[pos as usize] as i32) != 32 {
                            line_start = line_start.wrapping_add(span).wrapping_sub(1);
                            max_lines = max_lines.wrapping_sub(1);
                            break;
                        }
                        // lineStart = pos + 1; maxLines--; break;
                        line_start = pos.wrapping_add(1);
                        max_lines = max_lines.wrapping_sub(1);
                        break;
                    }
                    // span++;
                    span = span.wrapping_add(1);
                } else {
                    // lineStart = pos + 1;
                    line_start = pos.wrapping_add(1);
                    // while (lineStart < total && chars[lineStart] == ' ') lineStart++;
                    while line_start < total && (chars[line_start as usize] as i32) == 32 {
                        line_start = line_start.wrapping_add(1);
                    }
                    // maxLines--; break;
                    max_lines = max_lines.wrapping_sub(1);
                    break;
                }
            } else {
                // return pos + 1;
                return pos.wrapping_add(1);
            }
        }
    }
    // return pos + 1;
    pos.wrapping_add(1)
}

/// `public final Vector wrapInto(Vector out, String str, int width)`
/// (`b.a:(Ljava/util/Vector;Ljava/lang/String;I)Ljava/util/Vector;`) — splits `str`
/// into wrapped lines for pixel `width`, appending each to `out` (which it first
/// clears). Java returns `out`; the sole caller (`FontManager.wrapLines`) already
/// holds that vector (`wrapCache`) and reads it directly, so this returns nothing.
/// `str` is never null on the reached paths.
pub fn wrap_into(font: &BitmapFontState, out: &mut Vec<Vec<u16>>, str: &[u16], width: i32) {
    // out.removeAllElements();
    out.clear();
    // int length = str == null ? 0 : str.length(); int total = length;
    let length: i32 = str.len() as i32;
    let total: i32 = length;
    // if (length == 0) return out;
    if length == 0 {
        return;
    }
    // char[] chars = str.toCharArray();
    let chars: &[u16] = str;
    // if (total < 64 && str.indexOf(10) == -1 && stringWidth(chars,0,total) <= width) { out.addElement(str); return out; }
    if total < 64
        && index_of_char(str, 10) == -1
        && bitmap_font::string_width_range(font, chars, 0, total) <= width
    {
        out.push(str.to_vec());
        return;
    }
    // int lineStart;  (declared outside the loop — its post-loop value drives the
    // tail; assigned on every iteration before the loop can exit, as in the Java)
    let mut line_start: i32;
    // int cursor = 0;
    let mut cursor: i32 = 0;
    // loop0: while (true)
    'loop0: loop {
        // lineStart = cursor; int rowWidth = 0; int span = 1;
        line_start = cursor;
        let mut row_width: i32 = 0;
        let mut span: i32 = 1;
        // while (true)
        loop {
            // int probe = (lineStart + span) - 1; int pos = probe;
            let probe: i32 = line_start.wrapping_add(span).wrapping_sub(1);
            let mut pos: i32 = probe;
            // if (probe < total)
            if probe < total {
                // char c = chars[pos]; rowWidth += charWidth(c);
                let c: u16 = chars[pos as usize];
                row_width = row_width.wrapping_add(bitmap_font::char_width(font, c));
                // if (c != '\n')
                if (c as i32) != 10 {
                    // if (rowWidth >= width)
                    if row_width >= width {
                        // while (pos > lineStart && chars[pos] != ' ') pos--;
                        while pos > line_start && (chars[pos as usize] as i32) != 32 {
                            pos = pos.wrapping_sub(1);
                        }
                        // if (chars[pos] != ' ') { cursor = (cursor + span) - 1; out.addElement(new String(chars, lineStart, cursor - lineStart)); break; }
                        if (chars[pos as usize] as i32) != 32 {
                            cursor = cursor.wrapping_add(span).wrapping_sub(1);
                            out.push(substring(
                                chars,
                                line_start,
                                cursor.wrapping_sub(line_start),
                            ));
                            break;
                        }
                        // cursor = pos + 1; out.addElement(new String(chars, lineStart, pos - lineStart)); break;
                        cursor = pos.wrapping_add(1);
                        out.push(substring(chars, line_start, pos.wrapping_sub(line_start)));
                        break;
                    }
                    // span++;
                    span = span.wrapping_add(1);
                } else {
                    // cursor = pos + 1;
                    cursor = pos.wrapping_add(1);
                    // while (cursor < total && chars[cursor] == ' ') cursor++;
                    while cursor < total && (chars[cursor as usize] as i32) == 32 {
                        cursor = cursor.wrapping_add(1);
                    }
                    // out.addElement(new String(chars, lineStart, pos - lineStart)); break;
                    out.push(substring(chars, line_start, pos.wrapping_sub(line_start)));
                    break;
                }
            } else {
                // break loop0;
                break 'loop0;
            }
        }
    }
    // if (lineStart < total) out.addElement(new String(chars, lineStart, total - lineStart));
    if line_start < total {
        out.push(substring(chars, line_start, total.wrapping_sub(line_start)));
    }
    // return out;  (the caller already holds `out`)
}
