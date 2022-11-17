use crate::MyApp;
use std::{env, path, process};
use anyhow::{bail, Result};

impl MyApp {

    pub fn deactivate_extension(&mut self, ctx: &egui::Context) {
        let mut close_dialog = false;
        // let mut install_exit_code = 0;
        let mut deactivate_now = false;
        egui::Window::new("Deactivate a Whitebox Extension Product")
        .open(&mut self.deactivate_extension_visible)
        .resizable(true)
        .vscroll(true)
        .show(ctx, |ui| {
            egui::Grid::new("deactivate_extension_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Extension product:");
                egui::ComboBox::from_id_source("Extension install combobox").show_index(
                    ui,
                    &mut self.ei.product_index,
                    self.ei.product_list.len(),
                    |i| self.ei.product_list[i].to_owned()
                );
                ui.end_row();
            });

            ui.separator();
            ui.vertical(|ui| {
                ui.set_height(170.);
                ui.horizontal(|ui| {
                    ui.label("Output:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✖").on_hover_text("Clear tool output").clicked() {
                            self.deactivatation_output = "".to_string();
                        }
                    });
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.deactivatation_output)
                            .id_source("Extension deactivation output")
                            .cursor_at_end(true)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(10)
                            .lock_focus(true)
                            .desired_width(f32::INFINITY)
                    );
                });
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Deactivate").clicked() {
                    deactivate_now = true;
                }

                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
            });
        });

        if deactivate_now {
            match self.perform_deactivation() {
                Ok(_) => {
                    self.refesh_tools();
                },
                Err(err) => {
                    self.deactivatation_output.push_str("Deactivation of Whitebox Extension was unsuccessful.\n");
                    let err_msg = format!("{}", err);
                    self.deactivatation_output.push_str(&err_msg);
                },
            }
        }

        if close_dialog {
            self.deactivate_extension_visible = false;
        }
    }

    fn perform_deactivation(&mut self) -> Result<()> {
        let ext_name = self.ei.product_list[self.ei.product_index].clone();
        self.deactivatation_output.push_str(&format!("Deactivating the {}...\n", ext_name));

        if ext_name.to_lowercase().contains("agri") && !self.installed_extensions.agriculture {
            bail!("You do not appear to have the {} installed.\n", ext_name);
        } else if ext_name.to_lowercase().contains("dem") && !self.installed_extensions.dem {
            bail!("You do not appear to have the {} installed.\n", ext_name);
        } else if ext_name.to_lowercase().contains("lidar") && !self.installed_extensions.lidar {
            bail!("You do not appear to have the {} installed.\n", ext_name);
        } else if !self.installed_extensions.gte  { // default to the general toolset
            bail!("You do not appear to have the {} installed.\n", ext_name);
        }

        let product = if ext_name.to_lowercase().contains("ag") {
            "ag".to_string()
        } else if ext_name.to_lowercase().contains("lidar") {
            "lidar".to_string()
        } else if ext_name.to_lowercase().contains("dem") {
            "dem".to_string()
        } else if ext_name.to_lowercase().contains("general") {
            "general".to_string()
        } else {
            bail!("Unrecognized extension product {}.\n", ext_name);
        };

        // Now let's run the register_license tool with the appropriate parameters
        let mut exe_dir = path::PathBuf::from(&self.state.whitebox_exe);
        exe_dir.pop();
        let plugins_dir = exe_dir.join("plugins");
        let register_license = path::Path::new(&plugins_dir).join(&format!("register_license{}", env::consts::EXE_SUFFIX));
        // println!("register_license: {:?}", register_license);
        // check that it exists.
        if !register_license.exists() {
            bail!("Error: register_license file does not exist in plugins directory. Could not register license.\n");
        } else { // file exists
            let output = process::Command::new(register_license)
                    .args([
                        "deactivate", 
                        &product
                    ])
                    .output()?;
                        
            if !output.status.success() {
                let err_msg = format!("Error registering the extension license.\n{}\n", String::from_utf8_lossy(&output.stderr));
                bail!(err_msg);
            } else {
                let s = std::str::from_utf8(&output.stdout).unwrap_or("").to_string();
                self.deactivatation_output.push_str(&format!("{}\n", s));
            }
        }

        Ok(())
    }
}