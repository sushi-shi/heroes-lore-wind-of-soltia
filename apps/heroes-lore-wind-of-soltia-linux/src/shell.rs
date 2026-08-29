//! The `winit` window + `softbuffer` presenter — the ONLY windowing code.
//!
//! It runs a `winit` event loop, drives [`GameHost`] once per frame (~50 ms, the
//! logo screen's own `setFps(20)` pacing), and blits the host's ARGB framebuffer to
//! the window with `softbuffer` (a plain CPU framebuffer→window copy; no
//! wgpu/shaders — this is 2D). Keyboard input is mapped to Nokia codes
//! ([`crate::keymap`]) and delivered to the host, which enqueues it on the `j2me-me`
//! serial queue (R9).
//!
//! This path needs a real display, so it is NOT exercised by the headless smoke; run
//! it on a machine with a display via `cargo run -p heroes-lore-wind-of-soltia-linux`.

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::host::{GameHost, InputEvent, H, W};
use crate::keymap::nokia_code;

/// The presenter's frame period (the logo screen runs at `setFps(20)` — ~50 ms).
const FRAME: Duration = Duration::from_millis(50);

/// Run the windowed host until the window is closed. Returns an error only if the
/// event loop itself fails to start (e.g. no display) — a loud failure, not a hang.
pub fn run(host: GameHost, scale: u32, title: String) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App {
        host,
        scale: scale.max(1),
        title,
        window: None,
        context: None,
        surface: None,
        pending: Vec::new(),
        next_frame: Instant::now(),
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App {
    host: GameHost,
    scale: u32,
    title: String,
    // Drop order (declaration order): surface, then context, then window — a
    // presenter is torn down before the surface handle it borrows.
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    context: Option<softbuffer::Context<Rc<Window>>>,
    window: Option<Rc<Window>>,
    pending: Vec<InputEvent>,
    next_frame: Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // already created (a second `resumed` after suspend)
        }
        let attrs = Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(
                (W as u32) * self.scale,
                (H as u32) * self.scale,
            ));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("heroes-lore-wind-of-soltia-linux: could not create a window: {e}");
                event_loop.exit();
                return;
            }
        };
        let context = match softbuffer::Context::new(window.clone()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("heroes-lore-wind-of-soltia-linux: softbuffer context failed: {e}");
                event_loop.exit();
                return;
            }
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("heroes-lore-wind-of-soltia-linux: softbuffer surface failed: {e}");
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window.clone());
        self.context = Some(context);
        self.surface = Some(surface);
        self.next_frame = Instant::now();
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                if code == KeyCode::Escape {
                    event_loop.exit();
                    return;
                }
                if repeat {
                    return; // the game reads discrete presses/releases, not autorepeat
                }
                if let Some(nokia) = nokia_code(code) {
                    self.pending.push(match state {
                        ElementState::Pressed => InputEvent::Press(nokia),
                        ElementState::Released => InputEvent::Release(nokia),
                    });
                }
            }

            WindowEvent::RedrawRequested => self.render(),

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame {
            self.next_frame = now + FRAME;
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Explicit teardown order (presenter → context → window) so nothing outlives
        // the handle it borrows.
        self.surface = None;
        self.context = None;
        self.window = None;
    }
}

impl App {
    /// Advance one game frame and present it, scaled, into the window.
    fn render(&mut self) {
        // Advance the game with the frame's accumulated input, moving the injected
        // clock forward by one frame period so game-time tracks wall-time.
        let inputs = std::mem::take(&mut self.pending);
        self.host.advance_clock(FRAME.as_millis() as i64);
        self.host.tick(&inputs);

        let (Some(window), Some(surface)) = (self.window.as_ref(), self.surface.as_mut()) else {
            return;
        };
        let size = window.inner_size();
        let (Some(win_w), Some(win_h)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return; // minimized / zero-area: nothing to present
        };
        if surface.resize(win_w, win_h).is_err() {
            return;
        }
        let mut buffer = match surface.buffer_mut() {
            Ok(b) => b,
            Err(_) => return,
        };
        present_scaled(
            self.host.frame().pixels(),
            &mut buffer,
            size.width,
            size.height,
        );
        let _ = buffer.present();
    }
}

/// Blit the `W×H` ARGB source into a `dst_w×dst_h` window buffer at the largest
/// integer scale that fits, centred, on a black ground. `softbuffer` reads each
/// `u32` as `0x00RRGGBB`, so the source's alpha byte is masked off (`& 0x00FF_FFFF`).
fn present_scaled(src: &[u32], dst: &mut [u32], dst_w: u32, dst_h: u32) {
    let (dw, dh) = (dst_w as i32, dst_h as i32);
    let scale = (dw / W).min(dh / H).max(1);
    let out_w = W * scale;
    let out_h = H * scale;
    let off_x = (dw - out_w) / 2;
    let off_y = (dh - out_h) / 2;

    for (i, px) in dst.iter_mut().enumerate() {
        let dx = (i as i32) % dw;
        let dy = (i as i32) / dw;
        let sx = (dx - off_x) / scale;
        let sy = (dy - off_y) / scale;
        *px = if dx >= off_x && dy >= off_y && sx < W && sy < H {
            src[(sy * W + sx) as usize] & 0x00FF_FFFF
        } else {
            0 // letterbox border
        };
    }
}
