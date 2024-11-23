use crate::frontend::{TabHandle, TabView};
use crate::shared::Shared;
use egui::Ui;

pub struct CustomView {
    custom: Box<dyn FnMut(&mut Ui)>,
}

impl CustomView {
    #[allow(unused)]
    pub fn new<T: FnMut(&mut Ui) + 'static>(custom: T) -> Shared<Self> {
        Self {
            custom: Box::new(custom),
        }
        .into()
    }
}

impl TabView for Shared<CustomView> {
    fn title(&self, tab: &TabHandle) -> String {
        format!("Custom Tab {}", tab.node.0)
    }

    fn content(&mut self, ui: &mut Ui) {
        let mut the_fn = self.borrow_mut();

        let also_the_fn = the_fn.custom.as_mut();

        also_the_fn(ui);
    }

    fn as_tab_handle(
        &self,
        surface: egui_dock::SurfaceIndex,
        node: egui_dock::NodeIndex,
    ) -> TabHandle {
        TabHandle::new(self.clone().into(), surface, node)
    }
}
