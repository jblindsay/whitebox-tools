use crate::MyApp;

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
                // ui.add(egui::Image::new(texture, texture.size_vec2()));
                ui.heading("Whitebox Runner v2.0.0");
                ui.label("Developed by Dr. John Lindsay, Whitebox Geospatial Inc.");
                ui.label("(c) Whitebox Geospatial Inc. 2022-2023");
                ui.hyperlink("https://www.whiteboxgeo.com/");
                ui.label(" ");
                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
                ui.end_row();
            });
        });

        if close_dialog {
            self.about_visible = false;
        }
    }
}