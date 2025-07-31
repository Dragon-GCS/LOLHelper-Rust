use eframe::{NativeOptions, egui::ViewportBuilder};
use lcu_helper::{app::MyApp, log::init_logger};

const WINDOW_SIZE: [f32; 2] = [1200.0, 600.0];

fn main() -> anyhow::Result<()> {
    init_logger();

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(WINDOW_SIZE)
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "My Egui App",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run eframe: {}", e))?;
    Ok(())
}
