mod about;
mod custom_widgets;
mod extension;
mod settings_panel;
mod tool_dialog;
mod tools_panel;
mod tree;

pub use custom_widgets::{ toggle };
pub use tree::Tree;
use about::WbLogo;
use anyhow::{bail, Result};
use extension::ExtensionInstall;
use std::collections::{ HashMap, HashSet, VecDeque };
use std::{env, path::Path};
use std::process::Command;
use serde_json::Value;
use tool_dialog::ToolInfo;
use eframe::egui;
use egui::CentralPanel;
use egui::FontFamily::Proportional;
use egui::FontId;
use egui::TextStyle::*;

static mut CLEAR_STATE: bool = false;

fn main() {
    // command-line args
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        for arg in args {
            if arg.trim().to_lowercase().contains("clear_state") {
                unsafe {
                    // This is the only way that I can see to pass command-line args to the eframe app.
                    CLEAR_STATE = true;
                }
            }
        }
    }

    let mut dir = env::current_exe().unwrap_or(Path::new("").to_path_buf());
    dir.pop();
    let img_directory = dir.join("img");
    let icon_file = img_directory.join("WBT_icon.png");
    let icon_data = if icon_file.exists() {
        // Some(load_icon(&icon_file.to_str().unwrap_or("No exe path found.").replace("\"", "")))
        match load_icon(&icon_file.to_str().unwrap_or("No exe path found.").replace("\"", "")) {
            Ok(v) => Some(v),
            Err(_) => None
        }
    } else {
        None
    };
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(1000.0, 700.0)),
        drag_and_drop_support: true,
        icon_data: icon_data,
        ..Default::default()
    };

    eframe::run_native(
        "Whitebox Runner",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    );
}

/// The state that we persist (serialize).
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct AppState {
    theme: AppTheme,
    settings_visible: bool,
    body_font_size: f32,
    header_font_size: f32,
    whitebox_exe: String,
    working_dir: String,
    view_tool_output: bool,
    max_procs: isize,
    compress_rasters: bool,
    textbox_width: f32,
    output_command: bool, // whether or not to display the tool raw command line
    show_toolboxes: bool,
    show_tool_search: bool,
    show_recent_tools: bool,
    most_recent: VecDeque<String>,
}

#[derive(Default)]
struct MyApp {
    state: AppState,
    num_tools: usize,
    tree: Tree,
    allowed_to_close: bool,
    show_confirmation_dialog: bool,
    list_of_open_tools: Vec<ToolInfo>,
    open_tools: Vec<bool>,
    tool_info: Vec<ToolInfo>,
    tool_descriptions: HashMap<String, String>,
    tool_order: HashMap<String, usize>,
    installed_extensions: InstalledExtensions,
    theme_changed: bool,
    fonts_changed: bool,
    wbt_version: String,
    search_words_str: String,
    about_visible: bool,
    extension_visible: bool,
    case_sensitive_search: bool,
    num_search_hits: usize,
    ei: ExtensionInstall,
    most_used_hm: HashMap<String, u16>,
    most_used: Vec<(u16, String)>,
    wb_logo: WbLogo,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut slf = Self::default();

        let clear_state: bool;
        unsafe {
            clear_state = CLEAR_STATE;
        }

        if clear_state {
            // Initialize state manually
            slf.state.theme = AppTheme::Dark;
            slf.state.settings_visible = false;
            slf.state.body_font_size = 14.0;
            slf.state.header_font_size = 18.0;
            slf.state.working_dir = "/".to_string();
            slf.state.view_tool_output = true;
            slf.state.max_procs = -1;
            slf.state.compress_rasters = true;
            slf.state.textbox_width = 230.0;
            slf.state.output_command = false;
            slf.state.show_toolboxes = true;
            slf.state.show_tool_search = false;
            slf.state.show_recent_tools = false;
            slf.state.most_recent = std::collections::VecDeque::new();
        } else {
            #[cfg(feature = "persistence")]
            if let Some(storage) = cc.storage {
                if let Some(state) = eframe::get_value(storage, eframe::APP_KEY) {
                    slf.state = state;
                } else {
                    // Initialize state manually
                    slf.state.theme = AppTheme::Dark;
                    slf.state.settings_visible = false;
                    slf.state.body_font_size = 14.0;
                    slf.state.header_font_size = 18.0;
                    slf.state.working_dir = "/".to_string();
                    slf.state.view_tool_output = true;
                    slf.state.max_procs = -1;
                    slf.state.compress_rasters = true;
                    slf.state.textbox_width = 230.0;
                    slf.state.output_command = false;
                    slf.state.show_toolboxes = true;
                    slf.state.show_tool_search = false;
                    slf.state.show_recent_tools = false;
                    slf.state.most_recent = std::collections::VecDeque::new();
                }
            }
        }
        
        slf.theme_changed = true;
        slf.fonts_changed = true;
        slf.state.whitebox_exe = slf.get_executable_path().unwrap_or("".to_string());
        if slf.state.working_dir.is_empty() {
            slf.state.working_dir = "/".to_owned();
        }
        _ = slf.get_tool_info();
        _ = slf.get_version();
        slf.ei = ExtensionInstall::new();
        slf
    }

    fn refesh_tools(&mut self) {
        // reset the various arrays/hashmaps
        self.list_of_open_tools.clear();
        self.open_tools.clear();
        self.tool_info.clear();
        self.tool_descriptions.clear();
        self.tool_order.clear();
        self.most_used_hm.clear();
        self.most_used.clear();
        self.state.most_recent.clear();

        _ = self.get_tool_info();
        _ = self.get_version();
    }

    // Get the tools and toolboxes
    fn get_tool_info(&mut self) -> Result<()> {
        // Start by getting the executable path
        let exe = self.get_executable_path().unwrap_or("".to_string());
        
        let output = Command::new(&exe)
                .args(["--toolbox"])
                .output()?;

        let mut tool_list = vec![];
        let mut toolboxes = HashSet::new();
        if output.status.success() {
            let s = std::str::from_utf8(&(output.stdout))?;
            let tool_data = s.split("\n").collect::<Vec<&str>>();
            for tool in tool_data {
                if !tool.trim().is_empty() {
                    let tool_and_box = tool.split(":").collect::<Vec<&str>>();
                    tool_list.push((tool_and_box[0].trim(), tool_and_box[1].trim()));
                    toolboxes.insert(tool_and_box[1].trim());
                }
            }
        } else {
            bail!("Could not execute the WhiteboxTools binary");
        }

        let mut tb: Vec<_> = toolboxes.into_iter().collect();
        tb.sort();

        let mut tb_hm = HashMap::new();
        for i in 0..tb.len() {
            let tlbx = tb[i].clone();
            let mut v = vec![];
            for j in 0..tool_list.len() {
                if tool_list[j].1 == tlbx {
                    v.push(tool_list[j].0);
                }
            }
            tb_hm.insert(tlbx, v);
        }

        // Get the tool descriptions
        let output = Command::new(exe)
                .args(["--listtools"])
                .output()?;

        let mut tool_descriptions = HashMap::new();
        if output.status.success() {
            let s = std::str::from_utf8(&(output.stdout))?;
            let tool_data = s.split("\n").collect::<Vec<&str>>();
            for tool in tool_data {
                if !tool.trim().is_empty() {
                    let tool_and_desc = tool.split(":").collect::<Vec<&str>>();
                    tool_descriptions.insert(tool_and_desc[0].trim().to_owned(), tool_and_desc[1].trim().to_owned());
                }
            }
        } else {
            bail!("Could not execute the WhiteboxTools binary");
        }

        let mut tool_order = HashMap::new();
        for i in 0..tool_list.len() {
            tool_order.insert(tool_list[i].0.to_owned(), i);
        }

        let mut num_tools = 0;
        for i in 0..tool_list.len() {
            let json_value = self.get_tool_parameters(tool_list[i].0)?; // Add the tool parameters JSON object to the tool info
            // self.open_tools.push(false);
            self.tool_info.push(ToolInfo::new(tool_list[i].0, tool_list[i].1, json_value));
            self.tool_info[num_tools].update_output_command(self.state.output_command);
            self.tool_info[num_tools].update_verbose_mode(self.state.view_tool_output);
            self.tool_info[num_tools].update_compress_rasters(self.state.compress_rasters);
            num_tools += 1;
        }

        let mut installed_extensions = InstalledExtensions::default();
        installed_extensions.gte = tool_order.contains_key("RandomForestClassification");
        if !installed_extensions.gte {
            if tool_order.contains_key("Curvedness") {
                installed_extensions.dem = true;
            }
            if tool_order.contains_key("ModifyLidar") {
                installed_extensions.lidar = true;
            }
            if tool_order.contains_key("YieldMap") {
                installed_extensions.agriculture = true;
            }
        }

        self.num_tools = tool_list.len();
        self.tree = Tree::from_toolboxes_and_tools(tb, tb_hm);
        self.tool_descriptions = tool_descriptions;
        self.tool_order = tool_order;
        self.installed_extensions = installed_extensions;
        self.most_used_hm = HashMap::new(); // just to initialize
        self.most_used = vec![]; // just to initialize

        Ok(())
    }

    fn get_version(&mut self) -> Result<()> {
        // Start by getting the executable path
        if let Some(exe) = self.get_executable_path() {
            let output = Command::new(&exe)
                .args(["--version"])
                .output()?;
        
            if output.status.success() {
                let s = std::str::from_utf8(&(output.stdout))?;
                let version_data = s.split("\n").collect::<Vec<&str>>();
                self.wbt_version = version_data[0].to_string();
                return Ok(());
            } else {
                println!("stdout: {}", std::str::from_utf8(output.stdout.as_slice()).unwrap_or("No message"));
                println!("stderr: {}", std::str::from_utf8(output.stderr.as_slice()).unwrap_or("No message"));
                bail!("Could not execute the WhiteboxTools binary");
            }
        } else {
            self.wbt_version = "Unknown version".to_string();
        }

        Ok(())
    }

    fn get_tool_parameters(&self, tool_name: &str) -> Result<Value> {
        let exe = self.get_executable_path().unwrap_or("".to_string());
        let output = Command::new(&exe)
            .args([&format!("--toolparameters={}", tool_name)])
            .output()?;
    
        let ret: Value;
        if output.status.success() {
            let s = std::str::from_utf8(&(output.stdout))?;
            ret = serde_json::from_str(s).unwrap_or(Value::Null);
        } else {
            println!("stdout: {}", std::str::from_utf8(output.stdout.as_slice()).unwrap_or("No message"));
            println!("stderr: {}", std::str::from_utf8(output.stderr.as_slice()).unwrap_or("No message"));
            bail!("Error running toolparameters command");
        }
        Ok(ret)
    }

    fn get_executable_path(&self) -> Option<String> {
        if self.state.whitebox_exe.is_empty() || !Path::new(&self.state.whitebox_exe).exists() {

            let mut dir = env::current_exe().unwrap_or(Path::new("").to_path_buf());
            dir.pop();

            let exe = dir.join(&format!("whitebox_tools{}", env::consts::EXE_SUFFIX));

            // check that it exists.
            if !exe.exists() {
                // bail!("Could not locate a local whitebox_tools executable in the Whitebox Runner directory.");
                return None;
            }

            Some(exe.to_str().unwrap_or("").to_string())
        } else {
            Some(self.state.whitebox_exe.clone())
        }
    }

    fn set_max_procs(&mut self) -> Result<()> {
        // Start by getting the executable path
        if let Some(exe) = self.get_executable_path() {
            let output = Command::new(&exe)
                .args([&format!("--max_procs={}", self.state.max_procs)])
                .output()?;
        
            if !output.status.success() {
                println!("stdout: {}", std::str::from_utf8(output.stdout.as_slice()).unwrap_or("No message"));
                println!("stderr: {}", std::str::from_utf8(output.stderr.as_slice()).unwrap_or("No message"));
                bail!("Error running --max_procs");
            }
        }

        Ok(())
    }

    fn update_recent_tools(&mut self, tool_name: &str) {
        let max_num = 15;
        if self.state.most_recent.len() == max_num {
            _ = self.state.most_recent.pop_back();
        }
        self.state.most_recent.push_front(tool_name.to_string());

        // most used
        if let Some(count) = self.most_used_hm.get(tool_name) {
            self.most_used_hm.insert(tool_name.to_string(), count + 1);
        } else {
            self.most_used_hm.insert(tool_name.to_string(), 1);
        };

        self.most_used = self.most_used_hm.iter().map(|v| (*v.1, v.0.to_string())).collect::<Vec<(u16, String)>>(); // self.most_used_hm.iter().map().collect();
        self.most_used.sort_by(|a, b| b.cmp(a));

        if self.tool_order.get(tool_name).is_some() {
            let tool_idx = *self.tool_order.get(tool_name).unwrap();
            let mut tool_info = self.tool_info[tool_idx].clone();
            tool_info.update_exe_path(&self.state.whitebox_exe);
            self.list_of_open_tools.push(tool_info);
            self.open_tools.push(true);
        }
    }
}

impl eframe::App for MyApp {
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }

    fn on_close_event(&mut self) -> bool {
        self.show_confirmation_dialog = true;
        self.allowed_to_close
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.theme_changed {
            // update the app theme
            match self.state.theme {
                AppTheme::Light => ctx.set_visuals(egui::Visuals::light()),
                AppTheme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            };
            self.theme_changed = false;
        }

        if self.fonts_changed {
            let mut style = (*ctx.style()).clone();

            // Redefine text_styles
            style.text_styles = [
            (Heading, FontId::new(self.state.header_font_size, Proportional)),
            // (Name("Heading2".into()), FontId::new(18.0, Proportional)),
            // (Name("Context".into()), FontId::new(14.0, Proportional)),
            (Body, FontId::new(self.state.body_font_size, Proportional)),
            (Monospace, FontId::new(self.state.body_font_size, egui::FontFamily::Monospace)),
            (Button, FontId::new(self.state.body_font_size, Proportional)),
            (Small, FontId::new(10.0, Proportional)),
            ].into();

            // Mutate global style with above changes
            ctx.set_style(style);
            self.fonts_changed = false;
        }
        

        CentralPanel::default().show(ctx, |ui| {
            // // Top menu panel
            // egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
            //     ui.horizontal(|ui| {
            //         egui::menu::bar(ui, |ui| {
            //             ui.menu_button("File", |ui| {
            //                 if ui.button("Close").clicked() {
            //                     frame.close();
            //                 }
            //             });

            //             ui.menu_button("Help", |ui| {
            //                 if ui.button("About").clicked() {
            //                     // ...
            //                 }
            //             });
            //         });
            //     });
            // });

            ui.horizontal(|_| {
                /*
                // Bottom panel
                egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        // if ui.visuals().dark_mode {
                        //     ui.horizontal(|ui| {
                        //         if ui.button("☀").on_hover_text("Switch to light mode").clicked() {
                        //             self.theme_changed = true;
                        //             self.state.theme = AppTheme::Light;
                        //         }
                        //     });
                        // } else {
                        //     ui.horizontal(|ui| {
                        //         if ui.button("🌙").on_hover_text("Switch to dark mode").clicked() {
                        //             self.theme_changed = true;
                        //             self.state.theme = AppTheme::Dark;
                        //         }
                        //     });
                        // }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(egui::ProgressBar::new(0.0)
                            .desired_width(85.)
                            .show_percentage());
                            ui.label("Progress:");
                        })
                    });
                });
                */

                // Tools panel
                self.tools_panel(ctx);

                // Top button panel
                egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        
                        // if ui.button("Close").clicked() {
                        //     frame.close();
                        // }
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.toggle_value(&mut self.about_visible, "ℹ")
                            .on_hover_text("About Whitebox Runner");
                            // .clicked() {
                            //     // Open About window.
                            //     self.about_visible = true;
                            // }
                            
                            ui.toggle_value(&mut self.state.settings_visible, "⛭") // ⚙
                            .on_hover_text("View settings");
                            // .clicked() {
                            //     self.state.settings_visible = !self.state.settings_visible;
                            // }

                            if ui.visuals().dark_mode {
                                ui.horizontal(|ui| {
                                    if ui.button("☀").on_hover_text("Switch to light mode").clicked() {
                                        self.theme_changed = true;
                                        self.state.theme = AppTheme::Light;
                                    }
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    if ui.button("🌙").on_hover_text("Switch to dark mode").clicked() {
                                        self.theme_changed = true;
                                        self.state.theme = AppTheme::Dark;
                                    }
                                });
                            }
                        });
                    });
                });

                // Settings panel
                if self.state.settings_visible {
                    self.settings_panel(ctx);
                }

                // Main area panel
                CentralPanel::default().show(ctx, |_| {
                    if self.about_visible {
                        self.about_window(ctx);
                    }
                    if self.extension_visible {
                        self.install_extension(ctx);
                    }

                    let mut remove_idx = -1isize;
                    for i in 0..self.list_of_open_tools.len() {
                        if self.open_tools[i] {
                            self.tool_dialog(ctx, i);
                        } else {
                            remove_idx = i as isize;
                        }
                    }
                    if remove_idx >= 0 {
                        self.list_of_open_tools.remove(remove_idx as usize);
                        self.open_tools.remove(remove_idx as usize);
                    }
                });
            });
        });

        // close the window?
        if self.show_confirmation_dialog {
            // Show confirmation dialog:
            // egui::Window::new("Do you want to quit?")
            //     .collapsible(false)
            //     .resizable(false)
            //     .show(ctx, |ui| {
            //         ui.horizontal(|ui| {
            //             if ui.button("Cancel").clicked() {
            //                 self.show_confirmation_dialog = false;
            //             }

            //             if ui.button("Yes").clicked() {
            //                 self.allowed_to_close = true;
            //                 frame.close();
            //             }
            //         });
            //     });
            if rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title("Closing Whitebox Runner")
            .set_description("Are you sure that you want to quit the application?")
            .set_buttons(rfd::MessageButtons::YesNo)
            .show() {
                self.allowed_to_close = true;
                frame.close();
            } else {
                self.show_confirmation_dialog = false;
            }
        }
    }    
}

/// Something to view in the demo windows
pub trait View {
    fn ui(&mut self, ui: &mut egui::Ui);
}

/// Something to view
pub trait Tool {
    /// `&'static` so we can also use it as a key to store open/close state.
    fn name(&self) -> &'static str;

    /// Show windows, etc
    fn show(&mut self, ctx: &egui::Context, open: &mut bool);
}

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum AppTheme {
    #[default]
    Light,
    Dark,
}

#[derive(Default)]
struct InstalledExtensions {
    gte: bool,
    lidar: bool,
    dem: bool,
    agriculture: bool,
}

fn load_icon(path: &str) -> Result<eframe::IconData> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)?.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    Ok(eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    })
}