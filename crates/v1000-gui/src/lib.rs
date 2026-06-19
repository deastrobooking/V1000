//! Editor application shell for V1000.
//!
//! As of M2 the app edits a [`Sequence`]: the preview reads frames from the
//! timeline (not a raw source), a minimal timeline strip shows the clips with a
//! click/drag-to-seek playhead, and the transport plays the sequence. With the
//! `ffmpeg` feature, "Open…" appends a decoded file as a new clip so you can see
//! multiple clips on the timeline; otherwise an animated test pattern plays.

use eframe::egui;

use v1000_codec::{FrameProducer, TestPatternSource};
use v1000_core::{Time, TimeCode};
use v1000_render::Transport;
use v1000_timeline::Sequence;

/// Top-level application state: a sequence being previewed.
pub struct V1000App {
    sequence: Sequence,
    transport: Transport,
    texture: Option<egui::TextureHandle>,
    /// Frame index (at the sequence timebase) currently uploaded, to skip
    /// redundant GPU uploads while the playhead sits on one frame.
    shown_frame: Option<i64>,
    has_frame: bool,
    status: String,
}

impl V1000App {
    /// Builds the app previewing a single-clip test-pattern sequence.
    pub fn new() -> Self {
        let sequence = Sequence::single(Box::new(TestPatternSource::default_preview()));
        let mut transport = Transport::new();
        transport.set_duration(sequence.duration());
        Self {
            sequence,
            transport,
            texture: None,
            shown_frame: None,
            has_frame: false,
            status: String::new(),
        }
    }

    /// Uploads the frame under the playhead to the GPU when it changes.
    fn sync_texture(&mut self, ctx: &egui::Context) {
        let frame_index = self
            .transport
            .playhead_time()
            .to_frame(self.sequence.timebase());
        if self.texture.is_some() && self.shown_frame == Some(frame_index) {
            return;
        }
        self.shown_frame = Some(frame_index);

        match self.sequence.frame_at(self.transport.playhead_time()) {
            Ok(Some(frame)) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(frame.size(), frame.pixels());
                match &mut self.texture {
                    Some(handle) => handle.set(image, egui::TextureOptions::LINEAR),
                    None => {
                        self.texture =
                            Some(ctx.load_texture("preview", image, egui::TextureOptions::LINEAR));
                    }
                }
                self.has_frame = true;
            }
            Ok(None) => self.has_frame = false, // gap under the playhead
            Err(e) => {
                self.has_frame = false;
                self.status = format!("decode error: {e}");
            }
        }
    }

    #[cfg(feature = "ffmpeg")]
    fn open_dialog(&mut self) {
        use v1000_codec::FrameSource;
        use v1000_core::Rational;

        let Some(path) = rfd::FileDialog::new()
            .add_filter("Video", &["mp4", "mov", "mkv", "webm", "avi", "m4v"])
            .pick_file()
        else {
            return;
        };
        match v1000_codec::FileDecoder::open(&path) {
            Ok(decoder) => {
                let (num, den) = decoder.fps();
                let rate = Rational::new(num as i64, den as i64);
                let duration = Time::from_frame(decoder.frame_count() as i64, rate);
                let id = self.sequence.add_media(Box::new(decoder));
                // Append after existing content so the timeline shows two clips.
                self.sequence.track_mut(0).append(id, Time::ZERO, duration);
                self.transport.set_duration(self.sequence.duration());
                self.shown_frame = None;
                self.status = format!("appended {}", path.display());
            }
            Err(e) => self.status = format!("open failed: {e}"),
        }
    }

    /// Ripple-deletes the clip on the top track under the playhead.
    fn ripple_delete_at_playhead(&mut self) {
        let t = self.transport.playhead_time();
        let last = self.sequence.tracks().len().saturating_sub(1);
        if let Some(i) = self.sequence.tracks()[last].clip_index_at(t) {
            self.sequence.track_mut(last).ripple_delete(i);
            self.transport.set_duration(self.sequence.duration());
            self.shown_frame = None;
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
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

    fn transport_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let play_label = if self.transport.is_playing() {
                "⏸"
            } else {
                "▶"
            };
            if ui
                .button(play_label)
                .on_hover_text("Play/Pause (Space)")
                .clicked()
            {
                self.transport.toggle();
            }
            if ui.button("⏮").on_hover_text("Go to start").clicked() {
                self.transport.seek_seconds(0.0);
            }
            if ui
                .button("✂ ripple-delete")
                .on_hover_text("Delete clip under playhead")
                .clicked()
            {
                self.ripple_delete_at_playhead();
            }

            let timebase = self.sequence.timebase();
            let now = TimeCode::from_time(self.transport.playhead_time(), timebase);
            let total = TimeCode::from_time(
                Time::from_seconds_f64(self.transport.duration_seconds()),
                timebase,
            );
            ui.monospace(format!("{now} / {total}"));
        });
    }

    /// Draws the clip lanes and the playhead; click/drag seeks.
    fn timeline_view(&mut self, ui: &mut egui::Ui) {
        let duration = self.transport.duration_seconds().max(1e-3);
        let lane_h = 26.0;
        let lanes = self.sequence.tracks().len().max(1) as f32;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), lane_h * lanes),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2.0, egui::Color32::from_gray(28));

        let x_of = |secs: f64| rect.left() + (secs / duration) as f32 * rect.width();

        // Topmost track on the top lane.
        for (row, track) in self.sequence.tracks().iter().rev().enumerate() {
            let y0 = rect.top() + row as f32 * lane_h;
            for (i, clip) in track.clips().iter().enumerate() {
                let clip_rect = egui::Rect::from_min_max(
                    egui::pos2(x_of(clip.timeline_start.as_seconds_f64()) + 1.0, y0 + 2.0),
                    egui::pos2(x_of(clip.end().as_seconds_f64()) - 1.0, y0 + lane_h - 2.0),
                );
                let fill = if i % 2 == 0 {
                    egui::Color32::from_rgb(60, 90, 130)
                } else {
                    egui::Color32::from_rgb(80, 70, 120)
                };
                painter.rect_filled(clip_rect, 2.0, fill);
                painter.rect_stroke(
                    clip_rect,
                    2.0,
                    egui::Stroke::new(1.0, egui::Color32::from_gray(180)),
                );
            }
        }

        // Playhead.
        let px = x_of(self.transport.playhead_seconds());
        painter.vline(
            px,
            rect.y_range(),
            egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 80, 80)),
        );

        if response.dragged() || response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let frac = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                self.transport.seek_fraction(frac as f64);
            }
        }
    }
}

impl Default for V1000App {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for V1000App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.stable_dt) as f64;
        self.transport.tick(dt);
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.transport.toggle();
        }

        self.sync_texture(ctx);

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| self.menu_bar(ui));

        egui::SidePanel::left("media_browser")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| ui.heading("Media"));

        egui::SidePanel::right("effects_panel")
            .resizable(true)
            .default_width(240.0)
            .show(ctx, |ui| ui.heading("Effects"));

        egui::TopBottomPanel::bottom("timeline")
            .resizable(true)
            .default_height(150.0)
            .show(ctx, |ui| {
                self.transport_bar(ui);
                ui.separator();
                self.timeline_view(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| match (&self.texture, self.has_frame) {
                (Some(handle), true) => {
                    ui.add(
                        egui::Image::from_texture(egui::load::SizedTexture::from_handle(handle))
                            .maintain_aspect_ratio(true)
                            .fit_to_exact_size(ui.available_size()),
                    );
                }
                _ => {
                    ui.label("no frame under playhead");
                }
            });
        });

        if self.transport.is_playing() {
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
