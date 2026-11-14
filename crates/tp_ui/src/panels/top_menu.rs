//! 3Point Üst Menü Çubuğu ve Mod Seçici.

use egui::{menu, Color32, Layout, RichText, Ui};

/// Çalışma modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StudioMode {
    #[default]
    Modeling,
    Sculpt,
    Animation,
    GameRuntime,
}

/// Shading / Görselleştirme modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadingMode {
    Wireframe,
    #[default]
    MatCap,
    Pbr,
}

/// Menü çubuğu kullanıcı eylemleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    NewScene,
    AddCube,
    AddSphere,
    AddCylinder,
    AddPlane,
    Undo,
    Redo,
    ToggleGrid,
}

/// Üst menü çubuğunu render eder.
pub fn render_top_menu(
    ui: &mut Ui,
    current_mode: &mut StudioMode,
    shading_mode: &mut ShadingMode,
) -> Option<MenuAction> {
    let mut triggered_action = None;

    menu::bar(ui, |ui| {
        ui.label(RichText::new("3POINT").strong().color(Color32::from_rgb(56, 139, 253)));
        ui.separator();

        menu::menu_button(ui, "Dosya", |ui| {
            if ui.button("Yeni Sahne (Ctrl+N)").clicked() {
                triggered_action = Some(MenuAction::NewScene);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Dışa Aktar (glTF 2.0)").clicked() {
                ui.close_menu();
            }
        });

        menu::menu_button(ui, "Düzenle", |ui| {
            if ui.button("Geri Al (Z)").clicked() {
                triggered_action = Some(MenuAction::Undo);
                ui.close_menu();
            }
            if ui.button("Yinele (Ctrl+Y)").clicked() {
                triggered_action = Some(MenuAction::Redo);
                ui.close_menu();
            }
        });

        menu::menu_button(ui, "Ekle", |ui| {
            if ui.button("🧊 Küp (Cube)").clicked() {
                triggered_action = Some(MenuAction::AddCube);
                ui.close_menu();
            }
            if ui.button("🌐 Küre (UV Sphere)").clicked() {
                triggered_action = Some(MenuAction::AddSphere);
                ui.close_menu();
            }
            if ui.button("🛢️ Silindir (Cylinder)").clicked() {
                triggered_action = Some(MenuAction::AddCylinder);
                ui.close_menu();
            }
            if ui.button("⬛ Düzlem (Plane)").clicked() {
                triggered_action = Some(MenuAction::AddPlane);
                ui.close_menu();
            }
        });

        menu::menu_button(ui, "Görünüm", |ui| {
            if ui.checkbox(&mut true, "Zemin Izgarası (Grid)").clicked() {
                triggered_action = Some(MenuAction::ToggleGrid);
            }
        });

        ui.separator();

        // Stüdyo Mod Seçici (Merkezde butonlar)
        ui.selectable_value(current_mode, StudioMode::Modeling, "📐 Modelleme");
        ui.selectable_value(current_mode, StudioMode::Sculpt, "🎨 Heykeltıraş");
        ui.selectable_value(current_mode, StudioMode::Animation, "🎬 Animasyon");
        ui.selectable_value(current_mode, StudioMode::GameRuntime, "🎮 Oyun Testi");

        // Sağ tarafta Görselleştirme Modları (Shading)
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.selectable_value(shading_mode, ShadingMode::Pbr, "PBR");
            ui.selectable_value(shading_mode, ShadingMode::MatCap, "MatCap");
            ui.selectable_value(shading_mode, ShadingMode::Wireframe, "Tel Kafes");
            ui.label(RichText::new("Aydınlatma:").color(Color32::GRAY));
        });
    });

    triggered_action
}

