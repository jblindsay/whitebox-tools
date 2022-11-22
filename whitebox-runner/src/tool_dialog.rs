use crate::MyApp;
use crate::toggle;
use case::CaseExt;
use std::{f32, fs, path, path::Path};
use whitebox_vector::{ShapeType, Shapefile};
use crate::tool_info::{
    ParameterFileType,
    ParameterType,
    ToolParameter,
    VectorGeometryType,
};

impl MyApp {

    pub fn tool_dialog(&mut self, ctx: &egui::Context, tool_idx: usize) {
        let mut close_dialog = false;
        let mut wk_dir = String::new();
        _ = self.get_tool_parameters(&self.list_of_open_tools[tool_idx].tool_name);
        egui::Window::new(&format!("{}", &self.list_of_open_tools[tool_idx].tool_name))
        .id(egui::Id::new(format!("{}-{}", &self.list_of_open_tools[tool_idx].tool_name, tool_idx)))
        .open(&mut self.open_tools[tool_idx])
        .resizable(true)
        .vscroll(false)
        .show(ctx, |ui| {

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Tool parameters:").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔃").on_hover_text("Reset parameters").clicked() { // ⟲
                        self.list_of_open_tools[tool_idx].reset();
                    }
                });
            });
            // ui.separator();

            egui::ScrollArea::vertical()
                .min_scrolled_height(50.)
                .max_height(150.0)
                .auto_shrink([true; 2])
                .show(ui, |ui| {

                egui::Grid::new(&format!("grid{}-{}", &self.list_of_open_tools[tool_idx].tool_name, tool_idx))
                .num_columns(2)
                .spacing([10.0, 8.0])
                .striped(true)
                .show(ui, |ui| {

                    // The following is used for ParameterType::VectorAttributeField. The hints for this widget
                    // need to be able to retrieve the value of it's parent ParameterType::ExistingFile, which
                    // it cannot do within the loop simply by referencing self.list_of_open_tools[tool_idx].parameters
                    // directly, due to a double mut borrow error. So this does a pre-pass looking for these
                    // widgets and cloning values.
                    let mut flagged_parameter = Vec::with_capacity(self.list_of_open_tools[tool_idx].parameters.len() - 1); // self.list_of_open_tools[tool_idx].parameters[0].clone();
                    for m in 0..self.list_of_open_tools[tool_idx].parameters.len() {
                        if self.list_of_open_tools[tool_idx].parameters[m].parameter_type == ParameterType::VectorAttributeField {
                            let flag = self.list_of_open_tools[tool_idx].parameters[m].str_vec_value[1].clone();
                            for n in 0..self.list_of_open_tools[tool_idx].parameters.len() {
                                for f in &self.list_of_open_tools[tool_idx].parameters[n].flags {
                                    if f.to_string() == flag {
                                        flagged_parameter.push(self.list_of_open_tools[tool_idx].parameters[n].str_value.clone());
                                    }
                                }
                            }
                        }
                    }
                    let mut flagged_parameter_idx = 0;
                    for parameter in &mut (self.list_of_open_tools[tool_idx].parameters) {
                        let suffix = if parameter.optional { "*".to_string() } else { "".to_string() };
                        let parameter_label = if parameter.name.len() + suffix.len() < 25 {
                            format!("{}{}", &parameter.name, suffix)
                        } else {
                            format!("{}...{}", &parameter.name[0..(22-suffix.len())], suffix)
                        };
                        let param_nm = if !parameter.optional { parameter.name.clone() } else { format!("{} [Optional]", parameter.name) };
                        let hover_text = match parameter.file_type {
                            ParameterFileType::Vector | ParameterFileType::RasterAndVector => {
                                format!("{}:  {} (Geometry Type={:?})", param_nm, parameter.description, parameter.geometry_type)
                            },
                            _ => {
                                format!("{}:  {}", param_nm, parameter.description)
                            }
                        };
                        ui.label(&parameter_label)
                        .on_hover_text(&hover_text);

                        match parameter.parameter_type {
                            ParameterType::Boolean => {
                                ui.add(toggle(&mut parameter.bool_value));
                            },
                            ParameterType::Directory => {
                                ui.horizontal(|ui| {
                                    if ui.add(
                                        egui::TextEdit::singleline(&mut parameter.str_value)
                                        .desired_width(self.state.textbox_width - 22.0)
                                    ).double_clicked() {
                                        let fdialog = get_file_dialog(&parameter.file_type); 
                                        if let Some(path) = fdialog
                                        .set_directory(std::path::Path::new(&self.state.working_dir))
                                        .pick_file() {
                                            parameter.str_value = path.display().to_string();
                                            // update the working directory
                                            // path.pop();
                                            // self.state.working_dir = path.display().to_string();
                                            wk_dir = path.display().to_string();
                                        }
                                    }
    
                                    ui.add_space(-(ui.style().spacing.item_spacing[0])+2.);
    
                                    ui.menu_button("⏷", |ui| {
                                        ui.set_min_width(150.);
                                        ui.set_max_width(250.);
                                        egui::ScrollArea::both()
                                        .max_height(400.0)
                                        .auto_shrink([true, true])
                                        .show(ui, |ui| {
                                            if self.state.recent_working_dirs.len() > 0 {
                                                for q in (0..self.state.recent_working_dirs.len()).rev() {
                                                    if let Some(lbl) = Path::new(&self.state.recent_working_dirs[q]).file_name() {
                                                        let lbl_str = lbl.to_str().unwrap_or(&self.state.recent_working_dirs[q]).to_string();
                                                        if ui.button(&lbl_str).clicked() {
                                                            parameter.str_value = self.state.recent_working_dirs[q].clone();
                                                            ui.close_menu();
                                                        }
                                                    }
                                                }
                                            } else {
                                                if ui.button("There are no recent working directories available. Please press `...` to select one.").clicked() {
                                                    ui.close_menu();
                                                }
                                            }
    
                                        });
                                    });    
                                });
                                
                                if ui.button("…").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().set_directory(std::path::Path::new(&self.state.working_dir)).pick_folder() {
                                        parameter.str_value = path.display().to_string();
                                    }
                                }
                            },
                            ParameterType::ExistingFile => {
                                ui.horizontal(|ui| {
                                    
                                    let resp = ui.add(
                                        egui::TextEdit::singleline(&mut parameter.str_value)
                                        .desired_width(self.state.textbox_width - 22.0)
                                    );
                                    if resp.lost_focus() {
                                        if !parameter.str_value.is_empty() && !path::Path::new(&parameter.str_value).exists() {
                                            // prepend the working directory and see if that file exists.
                                            let f = path::Path::new(&self.state.working_dir).join(&parameter.str_value);
                                            if f.exists() {
                                                parameter.str_value = f.to_str().unwrap_or("").to_string();
                                            } else {
                                                if rfd::MessageDialog::new()
                                                .set_level(rfd::MessageLevel::Warning)
                                                .set_title("File does not exist")
                                                .set_description("The specified file does not exist in the current working directory. Do you want to continue?")
                                                .set_buttons(rfd::MessageButtons::YesNo)
                                                .show() {
                                                    // do nothing
                                                } else {
                                                    // Reset the parameter string value.
                                                    parameter.str_value = "".to_string();
                                                }
                                            }
                                        }
                                    }
                                    if resp.double_clicked() {
                                        let fdialog = get_file_dialog(&parameter.file_type); 
                                        if let Some(path) = fdialog
                                        .set_directory(std::path::Path::new(&self.state.working_dir))
                                        .pick_file() {
                                            parameter.str_value = path.display().to_string();
                                            
                                            if parameter.file_type == ParameterFileType::Vector && 
                                            parameter.geometry_type != VectorGeometryType::Any {
                                                check_geometry_type(parameter, &self.state.working_dir);
                                            }

                                            // update the working directory
                                            // path.pop();
                                            // self.state.working_dir = path.display().to_string();
                                            // self.update_working_dir(&path.display().to_string());
                                            wk_dir = path.display().to_string();
                                        }
                                    }
                                    
                                    ui.add_space(-(ui.style().spacing.item_spacing[0])+2.);

                                    ui.menu_button("⏷", |ui| {
                                        ui.set_min_width(150.);
                                        ui.set_max_width(250.);
                                        if self.state.recent_working_dirs.len() == 0 {
                                            if ui.button("The current working directory is not set. Press `...` to choose a new directory instead.").clicked() {
                                                ui.close_menu();
                                            }
                                        } else {
                                            egui::ScrollArea::both()
                                            .max_height(400.0)
                                            .auto_shrink([true, true])
                                            .show(ui, |ui| {
                                                if self.state.recent_working_dirs.len() > 1 {
                                                    ui.label(egui::RichText::new("Recent directories:")
                                                    .italics()
                                                    .strong()
                                                    .color(ui.visuals().hyperlink_color));
                                                }
                                                // first find all the files in each of the recent directories, except the most recent.
                                                if self.state.recent_working_dirs.len() > 1 {
                                                    for q in (0..self.state.recent_working_dirs.len()-1).rev() { // The '-1' excludes the most recent dir.
                                                        let extensions = get_file_extensions(&parameter.file_type);
                                                        let dir = &self.state.recent_working_dirs[q];
            
                                                        let mut files: Vec<String> = vec![];
                                                        if let Ok(paths) = fs::read_dir(dir) {
                                                            for path in paths {
                                                                if let Ok(dir_entry) = path {
                                                                    let p = dir_entry.path();
                                                                    if p.is_file() {
                                                                        if !extensions.is_empty() {
                                                                            if let Some(exe) = p.extension() {
                                                                                let ext_str = exe.to_str().unwrap_or("").to_lowercase();
                                                                                for e in &extensions {
                                                                                    if e.to_lowercase() == ext_str {
                                                                                        if let Some(short_fn) = p.file_name() {
                                                                                            files.push(short_fn.to_str().unwrap_or("").to_string());
                                                                                            break;
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        } else {
                                                                            if let Some(short_fn) = p.file_name() {
                                                                                files.push(short_fn.to_str().unwrap_or("").to_string());
                                                                            }
                                                                        }                                        
                                                                    }
                                                                }
                                                            }
                                                        }
            
                                                        if files.len() > 0 {
                                                            files.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                                                            if let Some(lbl) = Path::new(dir).file_name() {
                                                                let lbl_str = lbl.to_str().unwrap_or(dir).to_string();
                                                                ui.menu_button(&lbl_str, |ui| {
                                                                    ui.set_min_width(150.);
                                                                    ui.set_max_width(250.);

                                                                    egui::ScrollArea::both()
                                                                    .max_height(400.0)
                                                                    .auto_shrink([true; 2])
                                                                    .show(ui, |ui| {
                                                                        if let Some(lbl) = Path::new(dir).file_name() {
                                                                            let lbl_str = lbl.to_str().unwrap_or(dir).to_string();
                                                                            ui.label(egui::RichText::new(&format!("Files in {lbl_str}:"))
                                                                            .italics()
                                                                            .strong()
                                                                            .color(ui.visuals().hyperlink_color));
                                                                        }

                                                                        for file in &files {
                                                                            if ui.add(egui::Button::new(file)).clicked() {
                                                                                parameter.str_value = format!("{}{}{}", dir, std::path::MAIN_SEPARATOR, file.clone());
            
                                                                                if parameter.file_type == ParameterFileType::Vector && 
                                                                                parameter.geometry_type != VectorGeometryType::Any {
                                                                                    check_geometry_type(parameter, &dir);
                                                                                }
            
                                                                                wk_dir = parameter.str_value.clone();
                                                                                ui.close_menu();
                                                                            }
                                                                        }
                                                                    });
                                                                });
                                                                ui.add_space(1.);
                                                            }
                                                        }
                                                    }
                                                }

                                                // now do the current working directory
                                                let extensions = get_file_extensions(&parameter.file_type);
                                                let dir = &self.state.recent_working_dirs[self.state.recent_working_dirs.len()-1];
    
                                                if self.state.recent_working_dirs.len() > 1 {
                                                    ui.separator();
                                                }

                                                if let Some(lbl) = Path::new(dir).file_name() {
                                                    let lbl_str = lbl.to_str().unwrap_or(dir).to_string();
                                                    ui.label(egui::RichText::new(&format!("Files in {lbl_str}:"))
                                                    .italics()
                                                    .strong()
                                                    .color(ui.visuals().hyperlink_color));
                                                }

                                                let mut files: Vec<String> = vec![];
                                                if let Ok(paths) = fs::read_dir(dir) {
                                                    for path in paths {
                                                        if let Ok(dir_entry) = path {
                                                            let p = dir_entry.path();
                                                            if p.is_file() {
                                                                if !extensions.is_empty() {
                                                                    if let Some(exe) = p.extension() {
                                                                        let ext_str = exe.to_str().unwrap_or("").to_lowercase();
                                                                        for e in &extensions {
                                                                            if e.to_lowercase() == ext_str {
                                                                                if let Some(short_fn) = p.file_name() {
                                                                                    files.push(short_fn.to_str().unwrap_or("").to_string());
                                                                                    break;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                } else {
                                                                    if let Some(short_fn) = p.file_name() {
                                                                        files.push(short_fn.to_str().unwrap_or("").to_string());
                                                                    }
                                                                }                                        
                                                            }
                                                        }
                                                    }
                                                }
    
                                                if files.len() > 0 {
                                                    files.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                                                    for file in &files {
                                                        if ui.add(egui::Button::new(file)).clicked() {
                                                            parameter.str_value = format!("{}{}{}", dir, std::path::MAIN_SEPARATOR, file.clone());

                                                            if parameter.file_type == ParameterFileType::Vector && 
                                                            parameter.geometry_type != VectorGeometryType::Any {
                                                                check_geometry_type(parameter, &dir);
                                                            }

                                                            wk_dir = parameter.str_value.clone();
                                                            ui.close_menu();
                                                        }
                                                    }
                                                } else {
                                                    if ui.button("No file of the required type are within the current working directory. Press `...` to choose a new directory instead.").clicked() {
                                                        ui.close_menu();
                                                    }
                                                }
                                            });

                                        }
                                        
                                    });
                                });

                                if ui.button("…").clicked() {
                                    let fdialog = get_file_dialog(&parameter.file_type); 
                                    if let Some(path) = fdialog
                                    .set_directory(std::path::Path::new(&self.state.working_dir))
                                    .pick_file() {
                                        parameter.str_value = path.display().to_string();

                                        if parameter.file_type == ParameterFileType::Vector && 
                                        parameter.geometry_type != VectorGeometryType::Any {
                                            check_geometry_type(parameter, &self.state.working_dir);
                                        }

                                        // update the working directory
                                        // path.pop();
                                        // self.state.working_dir = path.display().to_string();
                                        // self.update_working_dir(&path.display().to_string());
                                        wk_dir = path.display().to_string();
                                    }
                                }
                            },
                            ParameterType::ExistingFileOrFloat => {
                                ui.horizontal(|ui| {
                                    if ui.add(
                                        egui::TextEdit::singleline(&mut parameter.str_value)
                                        .desired_width(self.state.textbox_width - 22.0)
                                    ).double_clicked() {
                                        let fdialog = get_file_dialog(&parameter.file_type); 
                                        if let Some(path) = fdialog
                                        .set_directory(std::path::Path::new(&self.state.working_dir))
                                        .pick_file() {
                                            parameter.str_value = path.display().to_string();
                                            wk_dir = path.display().to_string();
                                        }
                                    }

                                    ui.add_space(-(ui.style().spacing.item_spacing[0])+2.);

                                    ui.menu_button("⏷", |ui| {
                                        ui.set_min_width(150.);
                                        ui.set_max_width(250.);
                                        if self.state.recent_working_dirs.len() == 0 {
                                            if ui.button("The current working directory is not set. Press `...` to choose a new directory instead.").clicked() {
                                                ui.close_menu();
                                            }
                                        } else {
                                            egui::ScrollArea::both()
                                            .max_height(400.0)
                                            .auto_shrink([true, true])
                                            .show(ui, |ui| {
                                                if self.state.recent_working_dirs.len() > 1 {
                                                    ui.label(egui::RichText::new("Recent directories:")
                                                    .italics()
                                                    .strong()
                                                    .color(ui.visuals().hyperlink_color));
                                                }
                                                // first find all the files in each of the recent directories, except the most recent.
                                                if self.state.recent_working_dirs.len() > 1 {
                                                    for q in (0..self.state.recent_working_dirs.len()-1).rev() { // The '-1' excludes the most recent dir.
                                                        let extensions = get_file_extensions(&parameter.file_type);
                                                        let dir = &self.state.recent_working_dirs[q];
            
                                                        let mut files: Vec<String> = vec![];
                                                        if let Ok(paths) = fs::read_dir(dir) {
                                                            for path in paths {
                                                                if let Ok(dir_entry) = path {
                                                                    let p = dir_entry.path();
                                                                    if p.is_file() {
                                                                        if !extensions.is_empty() {
                                                                            if let Some(exe) = p.extension() {
                                                                                let ext_str = exe.to_str().unwrap_or("").to_lowercase();
                                                                                for e in &extensions {
                                                                                    if e.to_lowercase() == ext_str {
                                                                                        if let Some(short_fn) = p.file_name() {
                                                                                            files.push(short_fn.to_str().unwrap_or("").to_string());
                                                                                            break;
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        } else {
                                                                            if let Some(short_fn) = p.file_name() {
                                                                                files.push(short_fn.to_str().unwrap_or("").to_string());
                                                                            }
                                                                        }                                        
                                                                    }
                                                                }
                                                            }
                                                        }
            
                                                        if files.len() > 0 {
                                                            files.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                                                            if let Some(lbl) = Path::new(dir).file_name() {
                                                                let lbl_str = lbl.to_str().unwrap_or(dir).to_string();
                                                                ui.menu_button(&lbl_str, |ui| {
                                                                    ui.set_min_width(150.);
                                                                    ui.set_max_width(250.);

                                                                    egui::ScrollArea::both()
                                                                    .max_height(400.0)
                                                                    .auto_shrink([true; 2])
                                                                    .show(ui, |ui| {
                                                                        if let Some(lbl) = Path::new(dir).file_name() {
                                                                            let lbl_str = lbl.to_str().unwrap_or(dir).to_string();
                                                                            ui.label(egui::RichText::new(&format!("Files in {lbl_str}:"))
                                                                            .italics()
                                                                            .strong()
                                                                            .color(ui.visuals().hyperlink_color));
                                                                        }
                                                                        
                                                                        for file in &files {
                                                                            if ui.add(egui::Button::new(file)).clicked() {
                                                                                parameter.str_value = format!("{}{}{}", dir, std::path::MAIN_SEPARATOR, file.clone());
            
                                                                                if parameter.file_type == ParameterFileType::Vector && 
                                                                                parameter.geometry_type != VectorGeometryType::Any {
                                                                                    check_geometry_type(parameter, &dir);
                                                                                }
            
                                                                                wk_dir = parameter.str_value.clone();
                                                                                ui.close_menu();
                                                                            }
                                                                        }
                                                                    });
                                                                });
                                                                ui.add_space(1.);
                                                            }
                                                        }
                                                    }
                                                }

                                                // now do the current working directory
                                                let extensions = get_file_extensions(&parameter.file_type);
                                                let dir = &self.state.recent_working_dirs[self.state.recent_working_dirs.len()-1];
    
                                                if self.state.recent_working_dirs.len() > 1 {
                                                    ui.separator();
                                                }

                                                if let Some(lbl) = Path::new(dir).file_name() {
                                                    let lbl_str = lbl.to_str().unwrap_or(dir).to_string();
                                                    ui.label(egui::RichText::new(&format!("Files in {lbl_str}:"))
                                                    .italics()
                                                    .strong()
                                                    .color(ui.visuals().hyperlink_color));
                                                }

                                                let mut files: Vec<String> = vec![];
                                                if let Ok(paths) = fs::read_dir(dir) {
                                                    for path in paths {
                                                        if let Ok(dir_entry) = path {
                                                            let p = dir_entry.path();
                                                            if p.is_file() {
                                                                if !extensions.is_empty() {
                                                                    if let Some(exe) = p.extension() {
                                                                        let ext_str = exe.to_str().unwrap_or("").to_lowercase();
                                                                        for e in &extensions {
                                                                            if e.to_lowercase() == ext_str {
                                                                                if let Some(short_fn) = p.file_name() {
                                                                                    files.push(short_fn.to_str().unwrap_or("").to_string());
                                                                                    break;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                } else {
                                                                    if let Some(short_fn) = p.file_name() {
                                                                        files.push(short_fn.to_str().unwrap_or("").to_string());
                                                                    }
                                                                }                                        
                                                            }
                                                        }
                                                    }
                                                }
    
                                                if files.len() > 0 {
                                                    files.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                                                    for file in &files {
                                                        if ui.add(egui::Button::new(file)).clicked() {
                                                            parameter.str_value = format!("{}{}{}", dir, std::path::MAIN_SEPARATOR, file.clone());

                                                            if parameter.file_type == ParameterFileType::Vector && 
                                                            parameter.geometry_type != VectorGeometryType::Any {
                                                                check_geometry_type(parameter, &dir);
                                                            }

                                                            wk_dir = parameter.str_value.clone();
                                                            ui.close_menu();
                                                        }
                                                    }
                                                } else {
                                                    if ui.button("No file of the required type are within the current working directory. Press `...` to choose a new directory instead.").clicked() {
                                                        ui.close_menu();
                                                    }
                                                }
                                            });

                                        }
                                        
                                    });

                                    if ui.button("…").clicked() {
                                        let fdialog = get_file_dialog(&parameter.file_type); 
                                        if let Some(path) = fdialog
                                        .set_directory(std::path::Path::new(&self.state.working_dir))
                                        .pick_file() {
                                            parameter.str_value = path.display().to_string();
                                            // update the working directory
                                            // path.pop();
                                            // self.state.working_dir = path.display().to_string();
                                            // self.update_working_dir(&path.display().to_string());
                                            wk_dir = path.display().to_string();
                                        }
                                    }

                                    ui.label("OR");
                                    
                                    ui.add(
                                        egui::TextEdit::singleline(&mut parameter.str_vec_value[0])
                                        .desired_width(50.0)
                                    );
                                });
                            },
                            ParameterType::FileList => {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    if ui.add(
                                        egui::TextEdit::multiline(&mut parameter.str_value)
                                        .desired_width(self.state.textbox_width)
                                        .desired_rows(4)
                                    ).double_clicked() {
                                        let fdialog = get_file_dialog(&parameter.file_type); 
                                        if let Some(path) = fdialog
                                        .set_directory(std::path::Path::new(&self.state.working_dir))
                                        .pick_file() {
                                            parameter.str_value = path.display().to_string();
                                            // update the working directory
                                            // path.pop();
                                            // self.state.working_dir = path.display().to_string();
                                            // self.update_working_dir(&path.display().to_string());
                                            wk_dir = path.display().to_string();
                                        }
                                    }
                                });
                                if ui.button("…").clicked() {
                                    let fdialog = get_file_dialog(&parameter.file_type);

                                    if let Some(paths) = fdialog
                                    .set_directory(std::path::Path::new(&self.state.working_dir))
                                    .pick_files() {
                                        // let s = String::new();
                                        for path in &paths {
                                            parameter.str_value.push_str(&format!("{}\n", path.display().to_string()));
                                        }
                                        
                                        // update the working directory
                                        // paths[0].pop();
                                        // self.state.working_dir = paths[0].display().to_string();
                                        // self.update_working_dir(&paths[0].display().to_string());
                                        wk_dir = paths[0].display().to_string();
                                    }
                                }
                            }
                            ParameterType::Float | ParameterType::Integer => {
                                // ui.add(egui::DragValue::new(&mut parameter.float_value).speed(0).max_decimals(5));
                                ui.add(
                                    egui::TextEdit::singleline(&mut parameter.str_value)
                                    .desired_width(50.0) //self.state.textbox_width)
                                );

                                // let text_edit = egui::TextEdit::singleline(&mut parameter.str_value)
                                // .desired_width(50.0);
                                // let output = text_edit.show(ui);
                                // if output.response.double_clicked() {
                                //     // What to do here?
                                // }

                            },
                            ParameterType::NewFile => {
                                // ui.text_edit_singleline(&mut parameter.str_value);
                                if ui.add(
                                    egui::TextEdit::singleline(&mut parameter.str_value)
                                    .desired_width(self.state.textbox_width)
                                ).double_clicked() {
                                    let fdialog = get_file_dialog(&parameter.file_type); 
                                    if let Some(path) = fdialog
                                    .set_directory(std::path::Path::new(&self.state.working_dir))
                                    .save_file() {
                                        parameter.str_value = path.display().to_string();
                                        // self.update_working_dir(&path.display().to_string());
                                        wk_dir = path.display().to_string();
                                    }
                                }
                                if ui.button("…").clicked() {
                                    let fdialog = get_file_dialog(&parameter.file_type); 
                                    if let Some(path) = fdialog.set_directory(std::path::Path::new(&self.state.working_dir)).save_file() {
                                        parameter.str_value = path.display().to_string();
                                        // self.update_working_dir(&path.display().to_string());
                                        wk_dir = path.display().to_string();
                                    }
                                }
                            },
                            ParameterType::OptionList => {
                                let alternatives = parameter.str_vec_value.clone();
                                egui::ComboBox::from_id_source(&parameter.name).show_index(
                                    ui,
                                    &mut parameter.int_value,
                                    alternatives.len(),
                                    |i| alternatives[i].to_owned()
                                );
                            }
                            ParameterType::String => {
                                ui.add(
                                    egui::TextEdit::singleline(&mut parameter.str_value)
                                    .desired_width(self.state.textbox_width)
                                );
                            },
                            ParameterType::StringOrNumber => {
                                ui.add(
                                    egui::TextEdit::singleline(&mut parameter.str_value)
                                    .desired_width(self.state.textbox_width)
                                );
                            },
                            ParameterType::VectorAttributeField => {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut parameter.str_value)
                                        .desired_width(self.state.textbox_width - 22.0)
                                    );
                                    ui.add_space(-(ui.style().spacing.item_spacing[0])+2.);

                                    if ui.menu_button("⏷", |ui| {
                                        egui::ScrollArea::both()
                                        .max_height(200.0)
                                        .auto_shrink([true; 2])
                                        .show(ui, |ui| {
                                            if parameter.str_vec_value.len() > 2 {
                                                for k in 2..parameter.str_vec_value.len() {
                                                    let att = &parameter.str_vec_value[k];
                                                    if ui.button(att).clicked() {
                                                        parameter.str_value = att.clone();
                                                        ui.close_menu();
                                                    }
                                                }
                                            } else {
                                                if ui.button("No attribute hints are available: The parent vector file must first be specified").clicked() {
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                    }).response.clicked() {
                                        if parameter.str_vec_value.len() > 2 {
                                            while parameter.str_vec_value.len() > 2 {
                                                parameter.str_vec_value.pop();
                                            }
                                        }
                                        let mut file_name = flagged_parameter[flagged_parameter_idx].clone();
                                        let mut file_path = path::PathBuf::new();
                                        file_path.push(&file_name);
                                        if !file_path.exists() {
                                            // prepend the working directory and see if that file exists.
                                            let mut file_path = path::PathBuf::new();
                                            file_path.push(&self.state.working_dir);
                                            file_path = file_path.join(&file_name);
                                            // file_path = path::PathBuf::new(&self.state.working_dir).join(&file_name);
                                            if file_path.exists() {
                                                file_name = file_path.to_str().unwrap_or("").to_string();
                                            }
                                        }
                                        if file_path.exists() {
                                            if let Ok(shape) = Shapefile::read(&file_name) {
                                                for att in &shape.attributes.fields {
                                                    parameter.str_vec_value.push(att.name.clone());
                                                }
                                            }
                                        }
                                    }

                                });

                                flagged_parameter_idx += 1;
                            },
                        }
                        
                        ui.end_row();
                    }
                });
            });

            if self.state.view_tool_output {
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_height(145.);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Tool output:").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✖").on_hover_text("Clear tool output").clicked() {
                                if let Ok(mut tool_output) = self.list_of_open_tools[tool_idx].tool_output.lock() {
                                    *tool_output = "".to_string();
                                }
                            }
                        });
                    });

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Ok(mut tool_output) = self.list_of_open_tools[tool_idx].tool_output.lock() {
                            ui.add(
                                egui::TextEdit::multiline(&mut *tool_output)
                                    .id_source(&format!("out_{}-{}", &self.list_of_open_tools[tool_idx].tool_name, tool_idx))
                                    .cursor_at_end(true)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(8)
                                    .lock_focus(true)
                                    .desired_width(f32::INFINITY)
                            );

                            if let Ok(cm) = self.list_of_open_tools[tool_idx].continuous_mode.lock() {
                                if *cm {
                                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                                }
                            }
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        // ui.small(""); // just to add some vertical distance between it and the output text box.
                        if let Ok(progress) = self.list_of_open_tools[tool_idx].progress.lock() {
                            if let Ok(progress_label) = self.list_of_open_tools[tool_idx].progress_label.lock() {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.add(egui::ProgressBar::new(*progress)
                                    .desired_width(100.0)
                                    .show_percentage());

                                    ui.label(&*progress_label);
                                });
                            }
                        }
                    });
                });
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Run").clicked() {
                    self.list_of_open_tools[tool_idx].update_working_dir(&self.state.working_dir);
                    self.list_of_open_tools[tool_idx].update_exe_path(&self.state.whitebox_exe);
                    self.list_of_open_tools[tool_idx].run();
                }
                if ui.button("Cancel").clicked() {
                    self.list_of_open_tools[tool_idx].cancel();
                }
                if ui.button("Help").clicked() {
                    let toolbox = self.list_of_open_tools[tool_idx]
                    .toolbox
                    .replace("GIS", "Gis")
                    .replace("TIN", "Tin")
                    .replace("LiDAR", "Lidar")
                    .replace("/", "")
                    .replace(" ", "")
                    .to_snake();

                    let tool_name = self.list_of_open_tools[tool_idx]
                    .tool_name
                    .replace("GIS", "Gis")
                    .replace("TIN", "Tin")
                    .replace("LiDAR", "Lidar")
                    .replace("/", "")
                    .replace(" ", "");
                    let url = format!("https://www.whiteboxgeo.com/manual/wbt_book/available_tools/{}.html#{}", toolbox, tool_name);
                    println!("URL: {url}");
                    if !webbrowser::open(&url).is_ok() {
                        if let Ok(mut tool_output) = self.list_of_open_tools[tool_idx].tool_output.lock() {
                            tool_output.push_str("Could not navigate to help link in browser.\n");

                            let help_str = self.list_of_open_tools[tool_idx].get_tool_help();
                            if help_str.is_some() {
                                *tool_output = help_str.unwrap_or("".to_string());
                            }
                        }
                    }

                }
                if ui.button("View Code").clicked() {
                    // let url = self.view_code(&(self.list_of_open_tools[tool_idx].tool_name));
                    let output = std::process::Command::new(&self.state.whitebox_exe)
                            .args([&format!("--viewcode={}", self.list_of_open_tools[tool_idx].tool_name)])
                            .output()
                            .expect("Could not execute the WhiteboxTools binary");
                    
                    if output.status.success() {
                        let url = match std::str::from_utf8(&(output.stdout)) {
                            Ok(v) => v.to_string(),
                            Err(_) => "https://github.com/jblindsay/whitebox-tools".to_string(),
                        };
                        if !webbrowser::open(&url).is_ok() {
                            if let Ok(mut tool_output) = self.list_of_open_tools[tool_idx].tool_output.lock() {
                                tool_output.push_str("Could not navigate to code link in browser.\n");
                            }
                        }
                    } else {
                        println!("stdout: {}", std::str::from_utf8(output.stdout.as_slice()).unwrap_or("None"));
                        println!("stderr: {}", std::str::from_utf8(output.stderr.as_slice()).unwrap_or("None"));
                    }
                }
                if ui.button("Close").clicked() {
                    close_dialog = true;
                }

                // let progress = *(self.list_of_open_tools[tool_idx].progress).lock().unwrap_or(0.);
                // let progress_label = &*(self.list_of_open_tools[tool_idx].progress_label).lock().unwrap_or("Progress");
                // ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                //     ui.add(egui::ProgressBar::new(progress)
                //     .desired_width(100.0)
                //     .show_percentage());

                //     ui.label(progress_label);
                // });
            });

            if let Ok(cm) = self.list_of_open_tools[tool_idx].continuous_mode.lock() {
                if *cm {
                    ctx.request_repaint();
                }
            }
        });

        if wk_dir.len() > 0 {
            self.update_working_dir(&wk_dir);
        }

        if close_dialog {
            self.open_tools[tool_idx] = false;
        }
    }
}

fn check_geometry_type(parameter: &mut ToolParameter, working_dir: &str) {
    if !path::Path::new(&parameter.str_value).exists() {
        // prepend the working directory and see if that file exists.
        let f = path::Path::new(working_dir).join(&parameter.str_value);
        if f.exists() {
            parameter.str_value = f.to_str().unwrap_or("").to_string();
        }
    }
    // Read the file and make sure that it is the right geometry type.
    match Shapefile::read(&parameter.str_value) {
        Ok(vector_data) => {
            let base_shape_type = vector_data.header.shape_type.base_shape_type();
            // make sure the input vector file is of the right shape type
            let err_found = match parameter.geometry_type {
                VectorGeometryType::Point => {
                    let mut ret = false;
                    if base_shape_type != ShapeType::Point {
                        ret = true;
                    }
                    ret
                },
                VectorGeometryType::Line => {
                    let mut ret = false;
                    if base_shape_type != ShapeType::PolyLine {
                        ret = true;
                    }
                    ret
                },
                VectorGeometryType::Polygon => {
                    let mut ret = false;
                    if base_shape_type != ShapeType::Polygon {
                        ret = true;
                    }
                    ret
                },
                VectorGeometryType::LineOrPolygon => {
                    let mut ret = false;
                    if base_shape_type != ShapeType::PolyLine && base_shape_type != ShapeType::Polygon {
                        ret = true;
                    }
                    ret
                },
                _ => { false }
            };
            if err_found {
                if rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Warning)
                .set_title("Wrong Vector Geometry Type")
                .set_description("The specified file does not have the correct vector geometry type for this parameter. Do you want to continue?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show() {
                    // do nothing
                } else {
                    // Reset the parameter string value.
                    parameter.str_value = "".to_string();
                }
            }
        },
        Err(_) => {} // do nothing
    }
}

fn get_file_extensions(pft: &ParameterFileType) -> Vec<&str> {
    match pft {
        ParameterFileType::Lidar => {
            vec!["las", "laz", "zLidar"]
        },
        ParameterFileType::Raster => {
            vec!["tif", "tiff", "bil", "hdr", "flt", "sdat", "sgrd", "rdc", "rst", "grd", "txt", "asc", "tas", "dep"]
        },
        ParameterFileType::Vector => {
            vec!["shp"]
        },
        ParameterFileType::RasterAndVector => {
            vec!["shp", "tif", "tiff", "bil", "hdr", "flt", "sdat", "sgrd", "rdc", "rst", "grd", "txt", "asc", "tas", "dep"]
        },
        ParameterFileType::Text => {
            vec!["txt"]
        },
        ParameterFileType::Html => {
            vec!["html"]
        },
        ParameterFileType::Csv => {
            vec!["csv"]
        },
        ParameterFileType::Dat => {
            vec!["dat"]
        },
        _ => { 
            vec![]
        }
    }
}

fn get_file_dialog(pft: &ParameterFileType) -> rfd::FileDialog {
    match pft {
        ParameterFileType::Lidar => {
            rfd::FileDialog::new()
            .add_filter("Lidar Files", &["las", "laz", "zLidar"])
            .add_filter("LAS Files", &["las"])
            .add_filter("LAZ Files", &["laz"])
            .add_filter("zLidar Files", &["zLidar"])
        },
        ParameterFileType::Raster => {
            rfd::FileDialog::new()
            .add_filter("Raster Files", &["tif", "tiff", "bil", "hdr", "flt", "sdat", "sgrd", "rdc", "rst", "grd", "txt", "asc", "tas", "dep"])
            .add_filter("GeoTIFF Files", &["tif", "tiff"])
        },
        ParameterFileType::Vector => {
            rfd::FileDialog::new()
            .add_filter("Vector Files", &["shp"])
        },
        ParameterFileType::RasterAndVector => {
            rfd::FileDialog::new()
            .add_filter("Raster Files", &["tif", "tiff", "bil", "hdr", "flt", "sdat", "sgrd", "rdc", "rst", "grd", "txt", "asc", "tas", "dep"])
            .add_filter("Vector Files", &["shp"])
        },
        ParameterFileType::Text => {
            rfd::FileDialog::new()
            .add_filter("Test Files", &["txt"])
        },
        ParameterFileType::Html => {
            rfd::FileDialog::new()
            .add_filter("HTML Files", &["html"])
        },
        ParameterFileType::Csv => {
            rfd::FileDialog::new()
            .add_filter("CSV Files", &["csv"])
        },
        ParameterFileType::Dat => {
            rfd::FileDialog::new()
            .add_filter("DAT Files", &["dat"])
        },
        _ => { 
            rfd::FileDialog::new()
        }
    }
}
