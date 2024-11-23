mod custom_view;
mod fancy_view;
mod quick_view;
mod regular_view;
mod world_view;

pub use custom_view::CustomView;
use egui_dock::{NodeIndex, SurfaceIndex};
use enum_dispatch::enum_dispatch;
pub use fancy_view::FancyView;
pub use quick_view::QuickView;
pub use regular_view::RegularView;
pub use world_view::WorldView;

use crate::{egui_tools::EguiRenderer, gpu::GPUCtx, shared::{Shared, WeakShared}};

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
    CustomView(Shared<CustomView>),
    QuickView(Shared<QuickView>),
}

/// Doesn't cover all tabs because not all tab types
/// are buildable, quick and ucstom tabs for exampel
pub enum PendingTabRequest {
    RegularView(SurfaceIndex, NodeIndex),
    FancyView(SurfaceIndex, NodeIndex),
    WorldView(SurfaceIndex, NodeIndex),
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

pub struct HandleList<'a> {
    pub handles: &'a mut Vec<TabHandle>,
    pub pending: Vec<PendingTabRequest>,
}

impl<'a> HandleList<'a> {
    pub fn new(handles: &'a mut Vec<TabHandle>) -> Self {
        Self {
            handles,
            pending: vec![],
        }
    }

    pub fn build_tabs(
        &mut self,
        ctx: &GPUCtx,
        egui_renderer: &mut EguiRenderer,
        render_passes: &mut Vec<WeakShared<WorldView>>,
        egui_passes: &mut Vec<WeakShared<WorldView>>,
    ) {
        for item in self.pending.drain(..) {
            match item {
                PendingTabRequest::RegularView(surface, node) => {
                    let tab = RegularView::new();
                    self.handles.push(tab.as_tab_handle(surface, node));
                }
                PendingTabRequest::FancyView(surface, node) => {
                    let tab = FancyView::new();
                    self.handles.push(tab.as_tab_handle(surface, node));
                }
                PendingTabRequest::WorldView(surface, node) => {
                    let tab = WorldView::new(ctx, egui_renderer, render_passes, egui_passes);
                    self.handles.push(tab.as_tab_handle(surface, node));
                }
            }
        }
    }
}

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

        if ui.button("Regular View").clicked() {
            self.pending
                .push(PendingTabRequest::RegularView(surface, node));
        }

        if ui.button("Fancy View").clicked() {
            self.pending
                .push(PendingTabRequest::FancyView(surface, node));
        }

        if ui.button("World View").clicked() {
            self.pending
                .push(PendingTabRequest::WorldView(surface, node));
        }
    }
}
