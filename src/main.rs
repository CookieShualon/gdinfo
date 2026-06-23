mod api;
mod icon_renderer;
mod models;
mod storage;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([640.0, 520.0])
            .with_min_inner_size([460.0, 380.0]),
        ..Default::default()
    };

    eframe::run_native(
        "GD Info",
        options,
        Box::new(|cc| Ok(Box::new(ui::GdInfoApp::new(cc)))),
    )
}
