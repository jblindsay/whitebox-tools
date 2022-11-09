use crate::MyApp;
use reqwest;

#[derive(Default)]
pub struct ExtensionInstall {
    product_index: usize,
    product_list: Vec<String>,
    email: String,
    seat_number: usize,
    activation_key: String,
    text_output: String,
}

impl ExtensionInstall {
    pub fn new() -> Self {
        ExtensionInstall {
            product_index: 0,
            product_list: vec![
                "General Toolset Extension".to_string(),
                "DEM & Spatial Hydrology Extension".to_string(),
                "Lidar & Remote Sensing Extension".to_string(),
                "Agriculture Extension".to_string()
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
        let mut install_successful = false;
        egui::Window::new("Install a Whitebox Extension Product")
        .open(&mut self.extension_visible)
        .resizable(false)
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
                        if ui.button("Clear").on_hover_text("Clear tool output").clicked() {
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
                    // QA/QC
                    let mut qa_qc_passed = true;

                    if self.ei.seat_number <= 0 {
                        if rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Error)
                        .set_title("Wrong Seat Number")
                        .set_description("The specified seat number is incorrect. It must be greater than 0.")
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show() {
                            qa_qc_passed = false;
                        }
                    }

                    if !self.ei.email.contains("@") || !self.ei.email.contains(".") {
                        if rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Error)
                        .set_title("Wrong Email")
                        .set_description("The specified email address is incorrect.")
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show() {
                            qa_qc_passed = false;
                        }
                    }

                    if self.ei.activation_key.trim().is_empty() {
                        if rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Error)
                        .set_title("No Activation Code")
                        .set_description("You have not specified an activation code.")
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show() {
                            qa_qc_passed = false;
                        }
                    }
                    
                    if qa_qc_passed {
                        let ext_name = self.ei.product_list[self.ei.product_index].clone();
                        let os = std::env::consts::OS.to_string();
                        let arch = std::env::consts::ARCH.to_string();
                        self.ei.text_output.push_str(&format!("Installing the {}...\nOS is {} {}\n", ext_name, os, arch));

                        let mut url = String::new();
                        // let mut unzipped_dir_name = String::new();
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
                                qa_qc_passed = false;
                            }
                            
                            // unzipped_dir_name = "AgricultureToolset".to_string();
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
                                qa_qc_passed = false;
                            }

                            // unzipped_dir_name = "DemAndSpatialHydrologyToolset".to_string();
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
                                qa_qc_passed = false;
                            }

                            // unzipped_dir_name = "LidarAndRemoteSensingToolset".to_string();
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
                                qa_qc_passed = false;
                            }

                            // unzipped_dir_name = "GeneralToolsetExtension".to_string();
                        }

                        if qa_qc_passed {
                            // let's download the file now...
                            self.ei.text_output.push_str(&format!("Downloading extension file from:\n{}\nPlease be patient...\n", url));

                            let client = reqwest::blocking::Client::builder()
                                .timeout(std::time::Duration::from_secs(90))
                                .build().expect("Failed to build reqwest blocking client.");

                            let req = client.get(&url).build().expect("Error");
                            let resp = client.execute(req).expect("Error executing");
                            let bytes = resp.bytes().unwrap();

                            let num_bytes = bytes.len();
                            self.ei.text_output.push_str(&format!("{} bytes downloaded.\n", num_bytes));

                            
                            // let mut out = std::fs::File::create("/Users/johnlindsay/Downloads/temp.zip").expect("failed to create file");
                            // std::io::copy(&mut archive, &mut out).expect("failed to copy content");

                            self.ei.text_output.push_str("File downloaded. Decompressing into the plugins folder...\n");
                            let archive = &bytes[0..];

                            let mut exe_dir = std::path::PathBuf::from(&self.state.whitebox_exe);
                            exe_dir.pop();
                            let plugins_dir = exe_dir.join("plugins");

                            // The third parameter allows you to strip away toplevel directories.
                            // If `archive` contained a single directory, its contents would be extracted instead.
                            zip_extract::extract(std::io::Cursor::new(archive), &plugins_dir, true).unwrap();

                            if !os.contains("win") {
                                // Mark each of the executable files in the plugins folder as executable.
                                let paths = std::fs::read_dir(&plugins_dir).unwrap();
                                for path in paths {
                                    let p = path.unwrap().path();
                                    if p.is_file() { //}.unwrap_or("") != "json" {
                                        let file_name = p.to_str().unwrap_or("").to_string();
                                        if !file_name.ends_with("json") {
                                            let output = std::process::Command::new("chmod")
                                                        .args(["755", &file_name])
                                                        .output()
                                                        .expect("failed to execute process");
                                            
                                            if !output.status.success() {
                                                self.ei.text_output.push_str(&format!("Error marking {} as executable.\n", file_name));
                                            }
                                        }
                                    }
                                }
                            }

                            // Now let's run the register_license tool with the appropriate parameters
                            let register_license = std::path::Path::new(&plugins_dir).join(&format!("register_license{}", std::env::consts::EXE_SUFFIX));
                            // println!("register_license: {:?}", register_license);
                            // check that it exists.
                            if !register_license.exists() {
                                self.ei.text_output.push_str("Error: register_license file does not exist in plugins directory. Could not register license.\n");
                                install_successful = false;
                            }
                            let output = std::process::Command::new(register_license)
                                        .args([
                                            "register", 
                                            &(self.ei.email), 
                                            &format!("{}", self.ei.seat_number), 
                                            &self.ei.activation_key
                                        ])
                                        .output()
                                        .expect("failed to execute process");
                                            
                            if !output.status.success() {
                                self.ei.text_output.push_str("Error registering the extension license. Possible invalid extension key.\n");
                                self.ei.text_output.push_str(&format!("{:?}\n", String::from_utf8_lossy(&output.stderr)));
                                install_successful = false;
                            }

                        } else {
                            self.ei.text_output.push_str("Your system OS/Architecture are currently unsupported.\nAborting install...\n")
                        }
                    }
                }

                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
            });
        });

        if install_successful {
            // refresh to the tools.
            self.get_tool_info();
            self.ei.text_output.push_str("Registration of Whitebox Extension was successful!\n");
        } else {
            self.ei.text_output.push_str("Registration of Whitebox Extension was unsuccessful.\n");
        }
        

        if close_dialog {
            self.extension_visible = false;
        }
    }
}