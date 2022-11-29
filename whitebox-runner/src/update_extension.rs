use crate::MyApp;
use reqwest;
use std::{env, ffi, fs, io, path, process, time};
use anyhow::{bail, Result};

#[derive(Default)]
pub struct ExtensionInstall {
    pub product_index: usize,
    pub product_list: Vec<String>,
    pub email: String,
    pub seat_number: usize,
    pub activation_key: String,
    pub text_output: String,
}

impl MyApp {

    pub fn update_extension(&mut self, ctx: &egui::Context) {
        let mut close_dialog = false;
        let mut update_now = false;
        egui::Window::new("Update a Whitebox Extension Product")
        .open(&mut self.update_extension_visible)
        .resizable(true)
        .vscroll(true)
        .show(ctx, |ui| {
            egui::Grid::new("update_extension_grid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Extension product:");
                egui::ComboBox::from_id_source("Extension update combobox").show_index(
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
                            self.ei.text_output = "".to_string();
                        }
                    });
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.ei.text_output)
                            .id_source("Extension update output")
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
                if ui.button("Update").clicked() {
                    update_now = true;
                }

                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
            });
        });

        if update_now {
            match self.perform_update() {
                Ok(_) => {
                    self.refesh_tools();
                    self.ei.text_output.push_str("Update of Whitebox Extension was successful!\n");
                },
                Err(err) => {
                    self.ei.text_output.push_str("Update of Whitebox Extension was unsuccessful.\n");
                    let err_msg = format!("{}", err);
                    if err_msg.to_lowercase().contains("invalid") && err_msg.to_lowercase().contains("key") {
                        self.ei.text_output.push_str("The specified key appears to be invalid. Please contact www.whiteboxgeo.com to purchase a valid activation key for this product.");
                    } else {
                        self.ei.text_output.push_str(&err_msg);
                    }
                },
            }
        }

        if close_dialog {
            self.extension_visible = false;
        }
    }

    fn perform_update(&mut self) -> Result<()> {
        let ext_name = self.ei.product_list[self.ei.product_index].clone();
        let os = env::consts::OS.to_string();
        let arch = env::consts::ARCH.to_string();
        self.ei.text_output.push_str(&format!("Updating the {}...\nOS is {} {}\n", ext_name, os, arch));

        let url: String;
        if ext_name.to_lowercase().contains("agri") {
            if os.contains("win") {
                url = "https://www.whiteboxgeo.com/AgricultureToolset/AgricultureToolset_win.zip".to_string();
            } else if os.contains("mac") && arch.contains("x86_64") {
                url = "https://www.whiteboxgeo.com/AgricultureToolset/AgricultureToolset_MacOS_Intel.zip".to_string();
            } else if os.contains("mac") && arch.contains("aarch64") {
                url = "https://www.whiteboxgeo.com/AgricultureToolset/AgricultureToolset_MacOS_ARM.zip".to_string();
            } else if os.contains("linux") {
                url = "https://www.whiteboxgeo.com/AgricultureToolset/AgricultureToolset_linux.zip".to_string();
            } else {
                bail!("Your system OS/Architecture are currently unsupported.\nAborting install...\n");
            }
        } else if ext_name.to_lowercase().contains("dem") {
            if os.contains("win") {
                url = "https://www.whiteboxgeo.com/DemAndSpatialHydrologyToolset/DemAndSpatialHydrologyToolset_win.zip".to_string();
            } else if os.contains("mac") && arch.contains("x86_64") {
                url = "https://www.whiteboxgeo.com/DemAndSpatialHydrologyToolset/DemAndSpatialHydrologyToolset_MacOS_Intel.zip".to_string();
            } else if os.contains("mac") && arch.contains("aarch64") {
                url = "https://www.whiteboxgeo.com/DemAndSpatialHydrologyToolset/DemAndSpatialHydrologyToolset_MacOS_ARM.zip".to_string();
            } else if os.contains("linux") {
                url = "https://www.whiteboxgeo.com/DemAndSpatialHydrologyToolset/DemAndSpatialHydrologyToolset_linux.zip".to_string();
            } else {
                bail!("Your system OS/Architecture are currently unsupported.\nAborting install...\n");
            }
        } else if ext_name.to_lowercase().contains("lidar") {
            if os.contains("win") {
                url = "https://www.whiteboxgeo.com/LidarAndRemoteSensingToolset/LidarAndRemoteSensingToolset_win.zip".to_string();
            } else if os.contains("mac") && arch.contains("x86_64") {
                url = "https://www.whiteboxgeo.com/LidarAndRemoteSensingToolset/LidarAndRemoteSensingToolset_MacOS_Intel.zip".to_string();
            } else if os.contains("mac") && arch.contains("aarch64") {
                url = "https://www.whiteboxgeo.com/LidarAndRemoteSensingToolset/LidarAndRemoteSensingToolset_MacOS_ARM.zip".to_string();
            } else if os.contains("linux") {
                url = "https://www.whiteboxgeo.com/LidarAndRemoteSensingToolset/LidarAndRemoteSensingToolset_linux.zip".to_string();
            } else {
                bail!("Your system OS/Architecture are currently unsupported.\nAborting install...\n");
            }
        } else { // default to the general toolset
            if os.contains("win") {
                url = "https://www.whiteboxgeo.com/GTE_Windows/GeneralToolsetExtension_win.zip".to_string();
            } else if os.contains("mac") && arch.contains("x86_64") {
                url = "https://www.whiteboxgeo.com/GTE_Darwin/GeneralToolsetExtension_MacOS_Intel.zip".to_string();
            } else if os.contains("mac") && arch.contains("aarch64") {
                url = "https://www.whiteboxgeo.com/GTE_Darwin/GeneralToolsetExtension_MacOS_ARM.zip".to_string();
            } else if os.contains("linux") {
                url = "https://www.whiteboxgeo.com/GTE_Linux/GeneralToolsetExtension_linux.zip".to_string();
            } else {
                bail!("Your system OS/Architecture are currently unsupported.\nAborting install...\n");
            }
        }

        // let's download the file now...
        self.ei.text_output.push_str(&format!("Downloading extension file from:\n{}\nPlease be patient...\n", url));

        if rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title("Downloading Whitebox Extension")
            .set_description("Downloading extension file. This may take a while and WbRunner may freeze while it is downloading.\n\nPLEASE BE PATIENT...")
            .set_buttons(rfd::MessageButtons::Ok)
            .show() {
                // do nothing
            }

        let client = reqwest::blocking::Client::builder()
            .timeout(time::Duration::from_secs(90))
            .build()?;

        let req = client.get(&url).build()?;
        let resp = client.execute(req)?;
        let bytes = resp.bytes()?;

        let num_bytes = bytes.len();
        self.ei.text_output.push_str(&format!("{} bytes downloaded.\n", num_bytes));

        
        // let mut out = fs::File::create("/Users/johnlindsay/Downloads/temp.zip")?;
        // io::copy(&mut archive, &mut out)?;

        self.ei.text_output.push_str("File downloaded. Decompressing into the plugins folder...\n");
        let archive = &bytes[0..];

        let mut exe_dir = path::PathBuf::from(&self.state.whitebox_exe);
        exe_dir.pop();
        let plugins_dir = exe_dir.join("plugins");

        // The third parameter allows you to strip away toplevel directories.
        zip_extract::extract(io::Cursor::new(archive), &plugins_dir, true)?;

        // ...but the files may still be located within a subfolder. 
        // If so, promote the contents of the subfolder into the higher level plugins folder.
        let paths = fs::read_dir(&plugins_dir)?;
        for path in paths {
            let p = path?.path();
            if p.is_dir() {
                // copy the files within p into plugins_dir
                let paths2 = fs::read_dir(p.to_str().unwrap_or(""))?;
                for path2 in paths2 {
                    let p2 = path2?.path();
                    if p2.is_file() {
                        let src = p2.to_str().unwrap_or("");
                        let dest = path::Path::new(&plugins_dir).join(p2.file_name().unwrap_or(ffi::OsStr::new("")));
                        fs::copy(src, dest)?;
                    }
                }
                // Now delete p
                fs::remove_dir_all(p.to_str().unwrap_or(""))?;
            }
        }

        if !os.contains("win") {
            // Mark each of the executable files in the plugins folder as executable.
            let paths = fs::read_dir(&plugins_dir)?;
            for path in paths {
                let p = path?.path();
                if p.is_file() {
                    let file_name = p.to_str().unwrap_or("").to_string();
                    if !file_name.ends_with("json") {
                        let output = process::Command::new("chmod")
                                    .args(["755", &file_name])
                                    .output()?;
                        
                        if !output.status.success() {
                            self.ei.text_output.push_str(&format!("Error marking {} as executable.\n", file_name));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}