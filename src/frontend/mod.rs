mod world_view;
mod regular_view;
mod fancy_view;

use egui_dock::{NodeIndex, SurfaceIndex};
use enum_dispatch::enum_dispatch;

pub use world_view::WorldView;
pub use fancy_view::FancyView;
pub use regular_view::RegularView;

use crate::shared::Shared;

#[enum_dispatch(TabInstance)]
pub trait TabView {
    fn title(&self, tab: &TabHandle) -> String;

    fn content(&mut self, ui: &mut egui::Ui);

    fn as_tab_handle(&self, surface: SurfaceIndex, node: NodeIndex) -> TabHandle;
}

#[enum_dispatch]
pub enum TabInstance {
    RegularView(Shared<RegularView>),
    FancyView(Shared<FancyView>),
    WorldView(Shared<WorldView>),
}

pub struct TabHandle {
    pub kind: TabInstance,
    pub surface: SurfaceIndex,
    pub node: NodeIndex,
}

impl TabHandle {
    pub fn new(kind: TabInstance, surface: SurfaceIndex, node: NodeIndex) -> Self {
        Self {
            kind,
            surface,
            node,
        }
    }
    pub fn title(&self) -> String {
        format!("{} - {}", self.kind.title(self), self.node.0)
    }

    pub fn content(&mut self, ui: &mut egui::Ui) {
        self.kind.content(ui);
    }
}

pub struct HandleList<'a>(pub &'a mut Vec<TabHandle>);

impl egui_dock::TabViewer for HandleList<'_> {
    type Tab = TabHandle;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.content(ui);
    }

    fn add_popup(&mut self, ui: &mut egui::Ui, surface: SurfaceIndex, node: NodeIndex) {
        ui.set_min_width(120.0);
        ui.style_mut().visuals.button_frame = false;

        if ui.button("Regular tab").clicked() {
            let tab = RegularView::new();
            self.0.push(tab.as_tab_handle(surface, node));
        }

        if ui.button("Fancy tab").clicked() {
            let tab = FancyView::new();
            self.0.push(tab.as_tab_handle(surface, node));
        }
    }
}
