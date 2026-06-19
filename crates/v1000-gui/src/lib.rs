//! Editor application shell for V1000.
//!
//! At milestone M0 this is the bare window: a menu bar and empty panels laid
//! out the way the finished editor will be (media browser, effects, preview,
//! timeline). Each panel is filled in as its milestone lands.

use eframe::egui;

/// Top-level application state. Holds nothing yet — project state arrives with
/// the timeline (M2).
#[derive(Default)]
pub struct V1000App;

impl eframe::App for V1000App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let _ = ui.button("New Project");
                    let _ = ui.button("Open Project…");
                    let _ = ui.button("Save");
                });
                ui.menu_button("Edit", |_ui| {});
                ui.menu_button("Help", |_ui| {});
            });
        });

        egui::SidePanel::left("media_browser")
            .resizable(true)
            .default_width(240.0)
            .show(ctx, |ui| ui.heading("Media"));

        egui::SidePanel::right("effects_panel")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| ui.heading("Effects"));

        egui::TopBottomPanel::bottom("timeline")
            .resizable(true)
            .default_height(220.0)
            .show(ctx, |ui| ui.heading("Timeline"));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| ui.label("Preview — M0 shell"));
        });
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
        Box::new(|_cc| Ok(Box::<V1000App>::default())),
    )
}
