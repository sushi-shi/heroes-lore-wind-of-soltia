//! `heroes-lore-wind-of-soltia-linux` — the native host library: a windowless
//! game driver ([`host::GameHost`]), the `winit`/`softbuffer` window [`shell`], and
//! the frame-oracle [`capture`] route driver. The binary (`main.rs`) is a thin CLI
//! over this; the headless smoke test drives [`host::GameHost`] directly, with no
//! display.

pub mod capture;
pub mod frame;
pub mod host;
pub mod jar;
pub mod keymap;
pub mod shell;
