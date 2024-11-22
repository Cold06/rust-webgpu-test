mod world_view;
mod regular_view;
mod fancy_view;

use std::cell::RefCell;
use std::rc::Rc;
use egui_dock::{NodeIndex, SurfaceIndex};
use crate::egui_tools::EguiRenderer;
use crate::frontend::fancy_view::FancyView;
use crate::frontend::regular_view::RegularView;

pub use world_view::{WorldView};

pub trait TabView {
    fn title(&self, tab: &UITab) -> String;

    fn content(&mut self, ui: &mut egui::Ui);

}

pub enum UITabKind {
    Regular(RegularView),
    Fancy(FancyView),
    WorldView(WorldView),
    Custom(Box<dyn FnMut(&mut egui::Ui)>),
}

pub struct UITab {
    pub kind: Rc<RefCell<UITabKind>>,
    pub surface: SurfaceIndex,
    pub node: NodeIndex,
}

impl UITab {
    pub fn regular(surface: SurfaceIndex, node: NodeIndex) -> Self {
        Self {
            kind: Rc::new(RefCell::new(UITabKind::Regular(RegularView{}))),
            surface,
            node,
        }
    }

    pub fn fancy(surface: SurfaceIndex, node: NodeIndex) -> Self {
        Self {
            kind: Rc::new(RefCell::new(UITabKind::Fancy(FancyView{}))),
            surface,
            node,
        }
    }

    pub fn custom(surface: SurfaceIndex, node: NodeIndex, fn_once: Box<dyn FnMut(&mut egui::Ui)>) -> Self {
        Self {
            kind: Rc::new(RefCell::new(UITabKind::Custom(fn_once))),
            surface,
            node,
        }
    }

    pub fn world_view(surface: SurfaceIndex, node: NodeIndex, kind: Rc<RefCell<UITabKind>>) -> Self {
        Self {
            kind,
            surface,
            node,
        }
    }

    pub fn title(&self) -> String {
        match *self.kind.borrow_mut() {
            UITabKind::Regular(ref view) => view.title(&self),
            UITabKind::Fancy(ref view) => view.title(&self),
            UITabKind::WorldView(ref view) => view.title(&self),
            UITabKind::Custom(_) => format!("Custom Tab {}", self.node.0),
        }
    }

    pub fn content(&mut self, ui: &mut egui::Ui) {
        match *self.kind.borrow_mut() {
            UITabKind::Regular(ref mut view) => {
                view.content(ui);
            }
            UITabKind::Fancy(ref mut view) => {
                view.content(ui);
            }
            UITabKind::WorldView(ref mut view) => {
                view.content(ui);
            },
            UITabKind::Custom(ref mut fn_once) => {
                fn_once(ui);
            }
        };
    }
}

pub struct TabViewer<'a> {
    pub added_nodes: &'a mut Vec<UITab>,
}

impl TabViewer<'_> {
    // TODO: since the design is based arround Rc<RefCell<T>>
    // we dont need to delegate it to the tab viewer
    pub fn on_egui(&mut self, egui_renderer: &mut EguiRenderer) {
        for node in self.added_nodes.iter_mut() {
            match *node.kind.borrow_mut() {
                UITabKind::WorldView(ref mut view) => {
                    view.on_egui(egui_renderer);
                },
                _ => {},
            }
        }
    }
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
