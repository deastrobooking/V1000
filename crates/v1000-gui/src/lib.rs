//! Editor application shell for V1000.
//!
//! At milestone M1 the central panel is a working preview player: it pulls the
//! current frame from a [`v1000_render::PreviewEngine`], uploads it to the GPU
//! through eframe's wgpu backend, and draws it, with transport controls
//! (play/pause, scrub, go-to-start) below. With the `ffmpeg` feature an
//! "Open…" button decodes a real file; otherwise an animated test pattern
//! plays.

use eframe::egui;

use v1000_codec::TestPatternSource;
use v1000_render::PreviewEngine;

/// Formats seconds as `M:SS.s`.
fn fmt_time(seconds: f64) -> String {
    let s = seconds.max(0.0);
    let minutes = (s / 60.0).floor() as u64;
    let rem = s - (minutes * 60) as f64;
    format!("{minutes}:{rem:04.1}")
}

/// Top-level application state: the preview player.
pub struct V1000App {
    engine: PreviewEngine,
    texture: Option<egui::TextureHandle>,
    /// Frame index currently uploaded, to skip redundant GPU uploads.
    shown_index: Option<u64>,
    status: String,
}

impl V1000App {
    /// Builds the app with the default animated test-pattern source.
    pub fn new() -> Self {
        let source = Box::new(TestPatternSource::default_preview());
        Self {
            engine: PreviewEngine::new(source),
            texture: None,
            shown_index: None,
            status: String::new(),
        }
    }

    /// Uploads the current frame to the GPU texture if the index changed.
    fn sync_texture(&mut self, ctx: &egui::Context) {
        let index = self.engine.current_index();
        if self.texture.is_some() && self.shown_index == Some(index) {
            return;
        }
        match self.engine.current_frame() {
            Ok(frame) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(frame.size(), frame.pixels());
                match &mut self.texture {
                    Some(handle) => handle.set(image, egui::TextureOptions::LINEAR),
                    None => {
                        self.texture =
                            Some(ctx.load_texture("preview", image, egui::TextureOptions::LINEAR));
                    }
                }
                self.shown_index = Some(index);
            }
            Err(e) => self.status = format!("decode error: {e}"),
        }
    }

    #[cfg(feature = "ffmpeg")]
    fn open_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Video", &["mp4", "mov", "mkv", "webm", "avi", "m4v"])
            .pick_file()
        else {
            return;
        };
        match v1000_codec::FileDecoder::open(&path) {
            Ok(decoder) => {
                self.engine.set_source(Box::new(decoder));
                self.shown_index = None;
                self.status = format!("opened {}", path.display());
            }
            Err(e) => self.status = format!("open failed: {e}"),
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                #[cfg(feature = "ffmpeg")]
                if ui.button("Open…").clicked() {
                    self.open_dialog();
                    ui.close_menu();
                }
                #[cfg(not(feature = "ffmpeg"))]
                ui.add_enabled(false, egui::Button::new("Open…"))
                    .on_disabled_hover_text("rebuild with --features ffmpeg to decode files");
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            if !self.status.is_empty() {
                ui.separator();
                ui.label(&self.status);
            }
        });
    }

    fn transport(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let play_label = if self.engine.is_playing() {
                "⏸"
            } else {
                "▶"
            };
            if ui
                .button(play_label)
                .on_hover_text("Play/Pause (Space)")
                .clicked()
            {
                self.engine.toggle();
            }
            if ui.button("⏮").on_hover_text("Go to start").clicked() {
                self.engine.seek_seconds(0.0);
            }

            let duration = self.engine.duration_seconds();
            let mut t = self.engine.playhead_seconds();
            let slider = egui::Slider::new(&mut t, 0.0..=duration.max(0.001)).show_value(false);
            if ui.add(slider).changed() {
                self.engine.seek_seconds(t);
            }

            ui.monospace(format!(
                "{} / {}   frame {}",
                fmt_time(t),
                fmt_time(duration),
                self.engine.current_index()
            ));
        });
    }
}

impl Default for V1000App {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for V1000App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Advance playback by the time since the last frame.
        let dt = ctx.input(|i| i.stable_dt) as f64;
        self.engine.tick(dt);

        // Space toggles play/pause.
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.engine.toggle();
        }

        self.sync_texture(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui));

        egui::SidePanel::left("media_browser")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| ui.heading("Media"));

        egui::SidePanel::right("effects_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| ui.heading("Effects"));

        egui::TopBottomPanel::bottom("transport")
            .resizable(false)
            .show(ctx, |ui| self.transport(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(handle) = &self.texture {
                ui.centered_and_justified(|ui| {
                    ui.add(
                        egui::Image::from_texture(egui::load::SizedTexture::from_handle(handle))
                            .maintain_aspect_ratio(true)
                            .fit_to_exact_size(ui.available_size()),
                    );
                });
            } else {
                ui.centered_and_justified(|ui| ui.label("no preview"));
            }
        });

        // Keep animating while playing.
        if self.engine.is_playing() {
            ctx.request_repaint();
        }
    }
}

/// Launches the editor window. Blocks until the window is closed.
///
/// # Errors
/// Returns any error from `eframe` while creating or running the window.
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("V1000")
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "V1000",
        options,
        Box::new(|_cc| Ok(Box::new(V1000App::new()))),
    )
}
