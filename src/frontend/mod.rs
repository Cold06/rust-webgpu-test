mod world_view;
mod regular_view;
mod fancy_view;

use egui_dock::{NodeIndex, SurfaceIndex};
use enum_dispatch::enum_dispatch;
use crate::frontend::fancy_view::FancyView;
use crate::frontend::regular_view::RegularView;

pub use world_view::{WorldView};
use crate::shared::Shared;

#[enum_dispatch(UITabKind)]
pub trait TabView {
    fn title(&self, tab: &UITab) -> String;

    fn content(&mut self, ui: &mut egui::Ui);

}

#[enum_dispatch]
pub enum UITabKind {
    RegularView,
    FancyView,
    Test(Shared<WorldView>),
}

pub struct UITab {
    pub kind: UITabKind,
    pub surface: SurfaceIndex,
    pub node: NodeIndex,
}

impl UITab {
    pub fn regular(surface: SurfaceIndex, node: NodeIndex) -> Self {
        Self {
            kind: RegularView{}.into(),
            surface,
            node,
        }
    }

    pub fn fancy(surface: SurfaceIndex, node: NodeIndex) -> Self {
        Self {
            kind: FancyView{}.into(),
            surface,
            node,
        }
    }

    // pub fn custom(surface: SurfaceIndex, node: NodeIndex, fn_once: Box<dyn FnMut(&mut egui::Ui)>) -> Self {
    //     Self {
    //         kind: Shared::new(UITabKind::Custom(fn_once)),
    //         surface,
    //         node,
    //     }
    // }

    pub fn world_view(surface: SurfaceIndex, node: NodeIndex, kind: Shared<WorldView>) -> Self {
        Self {
            kind: UITabKind::Test(kind),
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

pub struct TabViewer<'a> {
    pub added_nodes: &'a mut Vec<UITab>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = UITab;

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
            self.added_nodes.push(UITab::regular(surface, node));
        }

        if ui.button("Fancy tab").clicked() {
            self.added_nodes.push(UITab::fancy(surface, node));
        }
    }
}
