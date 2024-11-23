use egui::{Color32, RichText, Ui};
use crate::frontend::{TabView, TabHandle};
use crate::shared::Shared;

pub struct FancyView {}

impl FancyView {
    pub fn new() -> Shared<Self> {
        Self {}.into()
    }
}

impl TabView for Shared<FancyView> {
    fn title(&self, tab: &TabHandle) -> String {
        format!("Fancy Tab {}", tab.node.0)
    }

    fn content(&mut self, ui: &mut Ui) {
        ui.label(
            RichText::new(format!(
                "Content of {}. This tab sure is fancy!",
                ""
            ))
                .italics()
                .size(20.0)
                .color(Color32::from_rgb(255, 128, 64)),
        );
    }

    fn as_tab_handle(&self, surface: egui_dock::SurfaceIndex, node: egui_dock::NodeIndex) -> TabHandle {
        TabHandle::new(self.clone().into(), surface, node)
    }
}