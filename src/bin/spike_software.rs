//! Legacy-Windows / software-renderer spike.
//!
//! Pure CPU stack: winit + softbuffer + `fast_image_resize`.
//! No wgpu, no GPU dependency. Targets `x86_64-win7-windows-msvc` Tier 3
//! for the Win7 legacy build via `--features legacy-windows`.
//!
//! Right-to-left reading mode: Left arrow = next, Right = prev, Space = next,
//! Home = first, End = last, Esc = quit. stderr logs each KeyDown→present.

use anyhow::Result;
use mangameeya_reborn::{archive::ZipPageSource, decode};
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

struct App {
    fixture: PathBuf,
    source: Option<ZipPageSource>,
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    _context: Option<Context<Arc<Window>>>,
    current_idx: usize,
    /// Decoded RGBA of current source page (un-resized).
    current_rgba: Option<image::RgbaImage>,
    /// Cached version pre-fitted to current window size; invalidated on
    /// page change or window resize.
    fitted_rgba: Option<image::RgbaImage>,
    fitted_for_size: Option<(u32, u32)>,
    last_key_time: Option<Instant>,
}

impl App {
    fn new(fixture: PathBuf) -> Self {
        Self {
            fixture,
            source: None,
            window: None,
            surface: None,
            _context: None,
            current_idx: 0,
            current_rgba: None,
            fitted_rgba: None,
            fitted_for_size: None,
            last_key_time: None,
        }
    }

    fn load_current(&mut self) -> Result<()> {
        let Some(source) = self.source.as_mut() else {
            return Ok(());
        };
        let bytes = source.page_bytes(self.current_idx)?;
        let img = decode::decode(&bytes)?;
        self.current_rgba = Some(img.to_rgba8());
        self.fitted_rgba = None;
        self.fitted_for_size = None;
        Ok(())
    }

    /// Resize current RGBA to fit current window, caching the result.
    fn ensure_fitted(&mut self, win_w: u32, win_h: u32) -> Result<()> {
        if self.fitted_for_size == Some((win_w, win_h)) && self.fitted_rgba.is_some() {
            return Ok(());
        }
        let Some(src) = self.current_rgba.as_ref() else {
            return Ok(());
        };
        let (sw, sh) = (src.width(), src.height());
        let scale = (win_w as f32 / sw as f32).min(win_h as f32 / sh as f32);
        let tw = ((sw as f32) * scale).max(1.0) as u32;
        let th = ((sh as f32) * scale).max(1.0) as u32;
        let dyn_img = image::DynamicImage::ImageRgba8(src.clone());
        let resized = decode::resize_lanczos3(&dyn_img, tw, th)?;
        self.fitted_rgba = Some(resized.to_rgba8());
        self.fitted_for_size = Some((win_w, win_h));
        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        let Some(window) = self.window.clone() else {
            return Ok(());
        };
        let size = window.inner_size();
        let (win_w, win_h) = (size.width.max(1), size.height.max(1));

        self.ensure_fitted(win_w, win_h)?;

        let Some(surface) = self.surface.as_mut() else {
            return Ok(());
        };
        surface
            .resize(
                NonZeroU32::new(win_w).expect("win_w nonzero"),
                NonZeroU32::new(win_h).expect("win_h nonzero"),
            )
            .map_err(|e| anyhow::anyhow!("softbuffer resize: {e:?}"))?;
        let mut buffer = surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("softbuffer buffer_mut: {e:?}"))?;

        // Background — dark grey
        let bg: u32 = 0x000d_0d0d;
        for px in buffer.iter_mut() {
            *px = bg;
        }

        if let Some(img) = self.fitted_rgba.as_ref() {
            let (iw, ih) = (img.width(), img.height());
            let pad_x = (win_w as i32 - iw as i32).max(0) / 2;
            let pad_y = (win_h as i32 - ih as i32).max(0) / 2;
            let raw = img.as_raw();
            for y in 0..ih {
                let dst_y = pad_y + y as i32;
                if dst_y < 0 || dst_y >= win_h as i32 {
                    continue;
                }
                let dst_row = (dst_y as usize) * (win_w as usize);
                let src_row = (y as usize) * (iw as usize) * 4;
                for x in 0..iw {
                    let dst_x = pad_x + x as i32;
                    if dst_x < 0 || dst_x >= win_w as i32 {
                        continue;
                    }
                    let s = src_row + (x as usize) * 4;
                    let r = raw[s] as u32;
                    let g = raw[s + 1] as u32;
                    let b = raw[s + 2] as u32;
                    // softbuffer expects 0x00RRGGBB
                    buffer[dst_row + dst_x as usize] = (r << 16) | (g << 8) | b;
                }
            }
        }

        buffer
            .present()
            .map_err(|e| anyhow::anyhow!("softbuffer present: {e:?}"))?;
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("spike_software — MangaMeeya Reborn legacy")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280u32, 800u32));
        let window = Arc::new(event_loop.create_window(attrs).expect("create_window"));
        let context = Context::new(window.clone()).expect("softbuffer Context");
        let surface = Surface::new(&context, window.clone()).expect("softbuffer Surface");
        let source = ZipPageSource::open(&self.fixture).expect("open fixture");
        eprintln!(
            "[spike_software] loaded {} pages from {}",
            source.page_count(),
            self.fixture.display()
        );
        self.window = Some(window);
        self._context = Some(context);
        self.surface = Some(surface);
        self.source = Some(source);
        self.load_current().expect("load page 0");
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => {
                self.fitted_rgba = None;
                self.fitted_for_size = None;
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        ..
                    },
                ..
            } => {
                self.last_key_time = Some(Instant::now());
                let count = self.source.as_ref().map(|s| s.page_count()).unwrap_or(0);
                if count == 0 {
                    return;
                }
                let new_idx = match logical_key {
                    Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::Space) => {
                        Some((self.current_idx + 1).min(count - 1))
                    }
                    Key::Named(NamedKey::ArrowRight) => self.current_idx.checked_sub(1),
                    Key::Named(NamedKey::Home) => Some(0),
                    Key::Named(NamedKey::End) => Some(count - 1),
                    Key::Named(NamedKey::Escape) => {
                        event_loop.exit();
                        return;
                    }
                    _ => None,
                };
                if let Some(idx) = new_idx
                    && idx != self.current_idx
                {
                    self.current_idx = idx;
                    if let Err(e) = self.load_current() {
                        eprintln!("[spike_software] load error: {e}");
                    }
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.render() {
                    eprintln!("[spike_software] render error: {e}");
                }
                if let Some(t) = self.last_key_time.take() {
                    let dt = t.elapsed().as_secs_f64() * 1000.0;
                    eprintln!(
                        "[spike_software] KeyDown→present: {:.3}ms (page {})",
                        dt, self.current_idx
                    );
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let fixture = PathBuf::from(
        args.get(1)
            .cloned()
            .or_else(|| std::env::var("MANGAMEEYA_BENCH_FIXTURE").ok())
            .unwrap_or_else(|| "bench-fixture.zip".into()),
    );
    eprintln!("[spike_software] fixture: {}", fixture.display());
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(fixture);
    event_loop.run_app(&mut app)?;
    Ok(())
}
