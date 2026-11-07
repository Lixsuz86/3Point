//! 3Point Özellikler ve Parametreler Paneli (Inspector).

use egui::{Color32, DragValue, Grid, RichText, Ui};
use glam::Vec3;
use tp_mesh::topology::FaceId;

/// Inspector paneli için veri girdi yapısı.
pub struct InspectorData<'a> {
    pub position: &'a mut Vec3,
    pub rotation: &'a mut Vec3,
    pub scale: &'a mut Vec3,
    pub vertex_count: usize,
    pub edge_count: usize,
    pub face_count: usize,
    pub selected_face: Option<FaceId>,
    pub extrude_dist: &'a mut f32,
    pub inset_factor: &'a mut f32,
}

/// Inspector paneli eylemleri.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InspectorAction {
    Extrude(f32),
    Inset(f32),
    Subdivide,
}

/// Inspector panelini render eder.
pub fn render_inspector(ui: &mut Ui, data: InspectorData<'_>) -> Option<InspectorAction> {
    let mut action = None;

    ui.heading(RichText::new("Özellikler (Inspector)").size(14.0));
    ui.separator();

    // 1. Transform Bileşeni
    ui.collapsing("📐 Transform", |ui| {
        Grid::new("transform_grid").num_columns(4).spacing([6.0, 4.0]).show(ui, |ui| {
            // Konum (Translation)
            ui.label(RichText::new("Konum:").strong());
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(235, 80, 80), "X");
                ui.add(DragValue::new(&mut data.position.x).speed(0.05));
            });
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(80, 220, 80), "Y");
                ui.add(DragValue::new(&mut data.position.y).speed(0.05));
            });
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(80, 140, 240), "Z");
                ui.add(DragValue::new(&mut data.position.z).speed(0.05));
            });
            ui.end_row();

            // Döndürme (Rotation)
            ui.label(RichText::new("Döndürme:").strong());
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(235, 80, 80), "X");
                ui.add(DragValue::new(&mut data.rotation.x).speed(0.5).suffix("°"));
            });
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(80, 220, 80), "Y");
                ui.add(DragValue::new(&mut data.rotation.y).speed(0.5).suffix("°"));
            });
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(80, 140, 240), "Z");
                ui.add(DragValue::new(&mut data.rotation.z).speed(0.5).suffix("°"));
            });
            ui.end_row();

            // Ölçek (Scale)
            ui.label(RichText::new("Ölçek:").strong());
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(235, 80, 80), "X");
                ui.add(DragValue::new(&mut data.scale.x).speed(0.02));
            });
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(80, 220, 80), "Y");
                ui.add(DragValue::new(&mut data.scale.y).speed(0.02));
            });
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(80, 140, 240), "Z");
                ui.add(DragValue::new(&mut data.scale.z).speed(0.02));
            });
            ui.end_row();
        });
    });

    ui.separator();

    // 2. Mesh Topoloji Bilgisi
    ui.collapsing("📊 Mesh İstatistikleri", |ui| {
        ui.horizontal(|ui| {
            ui.label("Köşeler (Vertices):");
            ui.label(RichText::new(data.vertex_count.to_string()).strong());
        });
        ui.horizontal(|ui| {
            ui.label("Kenarlar (Edges):");
            ui.label(RichText::new(data.edge_count.to_string()).strong());
        });
        ui.horizontal(|ui| {
            ui.label("Yüzeyler (Faces):");
            ui.label(RichText::new(data.face_count.to_string()).strong());
        });

        if let Some(fid) = data.selected_face {
            ui.colored_label(Color32::from_rgb(255, 195, 0), format!("Seçili Yüzey: #{:?}", fid.0));
        } else {
            ui.colored_label(Color32::GRAY, "Seçili yüzey yok (Seçmek için tıkla)");
        }
    });

    ui.separator();

    // 3. Modelleme Operatörleri ve Parametreleri
    ui.collapsing("🛠️ Modelleme Operatörleri", |ui| {
        ui.add_enabled_ui(data.selected_face.is_some(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Mesafe:");
                ui.add(DragValue::new(data.extrude_dist).speed(0.05).range(0.05..=10.0));
                if ui.button("⇱ Extrude").clicked() {
                    action = Some(InspectorAction::Extrude(*data.extrude_dist));
                }
            });

            ui.horizontal(|ui| {
                ui.label("Kalınlık:");
                ui.add(DragValue::new(data.inset_factor).speed(0.01).range(0.02..=0.95));
                if ui.button("⧉ Inset").clicked() {
                    action = Some(InspectorAction::Inset(*data.inset_factor));
                }
            });
        });

        ui.add_space(4.0);
        if ui.button("❖ Subdivide (Tüm Mesh)").clicked() {
            action = Some(InspectorAction::Subdivide);
        }
    });

    action
}
