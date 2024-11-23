use crate::frontend::{TabHandle, TabView};
use crate::shared::Shared;
use egui::Ui;

pub struct CodeView {
    language: String,
    code: Shared<String>,
    show_theme: bool,
}

impl CodeView {
    pub fn new() -> Shared<Self> {
        Self {
            language: String::from("js"),
            code: String::from(include_str!("../../js/outline.js")).into(),
            show_theme: false,
        }
        .into()
    }

    #[allow(unused)]
    fn get_code(&self) -> Shared<String> {
        self.code.clone()
    }
}

impl TabView for Shared<CodeView> {
    fn title(&self, tab: &TabHandle) -> String {
        format!("Code Editor {}", tab.node.0)
    }

    fn content(&mut self, ui: &mut Ui) {
        self.with(|this| {
            let mut theme =
                egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());

            if this.show_theme {
                ui.collapsing("Theme", |ui| {
                    ui.group(|ui| {
                        theme.ui(ui);
                        theme.clone().store_in_memory(ui.ctx());
                    });
                });
            }

            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                let mut layout_job = egui_extras::syntax_highlighting::highlight(
                    ui.ctx(),
                    ui.style(),
                    &theme,
                    string,
                    &mut this.language,
                );
                layout_job.wrap.max_width = wrap_width;
                ui.fonts(|f| f.layout_job(layout_job))
            };

            // if ui.button("Eval").clicked() {
            //     {
            //         inner.with(|u| u.clear());
            //     }

            //     inner_c.with(|code| {
            //         inner_vm.with(|u| {
            //             u.eval_with_canvas(code.to_string(), inner.clone())
            //         });
            //     });

            //     let new_texture_sync = inner.with(|u| u.as_bytes().unwrap());

            //     inner_text.with(|u| {
            //         u.update(&inner_ctx, &new_texture_sync);
            //     });
            // }

            this.code.with(|code| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(code)
                            .font(egui::TextStyle::Monospace) // for cursor height
                            .code_editor()
                            .desired_rows(10)
                            .lock_focus(true)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut layouter),
                    );
                });
            });
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
