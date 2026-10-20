#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod calc;
mod eval;
mod format;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Casio fx-82 SOLAR II")
            .with_inner_size([360.0, 640.0])
            .with_min_inner_size([270.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Casio fx-82 SOLAR II",
        options,
        Box::new(|cc| Ok(Box::new(ui::App::new(cc)))),
    )
}
