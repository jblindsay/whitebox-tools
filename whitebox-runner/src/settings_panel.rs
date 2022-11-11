use crate::MyApp;
use crate::toggle;
use crate::AppTheme;
// use egui::FontFamily::Proportional;
// use egui::FontId;
// use egui::TextStyle::*;

impl MyApp {
    pub fn settings_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("settings_panel").show(ctx, |ui| {
            ui.heading("⛭ Settings"); // ⚙
            egui::Grid::new("my_grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .striped(true)
                .show(ui, |ui| {

                    // Whitebox Runner settings
                    ui.label(egui::RichText::new("Whitebox Runner Settings:")
                    .italics()
                    .strong()
                    .color(ui.visuals().hyperlink_color));
                    ui.end_row();

                    // Working directory
                    ui.label("Working directory:");
                    ui.horizontal(|ui| {
                        // ui.text_edit_singleline(&mut self.state.working_dir);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.state.working_dir)
                            .desired_width(self.state.textbox_width)
                        );
                        if ui.button("…").clicked() {
                            if let Some(path) = rfd::FileDialog::new().set_directory(std::path::Path::new(&self.state.working_dir)).pick_folder() {
                                self.state.working_dir = path.display().to_string();
                            }
                        }
                    });
                    ui.end_row();

                    if ui.visuals().dark_mode {
                        ui.label("Switch to light mode:");
                        if ui.button("☀ Light").clicked() {
                            self.theme_changed = true;
                            self.state.theme = AppTheme::Light;
                        }
                    } else {
                        ui.label("Switch to dark mode:");
                        if ui.button("🌙 Dark").clicked() {
                            self.theme_changed = true;
                            self.state.theme = AppTheme::Dark;
                        }
                    }
                    ui.end_row();

                    // Font sizes

                    self.state.body_font_size = self.state.body_font_size.clamp(6.0, 30.0);
                    ui.label("Body font size:");
                    if ui.add(egui::DragValue::new(&mut self.state.body_font_size).speed(0).fixed_decimals(1)).lost_focus() {
                        self.state.body_font_size = self.state.body_font_size.clamp(6.0, 30.0);
                        self.fonts_changed = true;
                    }
                    ui.end_row();
                    
                    self.state.header_font_size = self.state.header_font_size.clamp(10.0, 36.0);
                    ui.label("Header font size:");
                    if ui.add(egui::DragValue::new(&mut self.state.header_font_size).speed(0).fixed_decimals(1)).lost_focus() {
                        self.state.header_font_size = self.state.header_font_size.clamp(10.0, 36.0);
                        self.fonts_changed = true;
                    }
                    ui.end_row();

                    // Update fonts
                    // Get current context style
                    // let mut style = (*ctx.style()).clone();

                    // // Redefine text_styles
                    // style.text_styles = [
                    // (Heading, FontId::new(self.state.header_font_size, Proportional)),
                    // // (Name("Heading2".into()), FontId::new(18.0, Proportional)),
                    // // (Name("Context".into()), FontId::new(14.0, Proportional)),
                    // (Body, FontId::new(self.state.body_font_size, Proportional)),
                    // (Monospace, FontId::new(self.state.body_font_size, egui::FontFamily::Monospace)),
                    // (Button, FontId::new(self.state.body_font_size, Proportional)),
                    // (Small, FontId::new(10.0, Proportional)),
                    // ].into();

                    // // Mutate global style with above changes
                    // ctx.set_style(style);

                    // Textbox width
                    self.state.textbox_width = self.state.textbox_width.clamp(100.0, 500.0);
                    ui.label("Textbox width:");
                    ui.add(egui::DragValue::new(&mut self.state.textbox_width).speed(0).fixed_decimals(0));
                    ui.end_row();
                    self.state.textbox_width = self.state.textbox_width.clamp(100.0, 500.0);

                    // Print command line statements
                    ui.label("Print command-line statements?");
                    let resp = ui.add(toggle(&mut self.state.output_command));
                    if resp.clicked() {
                        for i in 0..self.tool_info.len() {
                            self.tool_info[i].update_output_command(self.state.output_command);
                        }
                    }
                    ui.end_row();

                    // Reset button
                    ui.label("Reset settings:");
                    if ui.button("🔃 Reset").on_hover_text("Reset Whitebox Runner settings").clicked() {
                        self.state.theme = AppTheme::Dark;
                        // self.state.settings_visible: bool,
                        self.state.body_font_size = 14.0;
                        self.state.header_font_size = 18.0;
                        // self.state.whitebox_exe: String,
                        self.state.working_dir = "/".to_string();
                        self.state.view_tool_output = true;
                        self.state.max_procs = -1;
                        self.state.compress_rasters = true;
                        self.state.textbox_width = 230.0;
                        self.state.output_command = false;
                        self.state.show_toolboxes = true;
                        self.state.show_tool_search = false;
                        self.state.show_recent_tools = false;
                        self.state.most_recent = std::collections::VecDeque::new();
                    }
                    ui.end_row();


                    
                    // WhiteboxTools Settings
                    ui.label(egui::RichText::new("WhiteboxTools Settings:")
                    .italics()
                    .strong()
                    .color(ui.visuals().hyperlink_color));
                    ui.end_row();

                    // ui.separator();
                    ui.label("WhiteboxTools executable:");
                    ui.horizontal(|ui| {
                        // ui.text_edit_singleline(&mut self.state.whitebox_exe);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.state.whitebox_exe)
                            .desired_width(self.state.textbox_width)
                        );
                        if ui.button("…").clicked() {
                            if let Some(path) = rfd::FileDialog::new().set_directory(std::path::Path::new(&self.state.whitebox_exe)).pick_file() {
                                self.state.whitebox_exe = path.display().to_string();
                                _ = self.get_tool_info();
                            }
                        }
                    });
                    ui.end_row();

                    // Refresh tools
                    ui.label("Refresh tools now:");
                    if ui.button("🛠 Refresh").clicked() {
                        // self.get_tool_info();
                        self.refesh_tools();
                    }
                    ui.end_row();

                    // Version
                    ui.label("WhiteboxTools Version:");
                    ui.label(&self.wbt_version.replace("by Dr. John B. Lindsay (c)", "(c) J. Lindsay").replace("(c) Dr. John Lindsay", "(c) J. Lindsay")); // too long
                    ui.end_row();
                    
                    // Num CPUs
                    ui.label("Max. number of processors: ");
                    // ui.text_edit_singleline(&mut self.max_procs_str);
                    // ui.add(egui::Slider::new(&mut self.max_procs_str, 0.0..=360.0));
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut self.state.max_procs).speed(0)).lost_focus() {
                            _ = self.set_max_procs();
                        }
                        ui.label("(-1 indicates all available processors)");
                    });
                    ui.end_row();

                    // Compress rasters
                    ui.label("Print tool output (Verbose mode)?");
                    let resp = ui.add(toggle(&mut self.state.view_tool_output));
                    if resp.clicked() {
                        for i in 0..self.tool_info.len() {
                            self.tool_info[i].update_verbose_mode(self.state.view_tool_output);
                        }
                    }
                    ui.end_row();

                    // Compress rasters
                    ui.label("Compress output rasters?");
                    let resp = ui.add(toggle(&mut self.state.compress_rasters));
                    if resp.clicked() {
                        for i in 0..self.tool_info.len() {
                            self.tool_info[i].update_compress_rasters(self.state.compress_rasters);
                        }
                    }
                    ui.end_row();

                    
                    // Extensions
                    ui.label(egui::RichText::new("Extension Settings:")
                    .italics()
                    .strong()
                    .color(ui.visuals().hyperlink_color));
                    ui.end_row();

                    ui.label("Installed extensions:");
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if self.installed_extensions.gte {
                                ui.label("☑ GTE");
                            } else {
                                ui.label("☐ GTE");
                            }
                            if self.installed_extensions.lidar {
                                ui.label("☑ LiDAR & Remote Sensing");
                            } else {
                                ui.label("☐ LiDAR & Remote Sensing");
                            }
                            
                        });
                        ui.horizontal(|ui| {
                            if self.installed_extensions.dem {
                                ui.label("☑ DEM & Hydrology");
                            } else {
                                ui.label("☐ DEM & Hydrology");
                            }
                            if self.installed_extensions.agriculture {
                                ui.label("☑ Agriculture");
                            } else {
                                ui.label("☐ Agriculture");
                            }
                            if !self.installed_extensions.gte && 
                            !self.installed_extensions.dem &&
                            !self.installed_extensions.lidar &&
                            !self.installed_extensions.agriculture {
                                ui.label("☑ None");
                            } else {
                                ui.label("☐ None");
                            }
                        });
                    });
                    ui.end_row();
                    
                    // if !self.installed_extensions.gte {
                        ui.label("Purchase activation keys at:");
                        ui.hyperlink("https://www.whiteboxgeo.com/");
                        ui.end_row();

                        ui.label("");
                        if ui.button("Install Whitebox Extension").clicked() {
                            self.extension_visible = true;
                        }
                        ui.end_row();
                    // }
                });
        });
    }
}
