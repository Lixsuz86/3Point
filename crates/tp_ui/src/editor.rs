//! 3Point DCC Editör Yöneticisi (`Editor`).

use crate::{
    panels::{
        render_inspector, render_outliner, render_toolbox, render_top_menu, InspectorAction,
        InspectorData, MenuAction, SceneObject, ShadingMode, StudioMode, ToolAction, TransformTool,
    },
    theme::apply_studio_theme,
    vulkan_render::EguiVulkanRenderer,
};
use ash::{vk, Device};
use egui::{ClippedPrimitive, Context, SidePanel, TopBottomPanel};
use egui_winit::State as EguiWinitState;
use glam::Vec3;
use gpu_allocator::vulkan::Allocator;
use tp_gfx::error::GfxError;
use tp_mesh::{topology::FaceId, SelectionMode};
use winit::{event::WindowEvent, window::Window};

/// Editör genelinde tetiklenen eylemler.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorEvent {
    Extrude(f32),
    Inset(f32),
    Subdivide,
    Undo,
    Redo,
    AddPrimitive(&'static str),
}

/// Editör arayüz güncellemesine sahneden aktarılan durum verileri.
pub struct EditorSceneContext<'a> {
    pub position: &'a mut Vec3,
    pub rotation: &'a mut Vec3,
    pub scale: &'a mut Vec3,
    pub vertex_count: usize,
    pub edge_count: usize,
    pub face_count: usize,
    pub selected_face: Option<FaceId>,
}

/// Tüm editör panellerini, giriş olaylarını ve egui durumunu yöneten yapı.
pub struct Editor {
    pub egui_ctx: Context,
    pub egui_winit: EguiWinitState,
    pub renderer: EguiVulkanRenderer,

    // Editör Durum Değişkenleri
    pub current_mode: StudioMode,
    pub shading_mode: ShadingMode,
    pub selection_mode: SelectionMode,
    pub transform_tool: TransformTool,
    pub selected_object: Option<SceneObject>,
    pub snap_grid: bool,
    pub mesh_visible: bool,

    // Modelleme Parametreleri
    pub extrude_dist: f32,
    pub inset_factor: f32,

    // Son karede tessellate edilmiş çizim ilkelleri
    pub primitives: Vec<ClippedPrimitive>,
    pub pixels_per_point: f32,
}

impl Editor {
    /// Yeni bir Editör örneği başlatır.
    pub fn new(
        window: &Window,
        device: &Device,
        allocator: &mut Allocator,
        render_pass: vk::RenderPass,
    ) -> Result<Self, GfxError> {
        let egui_ctx = Context::default();
        apply_studio_theme(&egui_ctx);

        let egui_winit = EguiWinitState::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            Some(2048),
        );

        let renderer = EguiVulkanRenderer::new(device, allocator, render_pass)?;

        tracing::info!("3Point UI Editör Katmanı başarıyla kuruldu.");

        Ok(Self {
            egui_ctx,
            egui_winit,
            renderer,
            current_mode: StudioMode::Modeling,
            shading_mode: ShadingMode::MatCap,
            selection_mode: SelectionMode::Face,
            transform_tool: TransformTool::Select,
            selected_object: Some(SceneObject::Mesh),
            snap_grid: true,
            mesh_visible: true,
            extrude_dist: 0.8,
            inset_factor: 0.25,
            primitives: Vec::new(),
            pixels_per_point: window.scale_factor() as f32,
        })
    }

    /// Pencere olaylarını egui'ye iletir.
    /// Eğer fare veya klavye UI üzerinde tüketildiyse `true` döner (Viewport etkileşimini engeller).
    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.egui_winit.on_window_event(window, event);
        response.consumed
    }

    /// Bir UI karesi çalıştırır ve çizim komutlarını derler.
    pub fn update(
        &mut self,
        window: &Window,
        scene: &mut EditorSceneContext<'_>,
    ) -> Option<EditorEvent> {
        let mut triggered_event = None;
        let raw_input = self.egui_winit.take_egui_input(window);

        self.egui_ctx.begin_pass(raw_input);

        // 1. Üst Menü Çubuğu
        TopBottomPanel::top("top_menu_bar").show(&self.egui_ctx, |ui| {
            if let Some(action) = render_top_menu(ui, &mut self.current_mode, &mut self.shading_mode) {
                match action {
                    MenuAction::Undo => triggered_event = Some(EditorEvent::Undo),
                    MenuAction::Redo => triggered_event = Some(EditorEvent::Redo),
                    MenuAction::AddCube => triggered_event = Some(EditorEvent::AddPrimitive("Cube")),
                    MenuAction::AddSphere => triggered_event = Some(EditorEvent::AddPrimitive("Sphere")),
                    MenuAction::AddCylinder => triggered_event = Some(EditorEvent::AddPrimitive("Cylinder")),
                    MenuAction::AddPlane => triggered_event = Some(EditorEvent::AddPrimitive("Plane")),
                    _ => {}
                }
            }
        });

        // 2. Sol Araç Çubuğu (Toolbox)
        SidePanel::left("left_toolbox")
            .resizable(false)
            .default_width(75.0)
            .show(&self.egui_ctx, |ui| {
                if let Some(tool_action) = render_toolbox(
                    ui,
                    &mut self.selection_mode,
                    &mut self.transform_tool,
                    &mut self.snap_grid,
                ) {
                    match tool_action {
                        ToolAction::Extrude => {
                            triggered_event = Some(EditorEvent::Extrude(self.extrude_dist));
                        }
                        ToolAction::Inset => {
                            triggered_event = Some(EditorEvent::Inset(self.inset_factor));
                        }
                        ToolAction::Subdivide => {
                            triggered_event = Some(EditorEvent::Subdivide);
                        }
                    }
                }
            });

        // 3. Sağ Panel: Üstte Outliner, Altta Inspector
        SidePanel::right("right_dock_panel")
            .resizable(true)
            .default_width(260.0)
            .width_range(200.0..=400.0)
            .show(&self.egui_ctx, |ui| {
                render_outliner(
                    ui,
                    &mut self.selected_object,
                    "Default Cube",
                    &mut self.mesh_visible,
                );

                ui.separator();

                let inspector_data = InspectorData {
                    position: scene.position,
                    rotation: scene.rotation,
                    scale: scene.scale,
                    vertex_count: scene.vertex_count,
                    edge_count: scene.edge_count,
                    face_count: scene.face_count,
                    selected_face: scene.selected_face,
                    extrude_dist: &mut self.extrude_dist,
                    inset_factor: &mut self.inset_factor,
                };

                if let Some(action) = render_inspector(ui, inspector_data) {
                    match action {
                        InspectorAction::Extrude(d) => triggered_event = Some(EditorEvent::Extrude(d)),
                        InspectorAction::Inset(f) => triggered_event = Some(EditorEvent::Inset(f)),
                        InspectorAction::Subdivide => triggered_event = Some(EditorEvent::Subdivide),
                    }
                }
            });

        let full_output = self.egui_ctx.end_pass();
        self.pixels_per_point = full_output.pixels_per_point;
        self.egui_winit
            .handle_platform_output(window, full_output.platform_output);

        // Çizim komutlarını derle (Tessellation)
        self.primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, self.pixels_per_point);

        triggered_event
    }

    /// Egui komutlarını Vulkan render geçişi içinde çizer.
    pub fn render(
        &mut self,
        device: &Device,
        cmd: vk::CommandBuffer,
        screen_width: f32,
        screen_height: f32,
    ) -> Result<(), GfxError> {
        self.renderer.render(
            device,
            cmd,
            screen_width,
            screen_height,
            self.pixels_per_point,
            &self.primitives,
        )
    }

    /// Kaynakları serbest bırakır.
    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        self.renderer.destroy(device, allocator);
    }
}
