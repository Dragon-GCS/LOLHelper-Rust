#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::Context;
use eframe::icon_data::from_png_bytes;
use eframe::{NativeOptions, egui::ViewportBuilder};

mod app;
mod log;

use app::MyApp;
use log::init_logger;

const APP_NAME: &str = "LOLHelper";
const WINDOW_SIZE: [f32; 2] = [1200.0, 600.0];

fn main() -> anyhow::Result<()> {
    init_logger();
    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(format!("{APP_NAME} v{}", env!("CARGO_PKG_VERSION")))
            .with_inner_size(WINDOW_SIZE)
            .with_icon(
                from_png_bytes(include_bytes!("../static/icon.png")).context("Failed to load")?,
            )
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
