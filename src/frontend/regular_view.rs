use egui::{RichText, Ui};
use crate::egui_tools::EguiRenderer;
use crate::frontend::{TabView, UITab};

pub struct RegularView {}

impl TabView for RegularView {
    fn title(&self, tab: &UITab) -> String {
        format!("Regular Tab {}", tab.node.0)
    }

    fn content(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(format!(
            "Content of {}. This tab is ho-hum.",
            ""
        )));
    }
}