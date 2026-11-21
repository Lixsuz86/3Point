//! 3Point Sahne Hiyerarşisi (Outliner).

use egui::{CollapsingHeader, Color32, RichText, Ui};

/// Sahnedeki seçili nesne kimliği.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneObject {
    Mesh(u32),
    Camera,
    Light,
}

/// Outliner içindeki her bir mesh girdisi.
pub struct OutlinerMeshEntry<'a> {
    pub id: u32,
    pub name: &'a str,
    pub visible: &'a mut bool,
}

/// Outliner eylemleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlinerAction {
    Select(SceneObject),
    DeleteSelected,
}

/// Outliner panelini çizer.
pub fn render_outliner(
    ui: &mut Ui,
    selected_object: &mut Option<SceneObject>,
    mesh_entries: &mut [OutlinerMeshEntry<'_>],
    camera_visible: &mut bool,
    light_visible: &mut bool,
) -> Option<OutlinerAction> {
    let mut action = None;

    ui.heading(RichText::new("Sahne Hiyerarşisi").size(14.0));
    ui.separator();

    CollapsingHeader::new("🌐 Sahne Koleksiyonu")
        .default_open(true)
        .show(ui, |ui| {
            // Tüm Mesh Nesneleri
            for entry in mesh_entries.iter_mut() {
                ui.horizontal(|ui| {
                    let vis_icon = if *entry.visible { "👁" } else { "🚫" };
                    if ui.small_button(vis_icon).clicked() {
                        *entry.visible = !*entry.visible;
                    }

                    let is_selected = *selected_object == Some(SceneObject::Mesh(entry.id));
                    let label = format!("🧊 {}", entry.name);
                    if ui.selectable_label(is_selected, label).clicked() {
                        *selected_object = Some(SceneObject::Mesh(entry.id));
                        action = Some(OutlinerAction::Select(SceneObject::Mesh(entry.id)));
                    }
                });
            }

            ui.separator();

            // Kamera Öğesi
            ui.horizontal(|ui| {
                let vis_icon = if *camera_visible { "👁" } else { "🚫" };
                if ui.small_button(vis_icon).clicked() {
                    *camera_visible = !*camera_visible;
                }
                let is_selected = *selected_object == Some(SceneObject::Camera);
                if ui.selectable_label(is_selected, "🎥 Perspektif Kamera").clicked() {
                    *selected_object = Some(SceneObject::Camera);
                    action = Some(OutlinerAction::Select(SceneObject::Camera));
                }
            });

            // Işık Öğesi
            ui.horizontal(|ui| {
                let vis_icon = if *light_visible { "👁" } else { "🚫" };
                if ui.small_button(vis_icon).clicked() {
                    *light_visible = !*light_visible;
                }
                let is_selected = *selected_object == Some(SceneObject::Light);
                if ui.selectable_label(is_selected, "☀️ Güneş Işığı (Sun)").clicked() {
                    *selected_object = Some(SceneObject::Light);
                    action = Some(OutlinerAction::Select(SceneObject::Light));
                }
            });
        });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button(RichText::new("🗑️ Seçileni Sil").small().color(Color32::from_rgb(248, 113, 113))).clicked() {
            action = Some(OutlinerAction::DeleteSelected);
        }
    });

    ui.colored_label(Color32::from_rgb(120, 125, 135), "İpucu: Nesneye tıklayarak seçebilirsiniz.");

    action
}
