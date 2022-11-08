use crate::MyApp;
use egui::{ CollapsingHeader, ScrollArea };

impl MyApp {
    pub fn tools_panel(&mut self, ctx: &egui::Context) {
        // Tool tree side panel
        egui::SidePanel::left("tool_panel").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(&format!("🛠 {} Available Tools", self.num_tools));
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.toggle_value(&mut self.state.show_toolboxes, "Toolboxes")
                .on_hover_text("Search for tools in their toolboxes")
                .clicked() {
                    self.state.show_toolboxes = true;
                    self.state.show_tool_search = false;
                    self.state.show_recent_tools = false;
                }
                if ui.toggle_value(&mut self.state.show_tool_search, "Tool Search")
                .on_hover_text("Search for tools by keywords")
                .clicked() {
                    self.state.show_toolboxes = false;
                    self.state.show_tool_search = true;
                    self.state.show_recent_tools = false;
                }
                if ui.toggle_value(&mut self.state.show_recent_tools, "Recent Tools")
                .on_hover_text("List recently used and most used tools.")
                .clicked() {
                    self.state.show_toolboxes = false;
                    self.state.show_tool_search = false;
                    self.state.show_recent_tools = true;
                }
            });
            ui.separator();
                    
            let mut clicked_tool = String::new();
            ui.vertical(|ui| {
                if self.state.show_toolboxes {
                    ScrollArea::vertical()
                    .max_height(f32::INFINITY)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        // let mut clicked_tool = String::new();
                        // self.tree.ui(ui); // This is a recursive approach that works better, but can't access MyApp.

                        // What follows is truly awful and fragile code. It relies on the fact that
                        // there are only 1-level sub-folders and no 2-level sub-folders. Should this
                        // ever change in the future, this would need to be updated.
                        CollapsingHeader::new(&self.tree.label)
                        .default_open(&self.tree.label == "Toolboxes")
                        // .icon(circle_icon)
                        .show(ui, |ui| {
                            // render the toolboxes
                            for i in 0..self.tree.children.len() {

                                let tree = &self.tree.children[i];
                                CollapsingHeader::new(&tree.label)
                                .default_open(false)
                                // .icon(circle_icon)
                                .show(ui, |ui| {
                                    for j in 0..tree.children.len() {
                                        let tree2 = &tree.children[j];
                                        if tree2.is_toolbox() {
                                            CollapsingHeader::new(&tree2.label)
                                            .default_open(false)
                                            // .icon(circle_icon)
                                            .show(ui, |ui| {
                                                for k in 0..tree2.children.len() {
                                                    let tree3 = &tree2.children[k];
                                                    let tool_index = *self.tool_order.get(&tree3.label.clone()).unwrap();

                                                    if ui.toggle_value(&mut self.open_tools[tool_index], &format!("🔧 {}", tree3.label))
                                                    .on_hover_text(self.tool_descriptions.get(&tree3.label).unwrap_or(&String::new()))
                                                    .clicked() {
                                                        self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                                        clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                                        // self.update_recent_tools(&tn);
                                                    }
                                                }
                                            });
                                        } else { // it's a tool
                                            let tool_index = *self.tool_order.get(&tree2.label.clone()).unwrap();
                                            if ui.toggle_value(&mut self.open_tools[tool_index], &format!("🔧 {}", tree2.label))
                                            .on_hover_text(self.tool_descriptions.get(&tree2.label).unwrap_or(&String::new()))
                                            .clicked() {
                                                self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                                clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                                // self.update_recent_tools(&tn);
                                            }
                                        }
                                    }
                                });
                            }
                        });

                        // if !clicked_tool.is_empty() {
                        //     self.update_recent_tools(&clicked_tool);
                        // }

                        // let margin = ui.visuals().clip_rect_margin;
                        // let current_scroll = ui.clip_rect().top() - ui.min_rect().top() + margin;
                        // let max_scroll = ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
                        // (current_scroll, max_scroll)
                    })
                    .inner;

                } else if self.state.show_tool_search {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            // ui.label("Keywords:")
                            // .on_hover_text("Search for keywords (separated by commas) in tool names or descriptions");
                            ui.label(egui::RichText::new("Keywords:")
                            .italics()
                            .strong()
                            .color(ui.visuals().hyperlink_color))
                            .on_hover_text("Search for keywords (separated by commas) in tool names or descriptions");

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Clear").on_hover_text("Clear search keywords").clicked() {
                                    self.search_words_str = "".to_string();
                                }
                            });
                        });
                        
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search_words_str)
                            .desired_width(self.state.textbox_width)
                            
                            // .on_hover_text("Search for keywords (separated by commas) in tool names or descriptions");
                        );

                        if !self.search_words_str.trim().is_empty() {
                            ScrollArea::vertical()
                            .max_height(f32::INFINITY)
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                // Perform the search...
                                let search_words = self.search_words_str.split(",").collect::<Vec<&str>>();
                                let mut hs = std::collections::HashSet::new();
                                for k in 0..search_words.len() {
                                    let sw = search_words[k].trim().to_lowercase();
                                    for tool_info in &self.tool_info {
                                        let tn = tool_info.tool_name.to_string();
                                        let desc = self.tool_descriptions.get(&tn).unwrap_or(&String::new()).to_lowercase();
                                        if tn.to_lowercase().contains(&sw) || desc.to_lowercase().contains(&sw) {
                                            hs.insert(tn);
                                        }
                                    }
                                }

                                let mut tools: Vec<_> = hs.into_iter().collect();
                                tools.sort();

                                for tool in tools {
                                    // ui.label(format!("{}", tool));
                                    let tool_index = *self.tool_order.get(&tool).unwrap();
                                    if ui.toggle_value(&mut self.open_tools[tool_index], &tool)
                                    .on_hover_text(self.tool_descriptions.get(&tool).unwrap_or(&String::new()))
                                    .clicked() {
                                        self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                        // let tn = self.tool_info[tool_index].tool_name.clone();
                                        // self.update_recent_tools(&tn);
                                        clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                    }
                                }

                                // let margin = ui.visuals().clip_rect_margin;

                                // let current_scroll2 = ui.clip_rect().top() - ui.min_rect().top() + margin;
                                // let max_scroll2 = ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
                                // (current_scroll2, max_scroll2)
                            })
                            .inner;
                        }
                    });
                } else if self.state.show_recent_tools {
                    ui.vertical(|ui| {
                        ScrollArea::vertical()
                        .id_source("recently_used_tools")
                        .max_height(f32::INFINITY)
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // ui.label("Recently used tools:");
                                ui.label(egui::RichText::new("Recently used tools:")
                                .italics()
                                .strong()
                                .color(ui.visuals().hyperlink_color));

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Clear").on_hover_text("Clear recent tools").clicked() {
                                        self.state.most_recent.clear();
                                    }
                                });
                            });

                            for tool in &self.state.most_recent {
                                // ui.label(format!("{}", tool));
                                let tool_index = *self.tool_order.get(tool).unwrap();
                                if ui.toggle_value(&mut self.open_tools[tool_index], tool)
                                .on_hover_text(self.tool_descriptions.get(tool).unwrap_or(&String::new()))
                                .clicked() {
                                    self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                    // let tn = self.tool_info[tool_index].tool_name.clone();
                                    // self.update_recent_tools(&tn);
                                    // clicked_tool = self.tool_info[tool_index].tool_name.clone();
                                }
                            }

                            ui.separator();
                            ui.horizontal(|ui| {
                                // ui.label("Most-used tools:");
                                ui.label(egui::RichText::new("Most-used tools:")
                                .italics()
                                .strong()
                                .color(ui.visuals().hyperlink_color));

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Clear").on_hover_text("Clear most-used tools").clicked() {
                                        self.most_used.clear();
                                        self.most_used_hm.clear();
                                    }
                                });
                            });

                            for val in &self.most_used {
                                let tool_index = *self.tool_order.get(&val.1).unwrap();
                                if ui.toggle_value(&mut self.open_tools[tool_index], &format!("{} ({})", val.1, val.0))
                                .on_hover_text(self.tool_descriptions.get(&val.1).unwrap_or(&String::new()))
                                .clicked() {
                                    self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                                }
                            }
                        });
                    }).inner;

                    // ui.vertical(|ui| {
                    //     ui.horizontal(|ui| {
                    //         ui.label("Recently used tools:");
                    //         ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    //             if ui.button("Clear").on_hover_text("Clear recent tools").clicked() {
                    //                 self.state.most_recent.clear();
                    //             }
                    //         });
                    //     });

                    //     ScrollArea::vertical()
                    //     .id_source("recently_used_tools")
                    //     .max_height(300.0) //f32::INFINITY)
                    //     .auto_shrink([false; 2])
                    //     .show(ui, |ui| {
                    //         for tool in &self.state.most_recent {
                    //             // ui.label(format!("{}", tool));
                    //             let tool_index = *self.tool_order.get(tool).unwrap();
                    //             if ui.toggle_value(&mut self.open_tools[tool_index], tool)
                    //             .on_hover_text(self.tool_descriptions.get(tool).unwrap_or(&String::new()))
                    //             .clicked() {
                    //                 self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                    //             }
                    //         }
                    //     })
                    //     .inner;

                    //     ui.separator();
                    //     ui.horizontal(|ui| {
                    //         ui.label("Most used tools:");
                    //         ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    //             if ui.button("Clear").on_hover_text("Clear recent tools").clicked() {
                    //                 self.state.most_recent.clear();
                    //             }
                    //         });
                    //     });

                    //     ScrollArea::vertical()
                    //     .id_source("most_used_tools")
                    //     .max_height(300.0) //f32::INFINITY)
                    //     .auto_shrink([false; 2])
                    //     .show(ui, |ui| {
                    //         ui.label("Hello");
                    //     //     for tool in &self.state.most_recent {
                    //     //         // ui.label(format!("{}", tool));
                    //     //         let tool_index = *self.tool_order.get(tool).unwrap();
                    //     //         if ui.toggle_value(&mut self.open_tools[tool_index], tool)
                    //     //         .on_hover_text(self.tool_descriptions.get(tool).unwrap_or(&String::new()))
                    //     //         .clicked() {
                    //     //             self.tool_info[tool_index].update_exe_path(&self.state.whitebox_exe);
                    //     //         }
                    //     //     }
                    //     })
                    //     .inner;
                    // });
                }
            });

            if !clicked_tool.is_empty() {
                self.update_recent_tools(&clicked_tool);
            }
            
        });
    }
}