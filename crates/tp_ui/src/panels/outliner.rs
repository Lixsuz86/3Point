//! 3Point Sahne Hiyerarşisi (Outliner).

use egui::{CollapsingHeader, Color32, RichText, Ui};

/// Sahnedeki nesne kimliği.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneObject {
    Mesh,
    Camera,
    Light,
}

/// Outliner panelini çizer.
pub fn render_outliner(
    ui: &mut Ui,
    selected_object: &mut Option<SceneObject>,
    mesh_name: &str,
    mesh_visible: &mut bool,
) {
    ui.heading(RichText::new("Sahne Hiyerarşisi").size(14.0));
    ui.separator();

    CollapsingHeader::new("🌐 Sahne Koleksiyonu")
        .default_open(true)
        .show(ui, |ui| {
            // Mesh Öğesi
            ui.horizontal(|ui| {
                let vis_icon = if *mesh_visible { "👁" } else { "🚫" };
                if ui.small_button(vis_icon).clicked() {
                    *mesh_visible = !*mesh_visible;
                }

                let is_selected = *selected_object == Some(SceneObject::Mesh);
                if ui.selectable_label(is_selected, format!("🧊 {mesh_name}")).clicked() {
                    *selected_object = Some(SceneObject::Mesh);
                }
            });

            // Kamera Öğesi
            ui.horizontal(|ui| {
                let _ = ui.small_button("👁");
                let is_selected = *selected_object == Some(SceneObject::Camera);
                if ui.selectable_label(is_selected, "🎥 Perspektif Kamera").clicked() {
                    *selected_object = Some(SceneObject::Camera);
                }
            });

            // Işık Öğesi
            ui.horizontal(|ui| {
                let _ = ui.small_button("👁");
                let is_selected = *selected_object == Some(SceneObject::Light);
                if ui.selectable_label(is_selected, "☀️ Güneş Işığı (Sun)").clicked() {
                    *selected_object = Some(SceneObject::Light);
                }
            });
        });

    ui.add_space(8.0);
    ui.colored_label(Color32::from_rgb(120, 125, 135), "İpucu: Nesneye tıklayarak seçebilirsiniz.");
}
