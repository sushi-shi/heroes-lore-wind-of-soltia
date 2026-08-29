//! `j2me-me` — the reusable Java ME / MIDP **2D** device runtime for strict J2ME
//! transliterations, as idiomatic Rust whose *observable behavior* matches the
//! Java ME contract. **2D only** (no M3G).
//!
//! This is the shared `j2me-me` foundation adopted from the j2me home
//! `_template/` and extended, during the Heroes Lore: Wind of Soltia migration,
//! with the game-agnostic capabilities the template's runtime did not yet carry
//! (all add-only, all candidates to upstream back into `_template`):
//!
//! - [`graphics`] gains `drawArc` / `fillArc` (the MIDP ellipse-sector rasteriser)
//!   on top of the template's `setColor`/clip/`translate`/`fillRect`/`drawRect`/
//!   `drawLine`/`drawImage`/`drawRegion` + `GraphicsError`/`SpriteTransform`;
//! - [`canvas`] gains the [`Displayable`] surface trait on top of the template's
//!   `Canvas`/`Display` serial paint-input queue;
//! - [`media`] carries the richer MMAPI model with `VolumeControl`
//!   (`getControl`/`setLevel`/`getLevel`), a `PlayerListener` registration, and
//!   the `getState()` MMAPI integers — a superset of the template's simpler player
//!   state machine;
//! - [`rms`] carries the monotonic-record-id `RecordStore` (`getNextRecordID`,
//!   `getRecordSize`, offset/length-checked `addRecord`) — a superset of the
//!   template's record store;
//! - [`image`] is the `Image.createImage(byte[])` / `createImage(String)`
//!   PNG-decode factory the template's runtime lacks entirely.
//!
//! The ARGB pixel buffer itself lives in the neutral [`j2me_canvas`] crate;
//! `j2me-me` surfaces it as `javax.microedition.lcdui.Image`
//! (`Image::create_mutable` is the MIDP `createImage(int, int)` factory).

pub mod canvas;
pub mod graphics;
pub mod image;
pub mod media;
pub mod rms;

pub use canvas::{Canvas, CanvasEvent, Display, Displayable};
pub use graphics::{Graphics, GraphicsError, SpriteTransform};
pub use image::{create_image_named, create_image_region, ImageResources};
pub use j2me_canvas::{source_over, Argb, Image, ImageError};
pub use media::{HostAudioOp, MediaRuntime, PlayerId, PlayerState};
pub use rms::{RecordStore, RmsRuntime};
