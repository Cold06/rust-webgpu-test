use egui::{Color32, RichText};
use egui_dock::{NodeIndex, SurfaceIndex};

pub enum UITabKind {
    Regular,
    Fancy,
}

pub struct UITab {
    pub kind: UITabKind,
    pub surface: SurfaceIndex,
    pub node: NodeIndex,
}

impl UITab {
    pub fn regular(surface: SurfaceIndex, node: NodeIndex) -> Self {
        Self {
            kind: UITabKind::Regular,
            surface,
            node,
        }
    }

    pub fn fancy(surface: SurfaceIndex, node: NodeIndex) -> Self {
        Self {
            kind: UITabKind::Fancy,
            surface,
            node,
        }
    }

    pub fn title(&self) -> String {
        match self.kind {
            UITabKind::Regular => format!("Regular Tab {}", self.node.0),
            UITabKind::Fancy => format!("Fancy Tab {}", self.node.0),
        }
    }

    pub fn content(&self) -> RichText {
        match self.kind {
            UITabKind::Regular => {
                RichText::new(format!("Content of {}. This tab is ho-hum.", self.title()))
            }
            UITabKind::Fancy => RichText::new(format!(
                "Content of {}. This tab sure is fancy!",
                self.title()
            ))
                .italics()
                .size(20.0)
                .color(Color32::from_rgb(255, 128, 64)),
        }
    }
}


pub struct TabViewer<'a> {
    pub added_nodes: &'a mut Vec<UITab>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = UITab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(tab.content());
    }

    fn add_popup(&mut self, ui: &mut egui::Ui, surface: SurfaceIndex, node: NodeIndex) {
        ui.set_min_width(120.0);
        ui.style_mut().visuals.button_frame = false;

        if ui.button("Regular tab").clicked() {
            self.added_nodes.push(UITab::regular(surface, node));
        }

        if ui.button("Fancy tab").clicked() {
            self.added_nodes.push(UITab::fancy(surface, node));
        }
    }
}


