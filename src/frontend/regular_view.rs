use egui::{RichText, Ui};
use crate::egui_tools::EguiRenderer;
use crate::frontend::{TabView, TabHandle};
use crate::frontend::fancy_view::FancyView;
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
}