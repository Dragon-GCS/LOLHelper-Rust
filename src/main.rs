mod context;
mod errors;
mod lcu;
mod log;

use eframe::{
    App,
    egui::{self, FontData, FontDefinitions, FontId, RichText},
};

use lcu::LcuClient;
use log::init_logger;
use std::sync::Arc;

struct MyApp {
    name: String,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        // cc.egui_ctx.style_mut(|style| {
        //     style.override_font_id = Some(FontId::monospace(32.0));
        // });
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "msyh".to_owned(),
            Arc::new(FontData::from_static(include_bytes!("../MSYH.TTC"))),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "msyh".to_owned());

        cc.egui_ctx.set_fonts(fonts);
        // cc.egui_ctx.set_zoom_factor(1.5);

        Self {
            name: "My Egui App".to_string(),
        }
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::from(&self.name).font(FontId::proportional(24.0)));
            })
        });
    }
}

fn main() -> anyhow::Result<()> {
    init_logger();
    eframe::run_native(
        "My Egui App",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run eframe: {}", e))?;
    Ok(())
}
