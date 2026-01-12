#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::Context;
use eframe::icon_data::from_png_bytes;
use eframe::{NativeOptions, egui::ViewportBuilder};
use lcu_helper::{app::MyApp, log::init_logger};

const APP_NAME: &str = "LOLHelper";
const WINDOW_SIZE: [f32; 2] = [1200.0, 600.0];

fn main() -> anyhow::Result<()> {
    init_logger();
    let icon = include_bytes!("../icon.png");
    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(format!("{APP_NAME} v{}", env!("CARGO_PKG_VERSION")))
            .with_inner_size(WINDOW_SIZE)
            .with_icon(from_png_bytes(icon).context("Failed to load")?)
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run eframe: {}", e))?;
    Ok(())
}
