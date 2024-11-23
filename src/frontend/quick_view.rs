use crate::frontend::{TabHandle, TabView};
use crate::shared::Shared;
use egui::Ui;

pub struct QuickView {
    custom: Option<Box<dyn FnOnce(&mut Ui)>>,
}

impl QuickView {
    pub fn new() -> Shared<Self> {
        Self { custom: None }.into()
    }
}

impl Shared<QuickView> {
    pub fn ui<T: FnOnce(&mut Ui) + 'static>(&mut self, new_fn: T) {
        self.with(|this| {
            this.custom = Some(Box::new(new_fn));
        });
    }
}

impl TabView for Shared<QuickView> {
    fn title(&self, tab: &TabHandle) -> String {
        format!("Quick Tab {}", tab.node.0)
    }

    fn content(&mut self, ui: &mut Ui) {
        self.with(|this| {
            if let Some(fn_to_run) = this.custom.take() {
                fn_to_run(ui);
            }
        });
    }

    fn as_tab_handle(
        &self,
        surface: egui_dock::SurfaceIndex,
        node: egui_dock::NodeIndex,
    ) -> TabHandle {
        TabHandle::new(self.clone().into(), surface, node)
    }
}
