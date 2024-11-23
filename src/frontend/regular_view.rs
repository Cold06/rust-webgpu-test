use egui::{RichText, Ui};
use crate::frontend::{TabView, TabHandle};
use crate::shared::Shared;

pub struct RegularView {}

impl RegularView {
    pub fn new() -> Shared<Self> {
        Self {}.into()
    }
}

impl TabView for Shared<RegularView> {
    fn title(&self, tab: &TabHandle) -> String {
        format!("Regular Tab {}", tab.node.0)
    }

    fn content(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(format!(
            "Content of {}. This tab is ho-hum.",
            ""
        )));
    }

    fn as_tab_handle(&self, surface: egui_dock::SurfaceIndex, node: egui_dock::NodeIndex) -> TabHandle {
        TabHandle::new(self.clone().into(), surface, node)
    }
}