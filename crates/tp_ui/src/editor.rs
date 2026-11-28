//! 3Point DCC Stüdyo ve Oyun Motoru Editör Katmanı (`Editor`).

use crate::{
    panels::{
        inspector::{render_inspector, InspectorAction, InspectorData},
        outliner::{render_outliner, OutlinerAction, OutlinerMeshEntry, SceneObject},
        timeline::{render_timeline, TimelineAction, TimelineData},
        tools::{render_toolbox, ToolAction, TransformTool},
        top_menu::{render_top_menu, MenuAction, ShadingMode, StudioMode},
    },
    theme::apply_studio_theme,
    vulkan_render::EguiVulkanRenderer,
};
use ash::{vk, Device};
use egui::{epaint::ClippedPrimitive, Color32, Context, DragValue, Grid, RichText, SidePanel, TopBottomPanel};
use egui_winit::State as EguiWinitState;
use glam::Vec3;
use gpu_allocator::vulkan::Allocator;
use tp_gfx::error::GfxError;
use tp_mesh::{topology::FaceId, SelectionMode};
use winit::{event::WindowEvent, window::Window};

/// Editör tarafından tetiklenen ve ana uygulamaya iletilen olaylar.
#[derive(Debug, Clone, PartialEq)]
pub enum EditorEvent {
    AddPrimitive(&'static str),
    Extrude(f32),
    Inset(f32),
    Subdivide,
    Undo,
    Redo,
    PlayAnimation,
    PauseAnimation,
    StopAnimation,
    ScrubAnimation(f32),
    PlayGame,
    PauseGame,
    StopGame,
    LoadEmptyScene,
    LoadCharacterScene,
    LoadPhysicsScene,
    ExportObj,
    Export3P,
    SelectObject(SceneObject),
    DeleteSelectedObject,
    ApplyMaterialPreset(crate::panels::MaterialPreset),
}

/// Sahnedeki her bir nesnenin Outliner bilgisi.
pub struct EditorMeshInfo<'a> {
    pub id: u32,
    pub name: &'a str,
    pub visible: &'a mut bool,
}

/// Editör arayüz güncellemesine sahneden aktarılan durum verileri.
pub struct EditorSceneContext<'a> {
    pub mesh_infos: Vec<EditorMeshInfo<'a>>,
    pub camera_visible: &'a mut bool,
    pub light_visible: &'a mut bool,
    pub active_mesh_name: &'a str,
    pub position: &'a mut Vec3,
    pub rotation: &'a mut Vec3,
    pub scale: &'a mut Vec3,
    pub material_name: &'a str,
    pub base_color: &'a mut [f32; 4],
    pub metallic: &'a mut f32,
    pub roughness: &'a mut f32,
    pub emissive: &'a mut [f32; 3],
    pub ao: &'a mut f32,
    pub vertex_count: usize,
    pub edge_count: usize,
    pub face_count: usize,
    pub selected_face: Option<FaceId>,
    pub anim_time: &'a mut f32,
    pub anim_duration: f32,
    pub anim_speed: &'a mut f32,
    pub anim_is_playing: bool,
    pub anim_looping: &'a mut bool,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub is_culled: bool,
    pub low_end_mode: &'a mut bool,
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
    pub textures_delta: egui::TexturesDelta,
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
            selected_object: Some(SceneObject::Mesh(0)),
            snap_grid: true,
            mesh_visible: true,
            extrude_dist: 0.8,
            inset_factor: 0.25,
            primitives: Vec::new(),
            pixels_per_point: window.scale_factor() as f32,
            textures_delta: egui::TexturesDelta::default(),
        })
    }

    /// Pencere olaylarını egui'ye iletir.
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
                    MenuAction::PlayGame => triggered_event = Some(EditorEvent::PlayGame),
                    MenuAction::PauseGame => triggered_event = Some(EditorEvent::PauseGame),
                    MenuAction::StopGame => triggered_event = Some(EditorEvent::StopGame),
                    MenuAction::NewEmptyScene => triggered_event = Some(EditorEvent::LoadEmptyScene),
                    MenuAction::NewCharacterScene => triggered_event = Some(EditorEvent::LoadCharacterScene),
                    MenuAction::NewPhysicsScene => triggered_event = Some(EditorEvent::LoadPhysicsScene),
                    MenuAction::ExportObj => triggered_event = Some(EditorEvent::ExportObj),
                    MenuAction::Export3P => triggered_event = Some(EditorEvent::Export3P),
                    _ => {}
                }
            }
        });

        // 2. Sol Araç Çubuğu (Moda Göre Dinamik)
        match self.current_mode {
            StudioMode::Modeling => {
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
            }
            StudioMode::Animation => {
                SidePanel::left("left_anim_toolbox")
                    .resizable(false)
                    .default_width(85.0)
                    .show(&self.egui_ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(4.0);
                            ui.label(RichText::new("GİZMO").small().strong());
                            ui.selectable_value(&mut self.transform_tool, TransformTool::Rotate, "🔄 Döndür");
                            ui.selectable_value(&mut self.transform_tool, TransformTool::Translate, "✥ Taşı");

                            ui.separator();
                            ui.label(RichText::new("POZ ARAÇLARI").small().strong());
                            let _ = ui.button("📌 Keyframe").on_hover_text("Mevcut zamana kemik anahtarı ekler");
                            let _ = ui.button("🎯 İki Kemik IK").on_hover_text("Bacak/kol analitik IK çözücüsü");
                            let _ = ui.button("🦴 FABRIK IK").on_hover_text("Omurga/kuyruk çok eklemli IK");
                            let _ = ui.button("🔄 Pozu Sıfırla").on_hover_text("Bind Pose durumuna döner");
                        });
                    });
            }
            StudioMode::Scene => {
                SidePanel::left("left_scene_toolbox")
                    .resizable(false)
                    .default_width(90.0)
                    .show(&self.egui_ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(4.0);
                            ui.label(RichText::new("NESNE EKLE").small().strong());
                            if ui.button("🧊 Küp").clicked() {
                                triggered_event = Some(EditorEvent::AddPrimitive("Cube"));
                            }
                            if ui.button("🌐 Küre").clicked() {
                                triggered_event = Some(EditorEvent::AddPrimitive("Sphere"));
                            }
                            if ui.button("🥫 Silindir").clicked() {
                                triggered_event = Some(EditorEvent::AddPrimitive("Cylinder"));
                            }
                            if ui.button("⏹ Düzlem").clicked() {
                                triggered_event = Some(EditorEvent::AddPrimitive("Plane"));
                            }

                            ui.separator();
                            ui.label(RichText::new("GİZMO").small().strong());
                            ui.selectable_value(&mut self.transform_tool, TransformTool::Select, "👆 İmleç");
                            ui.selectable_value(&mut self.transform_tool, TransformTool::Translate, "✥ Taşı (G)");
                            ui.selectable_value(&mut self.transform_tool, TransformTool::Rotate, "🔄 Döndür (R)");
                            ui.selectable_value(&mut self.transform_tool, TransformTool::Scale, "⤢ Ölçekle (S)");

                            ui.separator();
                            if ui.button(RichText::new("🗑️ Nesneyi Sil").color(Color32::from_rgb(248, 113, 113))).clicked() {
                                triggered_event = Some(EditorEvent::DeleteSelectedObject);
                            }
                        });
                    });
            }
            StudioMode::GameRuntime => {
                // Oyun modunda sol panel gizlenir (tam ekran oyun alanı)
            }
        }

        // 3. Sağ Panel: Outliner ve Inspector (Moda Göre Dinamik)
        SidePanel::right("right_dock_panel")
            .resizable(true)
            .default_width(260.0)
            .width_range(200.0..=400.0)
            .show(&self.egui_ctx, |ui| {
                let mut outliner_entries: Vec<OutlinerMeshEntry<'_>> = scene
                    .mesh_infos
                    .iter_mut()
                    .map(|info| OutlinerMeshEntry {
                        id: info.id,
                        name: info.name,
                        visible: info.visible,
                    })
                    .collect();

                if let Some(outliner_action) = render_outliner(
                    ui,
                    &mut self.selected_object,
                    &mut outliner_entries,
                    scene.camera_visible,
                    scene.light_visible,
                ) {
                    match outliner_action {
                        OutlinerAction::Select(obj) => triggered_event = Some(EditorEvent::SelectObject(obj)),
                        OutlinerAction::DeleteSelected => triggered_event = Some(EditorEvent::DeleteSelectedObject),
                    }
                }

                ui.separator();

                match self.current_mode {
                    StudioMode::Modeling | StudioMode::Scene => {
                        let inspector_data = InspectorData {
                            position: scene.position,
                            rotation: scene.rotation,
                            scale: scene.scale,
                            material_name: scene.material_name,
                            base_color: scene.base_color,
                            metallic: scene.metallic,
                            roughness: scene.roughness,
                            emissive: scene.emissive,
                            ao: scene.ao,
                            vertex_count: scene.vertex_count,
                            edge_count: scene.edge_count,
                            face_count: scene.face_count,
                            selected_face: scene.selected_face,
                            extrude_dist: &mut self.extrude_dist,
                            inset_factor: &mut self.inset_factor,
                            fps: scene.fps,
                            frame_time_ms: scene.frame_time_ms,
                            is_culled: scene.is_culled,
                            low_end_mode: scene.low_end_mode,
                        };

                        if let Some(action) = render_inspector(ui, inspector_data) {
                            match action {
                                InspectorAction::Extrude(d) => triggered_event = Some(EditorEvent::Extrude(d)),
                                InspectorAction::Inset(f) => triggered_event = Some(EditorEvent::Inset(f)),
                                InspectorAction::Subdivide => triggered_event = Some(EditorEvent::Subdivide),
                                InspectorAction::ApplyPreset(p) => triggered_event = Some(EditorEvent::ApplyMaterialPreset(p)),
                            }
                        }
                    }
                    StudioMode::Animation => {
                        ui.heading(RichText::new("🎬 Animasyon Özellikleri").size(14.0));
                        ui.separator();
                        ui.collapsing("🦴 İskelet Bilgisi", |ui| {
                            ui.label("• Root (Kök Kemik)");
                            ui.label("  └── Spine (Omurga)");
                            ui.label("      └── Head (Kafa)");
                        });
                        ui.collapsing("📐 Kıpırdama & Hız Ayarları", |ui| {
                            Grid::new("anim_settings_grid").num_columns(2).spacing([6.0, 4.0]).show(ui, |ui| {
                                ui.label("Klip Süresi:");
                                ui.label(format!("{:.1} sn", scene.anim_duration));
                                ui.end_row();

                                ui.label("Oynatma Hızı:");
                                ui.add(DragValue::new(scene.anim_speed).speed(0.1).range(0.1..=4.0).suffix("x"));
                                ui.end_row();

                                ui.label("Döngü (Loop):");
                                ui.checkbox(scene.anim_looping, "");
                                ui.end_row();
                            });
                        });
                    }
                    StudioMode::GameRuntime => {
                        ui.heading(RichText::new("🎮 Canlı Oyun Durumu").size(14.0));
                        ui.separator();
                        ui.collapsing("⚡ Fizik Motoru (Rapier3D)", |ui| {
                            ui.label("Yerçekimi: -9.81 m/s² (Y)");
                            ui.label("Dinamik Gövdeler: Aktif");
                            ui.label("Çarpıştırıcılar: Aktif (AABB/TriMesh)");
                        });
                        ui.collapsing("📊 Canlı Telemetri", |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Anlık FPS:");
                                ui.colored_label(Color32::from_rgb(74, 222, 128), format!("{:.0}", scene.fps));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Kare Süresi:");
                                ui.label(format!("{:.2} ms", scene.frame_time_ms));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Görünürlük:");
                                if scene.is_culled {
                                    ui.colored_label(Color32::from_rgb(248, 113, 113), "🚫 Elendi");
                                } else {
                                    ui.colored_label(Color32::from_rgb(74, 222, 128), "👁️ Görünür");
                                }
                            });
                        });
                    }
                }
            });

        // 4. Alt Panel: Yalnızca Animasyon ve Oyun modunda gösterilir
        if self.current_mode == StudioMode::Animation {
            TopBottomPanel::bottom("bottom_timeline_panel")
                .resizable(false)
                .default_height(42.0)
                .show(&self.egui_ctx, |ui| {
                    let timeline_data = TimelineData {
                        current_time: scene.anim_time,
                        duration: scene.anim_duration,
                        speed: scene.anim_speed,
                        is_playing: scene.anim_is_playing,
                        looping: scene.anim_looping,
                        fps: 30,
                    };
                    if let Some(timeline_action) = render_timeline(ui, timeline_data) {
                        match timeline_action {
                            TimelineAction::Play => triggered_event = Some(EditorEvent::PlayAnimation),
                            TimelineAction::Pause => triggered_event = Some(EditorEvent::PauseAnimation),
                            TimelineAction::Stop => triggered_event = Some(EditorEvent::StopAnimation),
                            TimelineAction::Scrub(t) => triggered_event = Some(EditorEvent::ScrubAnimation(t)),
                            TimelineAction::AddKeyframe => {}
                        }
                    }
                });
        }

        let full_output = self.egui_ctx.end_pass();
        self.pixels_per_point = full_output.pixels_per_point;
        self.egui_winit
            .handle_platform_output(window, full_output.platform_output);

        // Yeni veya güncellenen yazı tipi doku paketlerini biriktir
        self.textures_delta.append(full_output.textures_delta);

        // Çizim komutlarını derle (Tessellation)
        self.primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, self.pixels_per_point);

        triggered_event
    }

    /// Egui komutlarını Vulkan render geçişi içinde çizer.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &Device,
        allocator: &mut Allocator,
        queue: vk::Queue,
        pool: vk::CommandPool,
        cmd: vk::CommandBuffer,
        screen_width: f32,
        screen_height: f32,
    ) -> Result<(), GfxError> {
        // Bekleyen font ve arayüz dokularını GPU'ya aktar
        if !self.textures_delta.set.is_empty() {
            for (_id, delta) in &self.textures_delta.set {
                self.renderer.update_texture(device, allocator, queue, pool, delta)?;
            }
            self.textures_delta.set.clear();
        }

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
