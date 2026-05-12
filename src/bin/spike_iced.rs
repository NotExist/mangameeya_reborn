//! Phase 2 spike — Iced + image widget.
//!
//! Same flow as spike_gpu but UI is Iced. Useful for comparing Iced framework
//! overhead against raw winit+wgpu (Phase 1b) and for validating CJK / IME
//! handling.
//!
//! Right-to-left reading mode: Left arrow = next, Right = prev, Space = next,
//! Home = first, End = last, Esc = quit. Status bar shows current filename so
//! CJK rendering is visible.

use iced::keyboard::key::Named;
use iced::keyboard::{self, Key};
use iced::widget::{column, container, image, text};
use iced::{Element, Length, Subscription, Task};
use mangameeya_reborn::{archive::ZipPageSource, decode};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

struct Reader {
    fixture: PathBuf,
    source: Option<Mutex<ZipPageSource>>,
    page_count: usize,
    current_idx: usize,
    current_handle: Option<image::Handle>,
    status: String,
    last_key_time: Option<Instant>,
}

#[derive(Debug, Clone)]
enum Message {
    LoadInitial,
    KeyPressed(Key),
}

impl Reader {
    fn new() -> (Self, Task<Message>) {
        let fixture = PathBuf::from(
            std::env::args()
                .nth(1)
                .or_else(|| std::env::var("MANGAMEEYA_BENCH_FIXTURE").ok())
                .unwrap_or_else(|| "bench-fixture.zip".into()),
        );
        (
            Self {
                fixture,
                source: None,
                page_count: 0,
                current_idx: 0,
                current_handle: None,
                status: "loading…".into(),
                last_key_time: None,
            },
            Task::done(Message::LoadInitial),
        )
    }

    fn load_page(&mut self, idx: usize) {
        let Some(source) = self.source.as_ref() else {
            return;
        };
        let mut s = source.lock().expect("source mutex");
        let bytes = match s.page_bytes(idx) {
            Ok(b) => b,
            Err(e) => {
                self.status = format!("load error: {}", e);
                return;
            }
        };
        let img = match decode::decode(&bytes) {
            Ok(i) => i,
            Err(e) => {
                self.status = format!("decode error: {}", e);
                return;
            }
        };
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        let handle = image::Handle::from_rgba(w, h, rgba.into_raw());
        self.current_handle = Some(handle);
        let name = s.entry_name(idx).to_string();
        self.current_idx = idx;
        self.status = format!("{} / {}  {}", idx + 1, self.page_count, name);
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::LoadInitial => {
                match ZipPageSource::open(&self.fixture) {
                    Ok(s) => {
                        self.page_count = s.page_count();
                        self.source = Some(Mutex::new(s));
                        self.load_page(0);
                    }
                    Err(e) => {
                        self.status = format!("fixture error: {}", e);
                    }
                }
                Task::none()
            }
            Message::KeyPressed(key) => {
                let key_t0 = Instant::now();
                self.last_key_time = Some(key_t0);
                if self.page_count == 0 {
                    return Task::none();
                }
                let new_idx = match key {
                    Key::Named(Named::ArrowLeft) | Key::Named(Named::Space) => {
                        Some((self.current_idx + 1).min(self.page_count - 1))
                    }
                    Key::Named(Named::ArrowRight) => self.current_idx.checked_sub(1),
                    Key::Named(Named::Home) => Some(0),
                    Key::Named(Named::End) => Some(self.page_count - 1),
                    Key::Named(Named::Escape) => {
                        return iced::exit();
                    }
                    _ => None,
                };
                if let Some(idx) = new_idx
                    && idx != self.current_idx
                {
                    self.load_page(idx);
                    let dt = key_t0.elapsed().as_secs_f64() * 1000.0;
                    eprintln!(
                        "[spike_iced] KeyDown→state-update: {:.3}ms (page {})",
                        dt, self.current_idx
                    );
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let body: Element<_> = if let Some(handle) = &self.current_handle {
            image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            text("loading…").into()
        };
        column![
            container(body).width(Length::Fill).height(Length::Fill),
            container(text(self.status.clone()).size(14))
                .padding(6)
                .width(Length::Fill),
        ]
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::on_key_press(|key, _mods| Some(Message::KeyPressed(key)))
    }
}

fn main() -> iced::Result {
    iced::application("spike_iced", Reader::update, Reader::view)
        .subscription(Reader::subscription)
        .window_size((1920.0, 1080.0))
        .run_with(Reader::new)
}
