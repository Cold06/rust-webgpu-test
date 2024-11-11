use crate::camera::Camera;
use transform_gizmo_egui::math::{DMat4, DQuat, DVec3, Transform};
use transform_gizmo_egui::{
    EnumSet, Gizmo, GizmoConfig, GizmoExt, GizmoMode, GizmoOrientation, GizmoResult,
};

pub struct GizmoExample {
    gizmo: Gizmo,
    gizmo_modes: EnumSet<GizmoMode>,
    gizmo_orientation: GizmoOrientation,
    scale: DVec3,
    rotation: DQuat,
    translation: DVec3,
}

impl GizmoExample {
    pub fn new() -> Self {
        Self {
            gizmo: Gizmo::default(),
            gizmo_modes: GizmoMode::all(),
            gizmo_orientation: GizmoOrientation::Local,
            scale: DVec3::ONE,
            rotation: DQuat::IDENTITY,
            translation: DVec3::ZERO,
        }
    }

    pub fn draw_gizmo(&mut self, ui: &mut egui::Ui, camera: &Camera) {
        // The whole clipping area of the UI is used as viewport
        let viewport = ui.clip_rect();

        let projection_matrix = DMat4::perspective_rh(
            camera.projection.fov_y as f64,
            camera.projection.aspect as f64,
            camera.projection.z_near as f64,
            camera.projection.z_far as f64,
        );

        // Fixed camera position
        let view_matrix = DMat4::look_at_rh(
            DVec3::new(
                camera.view.position.x as f64,
                camera.view.position.y as f64,
                camera.view.position.z as f64,
            ),
            DVec3::new(
                camera.view.center.x as f64,
                camera.view.center.y as f64,
                camera.view.center.z as f64,
            ),
            DVec3::Y,
        );

        // Ctrl toggles snapping
        let snapping = ui.input(|input| input.modifiers.ctrl);

        self.gizmo.update_config(GizmoConfig {
            view_matrix: view_matrix.into(),
            projection_matrix: projection_matrix.into(),
            viewport,
            modes: self.gizmo_modes,
            orientation: self.gizmo_orientation,
            snapping,
            ..Default::default()
        });

        let mut transform =
            Transform::from_scale_rotation_translation(self.scale, self.rotation, self.translation);

        if let Some((result, new_transforms)) = self.gizmo.interact(ui, &[transform]) {
            for (new_transform, transform) in
                new_transforms.iter().zip(std::iter::once(&mut transform))
            {
                *transform = *new_transform;
            }

            self.scale = transform.scale.into();
            self.rotation = transform.rotation.into();
            self.translation = transform.translation.into();

            let text = match result {
                GizmoResult::Rotation {
                    axis,
                    delta: _,
                    total,
                    is_view_axis: _,
                } => {
                    format!(
                        "Rotation axis: ({:.2}, {:.2}, {:.2}), Angle: {:.2} deg",
                        axis.x,
                        axis.y,
                        axis.z,
                        total.to_degrees()
                    )
                }
                GizmoResult::Translation { delta: _, total } => {
                    format!(
                        "Translation: ({:.2}, {:.2}, {:.2})",
                        total.x, total.y, total.z,
                    )
                }
                GizmoResult::Scale { total } => {
                    format!("Scale: ({:.2}, {:.2}, {:.2})", total.x, total.y, total.z,)
                }
                GizmoResult::Arcball { delta: _, total } => {
                    let (axis, angle) = DQuat::from(total).to_axis_angle();
                    format!(
                        "Rotation axis: ({:.2}, {:.2}, {:.2}), Angle: {:.2} deg",
                        axis.x,
                        axis.y,
                        axis.z,
                        angle.to_degrees()
                    )
                }
            };

            ui.label(text);
        }
    }

    pub fn draw_options(&mut self, ui: &mut egui::Ui) {
        ui.heading("Options");
        ui.separator();

        egui::Grid::new("options_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Modes");
                egui::ComboBox::from_id_salt("mode_cb")
                    .selected_text(format!("{}", self.gizmo_modes.len()))
                    .show_ui(ui, |ui| {
                        for mode in GizmoMode::all() {
                            let mut mode_selected = self.gizmo_modes.contains(mode);
                            ui.toggle_value(&mut mode_selected, format!("{:?}", mode));
                            if mode_selected {
                                self.gizmo_modes.insert(mode);
                            } else {
                                self.gizmo_modes.remove(mode);
                            }
                        }
                    });
                ui.end_row();

                ui.label("Orientation");
                egui::ComboBox::from_id_salt("orientation_cb")
                    .selected_text(format!("{:?}", self.gizmo_orientation))
                    .show_ui(ui, |ui| {
                        for orientation in [GizmoOrientation::Global, GizmoOrientation::Local] {
                            ui.selectable_value(
                                &mut self.gizmo_orientation,
                                orientation,
                                format!("{:?}", orientation),
                            );
                        }
                    });
                ui.end_row();
            });
    }
}
