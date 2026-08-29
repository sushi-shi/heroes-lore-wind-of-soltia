//! Wrapped standard media — `.png` (12), `.mid` (20), `.wav` (9).
//!
//! These are ordinary media files carried inside the JAR. Phase 1 validates the
//! container magic and enough of the header to reject truncated/garbage input; it
//! does not decode pixels/notes/samples.
//!
//! - PNG: 8-byte signature `89 50 4E 47 0D 0A 1A 0A` then an `IHDR` chunk.
//! - MIDI: `MThd` header chunk (`4D 54 68 64`) then a `u32 BE` header length of 6.
//! - WAV: `RIFF` (`52 49 46 46`) …4-byte size… `WAVE` (`57 41 56 45`).

use crate::{FormatError, Reader};

/// Which media container was validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// PNG image.
    Png,
    /// Standard MIDI file.
    Midi,
    /// RIFF/WAVE audio.
    Wav,
}

/// Result of validating a media wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaInfo {
    /// The validated container kind.
    pub kind: MediaKind,
    /// Total byte length.
    pub len: usize,
}

/// Validate a `.png` wrapper.
pub fn parse_png(input: &[u8]) -> Result<MediaInfo, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);
    let sig = r.take(8, "png signature")?;
    if sig != crate::mpd::PNG_MAGIC {
        return Err(FormatError::BadMagic {
            what: "png signature",
        });
    }
    // First chunk must be IHDR (length 13, type "IHDR").
    let ihdr_len = r.u32_be("png IHDR length")?;
    let ctype = r.take(4, "png IHDR type")?;
    if ctype != b"IHDR" {
        return Err(FormatError::BadMagic { what: "png IHDR" });
    }
    if ihdr_len != 13 {
        return Err(FormatError::BadField {
            what: "png IHDR length != 13",
        });
    }
    r.skip(13, "png IHDR data")?;
    Ok(MediaInfo {
        kind: MediaKind::Png,
        len: input.len(),
    })
}

/// Validate a `.mid` wrapper.
pub fn parse_mid(input: &[u8]) -> Result<MediaInfo, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);
    let magic = r.take(4, "midi MThd")?;
    if magic != b"MThd" {
        return Err(FormatError::BadMagic { what: "midi MThd" });
    }
    let hdr_len = r.u32_be("midi header length")?;
    if hdr_len != 6 {
        return Err(FormatError::BadField {
            what: "midi MThd header length != 6",
        });
    }
    r.skip(6, "midi header body")?;
    Ok(MediaInfo {
        kind: MediaKind::Midi,
        len: input.len(),
    })
}

/// Validate a `.wav` wrapper.
pub fn parse_wav(input: &[u8]) -> Result<MediaInfo, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);
    let riff = r.take(4, "wav RIFF")?;
    if riff != b"RIFF" {
        return Err(FormatError::BadMagic { what: "wav RIFF" });
    }
    r.skip(4, "wav chunk size")?;
    let wave = r.take(4, "wav WAVE")?;
    if wave != b"WAVE" {
        return Err(FormatError::BadMagic { what: "wav WAVE" });
    }
    Ok(MediaInfo {
        kind: MediaKind::Wav,
        len: input.len(),
    })
}
