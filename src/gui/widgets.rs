use nalgebra_glm as glm;

pub fn drag_vec3(ui: &mut egui::Ui, vec: &mut glm::Vec3) -> egui::InnerResponse<()> {
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut vec.x).clamp_range(-1000.0..=1000.0).speed(0.1).prefix("X: "));
        ui.add(egui::DragValue::new(&mut vec.y).clamp_range(-1000.0..=1000.0).speed(0.1).prefix("Y: "));
        ui.add(egui::DragValue::new(&mut vec.z).clamp_range(-1000.0..=1000.0).speed(0.1).prefix("Z: "));
    })
}
