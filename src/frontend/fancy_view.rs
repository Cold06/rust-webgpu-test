use egui::{Color32, RichText, Ui};
use crate::frontend::{TabView, UITab};

pub struct FancyView {}

impl TabView for FancyView {
    fn title(&self, tab: &UITab) -> String {
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
}