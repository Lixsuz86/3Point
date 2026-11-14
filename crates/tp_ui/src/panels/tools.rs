//! 3Point Sol Araç Çubuğu (Toolbox).

use egui::{RichText, Ui};
use tp_mesh::SelectionMode;

/// Transform manipülatör modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformTool {
    #[default]
    Select,
    Translate,
    Rotate,
    Scale,
}

/// Araç çubuğu eylemleri.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolAction {
    Extrude,
    Inset,
    Subdivide,
}

/// Sol araç çubuğunu çizer.
pub fn render_toolbox(
    ui: &mut Ui,
    selection_mode: &mut SelectionMode,
    transform_tool: &mut TransformTool,
    snap_grid: &mut bool,
) -> Option<ToolAction> {
    let mut triggered_action = None;

    ui.vertical_centered(|ui| {
        ui.add_space(4.0);
        ui.label(RichText::new("SEÇİM").small().strong());
        ui.selectable_value(selection_mode, SelectionMode::Vertex, "🔴 Köşe");
        ui.selectable_value(selection_mode, SelectionMode::Edge, "➖ Kenar");
        ui.selectable_value(selection_mode, SelectionMode::Face, "⬛ Yüzey");

        ui.separator();

        ui.label(RichText::new("GİZMO").small().strong());
        ui.selectable_value(transform_tool, TransformTool::Select, "👆 İmleç");
        ui.selectable_value(transform_tool, TransformTool::Translate, "✥ Taşı (G)");
        ui.selectable_value(transform_tool, TransformTool::Rotate, "🔄 Döndür (R)");
        ui.selectable_value(transform_tool, TransformTool::Scale, "⤢ Ölçekle (S)");

        ui.separator();

        ui.label(RichText::new("ARAÇLAR").small().strong());
        if ui.button("⇱ Extrude (E)").on_hover_text("Seçili yüzeyi normali boyunca uzatır").clicked() {
            triggered_action = Some(ToolAction::Extrude);
        }
        if ui.button("⧉ Inset (I)").on_hover_text("Seçili yüzeyi içe doğru daraltır").clicked() {
            triggered_action = Some(ToolAction::Inset);
        }
        if ui.button("❖ Subdivide").on_hover_text("Modeli dörtgenlere bölerek yumuşatır").clicked() {
            triggered_action = Some(ToolAction::Subdivide);
        }

        ui.separator();
        ui.checkbox(snap_grid, "Izgara Snap");
    });

    triggered_action
}

