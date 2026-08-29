//! Framebuffer analysis shared by the CLI headless run and the smoke test, so both
//! judge "a real frame" by exactly the same rule (the same rule `tests/first_frame.rs`
//! uses in the transliteration crate: distinct-colour richness + not one flat fill +
//! not all-white).

use std::collections::{HashMap, HashSet};

use j2me_me::Image;

/// `Image::create_mutable` fills opaque white; an unpainted (skipped) frame keeps it.
const WHITE: u32 = 0xFFFF_FFFF;

/// Coarse pixel statistics of a painted frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameStats {
    /// Distinct ARGB values in the frame.
    pub distinct: usize,
    /// Pixels equal to the unpainted initial white.
    pub white: usize,
    /// The most common single colour's pixel count (the dominant fill).
    pub dominant: usize,
    /// Total pixels.
    pub total: usize,
}

/// Measure a framebuffer.
pub fn analyze(img: &Image) -> FrameStats {
    let px = img.pixels();
    let total = px.len();
    let distinct: HashSet<u32> = px.iter().copied().collect();
    let mut counts: HashMap<u32, usize> = HashMap::new();
    let mut white = 0usize;
    for &p in px {
        *counts.entry(p).or_insert(0) += 1;
        if p == WHITE {
            white += 1;
        }
    }
    let dominant = counts.values().copied().max().unwrap_or(0);
    FrameStats {
        distinct: distinct.len(),
        white,
        dominant,
        total,
    }
}

impl FrameStats {
    /// A real, non-blank frame: many distinct colours and not a single flat fill.
    /// A blank framebuffer (all white / one colour) has `distinct == 1` and
    /// `dominant == total`, failing both clauses — the proven-red control.
    ///
    /// This does NOT require `white == 0`: the title/logo frame is drawn over a
    /// white clear, so most of the frame is legitimately white. Distinct-colour
    /// richness is the discriminator; `dominant < total` rejects a uniform frame.
    pub fn is_real_frame(&self) -> bool {
        self.distinct >= 8 && self.dominant < self.total
    }
}
