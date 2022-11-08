use std::collections::HashMap;

#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Tree {
    pub label: String, 
    pub children: Vec<Tree>,
}

impl Tree {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_owned(),
            children: vec![],
        }
    }

    pub fn add_child(&mut self, child: Tree) {
        self.children.push(child);
    }

    pub fn from_toolboxes_and_tools(toolboxes: Vec<&str>, toolbox_tools: HashMap<&str, Vec<&str>>) -> Self {
        let mut ret = Tree {
            label: "Toolboxes".to_owned(),
            children: vec![],
        };
        for i in 0..toolboxes.len() {
            let tb = toolboxes[i];
            let tools = &toolbox_tools[tb];
            if !tb.contains("/") {
                let mut t = Tree::new(tb);
                for j in i+1..toolboxes.len() {
                    if toolboxes[j].contains(tb) {
                        let sub_tb = toolboxes[j].split("/").collect::<Vec<&str>>();
                        let mut subt = Tree::new((sub_tb[sub_tb.len()-1]).trim());
                        let tools2 = &toolbox_tools[toolboxes[j]];
                        for tl in tools2 {
                            subt.add_child(Tree::new(tl));
                        }
                        t.children.push(subt);
                    } else {
                        break;
                    }
                }
                for tl in tools {
                    t.add_child(Tree::new(tl));
                }
                ret.children.push(t);
            }
        }

        ret
    }

    pub fn is_toolbox(&self) -> bool {
        self.children.len() > 0
    }
    
    // pub fn ui(&mut self, ui: &mut egui::Ui) {
    //     self.ui_impl(ui)
    // }
    
    // fn ui_impl(&mut self, ui: &mut egui::Ui) {
    //     if self.is_toolbox() {
    //         egui::CollapsingHeader::new(&self.label)
    //         .default_open(self.label == "Toolboxes")
    //         // .icon(circle_icon)
    //         .show(ui, |ui|
    //             for i in 0..self.children.len() {
    //                 self.children[i].ui_impl(ui);
    //             }
    //         );
    //     } else { // tool
    //         if ui.button(&format!("🔧 {}", self.label))
    //         .on_hover_text(&self.label)
    //         .clicked() {
    //             println!("Clicked {}", self.label);
    //         }
    //     }
    // }
}

// fn circle_icon(ui: &mut egui::Ui, openness: f32, response: &egui::Response) {
//     let stroke = ui.style().interact(&response).fg_stroke;
//     let radius = egui::lerp(1.0..=2.0, openness);
//     ui.painter().circle_filled(response.rect.center(), radius, stroke.color);
// }