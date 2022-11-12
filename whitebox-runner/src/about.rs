use crate::MyApp;
use egui_extras::RetainedImage;

pub struct WbLogo {
    // wb_logo: Option<egui::TextureHandle>,
    image: RetainedImage,
}

impl Default for WbLogo {
    fn default() -> Self {
        Self {
            image: RetainedImage::from_image_bytes(
                "../WBT_icon.png",
                include_bytes!("../WBT_icon.png"),
            )
            .unwrap(),
        }
    }
}

impl MyApp {

    pub fn about_window(&mut self, ctx: &egui::Context) {
        let mut close_dialog = false;
        egui::Window::new("About Whitebox Runner")
        .open(&mut self.about_visible)
        .resizable(false)
        .vscroll(true)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Add the logo here.
                self.wb_logo.image.show(ui);

                ui.heading("Whitebox Runner v2.0.0");
                ui.label("Developed by Dr. John Lindsay, Whitebox Geospatial Inc.");
                ui.label("© Whitebox Geospatial Inc. 2022-2023");
                ui.hyperlink("https://www.whiteboxgeo.com/");
                ui.label(" ");
                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
            });
        });

        if close_dialog {
            self.about_visible = false;
        }
    }
}