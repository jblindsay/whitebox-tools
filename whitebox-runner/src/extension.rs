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

impl ExtensionInstall {
    pub fn new() -> Self {
        ExtensionInstall {
            product_index: 0,
            product_list: vec![
                "General Toolset Extension".to_string(),
                "DEM & Spatial Hydrology Extension".to_string(),
                "Lidar & Remote Sensing Extension".to_string()
                ],
            email: String::new(),
            seat_number: 0,
            activation_key: String::new(),
            text_output: String::new(),
        }
    }
}

impl MyApp {

    pub fn install_extension(&mut self, ctx: &egui::Context) {
        let mut close_dialog = false;
        // let mut install_exit_code = 0;
        let mut install_now = false;
        egui::Window::new("Install a Whitebox Extension Product")
        .open(&mut self.extension_visible)
        .resizable(true)
        .vscroll(true)
        .show(ctx, |ui| {
            egui::Grid::new("install_extension_grid")
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
                
                ui.label("Email address:").on_hover_text("This is the email address of the person to whom the activation key was issued");
                ui.add(
                    egui::TextEdit::singleline(&mut self.ei.email)
                    .desired_width(self.state.textbox_width)
                );
                ui.end_row();

                ui.label("Seat number:").on_hover_text("Activation keys can be for multiple seats. Which seat number are you registering here?");
                ui.add(egui::DragValue::new(&mut self.ei.seat_number).speed(0));
                ui.end_row();

                ui.label("Activation key:").on_hover_text("The activation key issued by Whitebox Geospatial Inc. You may purchase this from www.whiteboxgeo.com.");
                ui.add(
                    egui::TextEdit::singleline(&mut self.ei.activation_key)
                    .desired_width(self.state.textbox_width)
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
                            .id_source("Extension install output")
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
                if ui.button("Install").clicked() {
                    install_now = true;
                }

                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
            });
        });

        if install_now {
            match self.perform_install() {
                Ok(_) => {
                    self.refesh_tools();
                    self.ei.text_output.push_str("Registration of Whitebox Extension was successful!\n");
                },
                Err(err) => {
                    self.ei.text_output.push_str("Registration of Whitebox Extension was unsuccessful.\n");
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

    fn perform_install(&mut self) -> Result<()> {
        // QA/QC
        if self.ei.seat_number <= 0 {
            if rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Wrong Seat Number")
            .set_description("The specified seat number is incorrect. It must be greater than 0.")
            .set_buttons(rfd::MessageButtons::Ok)
            .show() {
                bail!("The specified seat number is incorrect. It must be greater than 0.");
            }
        }

        if !self.ei.email.contains("@") || !self.ei.email.contains(".") {
            if rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Wrong Email")
            .set_description("The specified email address is incorrect.")
            .set_buttons(rfd::MessageButtons::Ok)
            .show() {
                bail!("The specified email address is incorrect.");
            }
        }

        if self.ei.activation_key.trim().is_empty() {
            if rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("No Activation Code")
            .set_description("You have not specified an activation code.")
            .set_buttons(rfd::MessageButtons::Ok)
            .show() {
                bail!("You have not specified an activation code.");
            }
        }
        
        let ext_name = self.ei.product_list[self.ei.product_index].clone();
        let os = env::consts::OS.to_string();
        let arch = env::consts::ARCH.to_string();
        self.ei.text_output.push_str(&format!("Installing the {}...\nOS is {} {}\n", ext_name, os, arch));

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

        // Now let's run the register_license tool with the appropriate parameters
        let register_license = path::Path::new(&plugins_dir).join(&format!("register_license{}", env::consts::EXE_SUFFIX));
        // println!("register_license: {:?}", register_license);
        // check that it exists.
        if !register_license.exists() {
            bail!("Error: register_license file does not exist in plugins directory. Could not register license.\n");
        } else { // file exists
            let output = process::Command::new(register_license)
                    .args([
                        "register", 
                        &self.ei.email.trim(), 
                        &format!("{}", self.ei.seat_number), 
                        &self.ei.activation_key.trim()
                    ])
                    .output()?;
                        
            if !output.status.success() {
                let err_msg = format!("Error registering the extension license. Possible invalid extension key.\n{}\n", String::from_utf8_lossy(&output.stderr));
                bail!(err_msg);
            }
        }

        Ok(())
    }
}