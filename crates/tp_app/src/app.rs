//! 3Point Ana Uygulama Durum Makinesi ve Vulkan 3D DCC Editörü (`winit 0.30`).

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::sync::Arc;
use tp_core::{TaskPool, Time};
use tp_gfx::{ash, gpu_allocator, GfxContext, GpuBuffer, PbrMaterial};
use tp_math::{
    glam::{EulerRot, Mat4, Quat},
    Frustum, Transform, Vec3, Vec4,
};
use tp_mesh::{
    bvh::MeshBvh,
    create_cube, create_cylinder, create_plane, create_uv_sphere,
    export_3p, export_obj,
    history::SnapshotMeshCommand,
    mesh::HalfEdgeMesh,
    picking::pick_face,
    topology::FaceId,
    UndoStack,
};
use tp_anim::{
    AnimationClip, AnimationCurve, AnimationPlayer, BoneTrack, Interpolation, Keyframe, Skeleton,
};
use tp_scene::{
    create_character_scene, create_empty_scene, create_physics_sandbox, CameraComponent,
    LightComponent, Scene,
};
use tp_ui::{
    Editor, EditorEvent, EditorMeshInfo, EditorSceneContext, MaterialPreset, SceneObject,
    ShadingMode, TransformTool,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

/// GPU üzerinde saklanan aktif mesh tamponları.
pub struct ActiveMeshGpu {
    pub vertex_buffer: GpuBuffer,
    pub index_buffer: GpuBuffer,
    pub index_count: u32,
}

/// Sahnedeki 3D Mesh Nesnesi (Topoloji, GPU Tamponları, Transform ve Görünürlük).
pub struct SceneMeshObject {
    pub id: u32,
    pub name: String,
    pub mesh: HalfEdgeMesh,
    pub bvh: MeshBvh,
    pub gpu: Option<ActiveMeshGpu>,
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    pub material: PbrMaterial,
    pub visible: bool,
    pub selected_face: Option<FaceId>,
}

impl SceneMeshObject {
    pub fn new(id: u32, name: String, mesh: HalfEdgeMesh, position: Vec3, color: Vec4) -> Self {
        let bvh = MeshBvh::build(&mesh);
        let material = PbrMaterial::new(name.clone(), color, 0.15, 0.45);
        Self {
            id,
            name,
            mesh,
            bvh,
            gpu: None,
            position,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
            material,
            visible: true,
            selected_face: None,
        }
    }

    pub fn compute_model_matrix(&self) -> Mat4 {
        let rot_quat = Quat::from_euler(
            EulerRot::XYZ,
            self.rotation.x.to_radians(),
            self.rotation.y.to_radians(),
            self.rotation.z.to_radians(),
        );
        Mat4::from_scale_rotation_translation(self.scale, rot_quat, self.position)
    }

    pub fn sync_to_gpu(&mut self, device: &ash::Device, allocator: &mut gpu_allocator::vulkan::Allocator) {
        let (vertices, indices) = self.mesh.to_tri_mesh();
        let v_size = (vertices.len() * std::mem::size_of::<tp_mesh::MeshVertexData>()) as u64;
        let i_size = (indices.len() * std::mem::size_of::<u32>()) as u64;

        if v_size == 0 || i_size == 0 {
            return;
        }

        if let Some(mut old) = self.gpu.take() {
            old.vertex_buffer.destroy(device, allocator);
            old.index_buffer.destroy(device, allocator);
        }

        let mut vb = match GpuBuffer::new_vertex_buffer(device, allocator, v_size) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::error!("Vertex buffer oluşturulamadı: {e}");
                return;
            }
        };
        let mut ib = match GpuBuffer::new_index_buffer(device, allocator, i_size) {
            Ok(buf) => buf,
            Err(e) => {
                tracing::error!("Index buffer oluşturulamadı: {e}");
                vb.destroy(device, allocator);
                return;
            }
        };

        if let Err(e) = vb.write_data(&vertices) {
            tracing::error!("Vertex veri yazımı başarısız: {e}");
            vb.destroy(device, allocator);
            ib.destroy(device, allocator);
            return;
        }
        if let Err(e) = ib.write_data(&indices) {
            tracing::error!("Index veri yazımı başarısız: {e}");
            vb.destroy(device, allocator);
            ib.destroy(device, allocator);
            return;
        }

        self.gpu = Some(ActiveMeshGpu {
            vertex_buffer: vb,
            index_buffer: ib,
            index_count: indices.len() as u32,
        });

        self.bvh = MeshBvh::build(&self.mesh);
    }
}

/// 3Point masaüstü uygulamasının çalışma zamanı durumu.
pub struct App {
    window: Option<Arc<Window>>,
    gfx: Option<GfxContext>,
    editor: Option<Editor>,
    time: Time,
    task_pool: TaskPool,

    // Çoklu Nesne Yönetimi (Multiple Objects)
    mesh_objects: Vec<SceneMeshObject>,
    next_mesh_id: u32,
    selected_object: SceneObject,
    undo_stack: UndoStack,

    // Kamera ve Işık Nesneleri
    camera_object_pos: Vec3,
    camera_object_visible: bool,
    light_object_pos: Vec3,
    light_object_dir: Vec3,
    light_object_visible: bool,

    // Kamera Orbit & Navigasyon Durumu
    camera_target: Vec3,
    camera_distance: f32,
    camera_yaw: f32,
    camera_pitch: f32,

    // Fare ve Gizmo Etkileşim Durumları
    is_orbiting: bool,
    is_panning: bool,
    is_transforming: bool,
    transform_drag_start_pos: Vec3,
    transform_drag_start_rot: Vec3,
    transform_drag_start_scale: Vec3,
    last_mouse_pos: Option<(f64, f64)>,

    // Animasyon Sistemi (Faz 4: tp_anim)
    skeleton: Skeleton,
    anim_player: AnimationPlayer,
    bone_world_transforms: Vec<Mat4>,
    skinning_matrices: Vec<Mat4>,
    anim_duration: f32,

    // Sahne & Oyun Motoru (Faz 5: tp_scene)
    scene: Scene,

    // Performans ve Optimizasyon (Faz 6)
    low_end_mode: bool,
    is_culled: bool,
    fps: f32,
    frame_time_ms: f32,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Yeni bir uygulama durumu başlatır.
    #[must_use]
    pub fn new() -> Self {
        let task_pool = TaskPool::default();
        let default_mesh = create_cube(Vec3::splat(1.5));
        let default_obj = SceneMeshObject::new(
            0,
            "Default Cube".into(),
            default_mesh,
            Vec3::ZERO,
            Vec4::new(0.82, 0.84, 0.88, 1.0),
        );

        tracing::info!(
            "3Point App nesnesi başlatılıyor (İş parçacığı havuzu: {} çekirdek)...",
            task_pool.num_threads()
        );

        // Örnek Karakter İskeleti (Armature)
        let mut skeleton = Skeleton::new();
        let root_bone = skeleton
            .add_bone("Root", None, Transform::from_translation(Vec3::new(0.0, -0.75, 0.0)))
            .unwrap_or(tp_anim::BoneId(0));
        let spine_bone = skeleton
            .add_bone("Spine", Some(root_bone), Transform::from_translation(Vec3::new(0.0, 0.75, 0.0)))
            .unwrap_or(tp_anim::BoneId(1));
        let _head_bone = skeleton
            .add_bone("Head", Some(spine_bone), Transform::from_translation(Vec3::new(0.0, 0.75, 0.0)))
            .unwrap_or(tp_anim::BoneId(2));

        let bone_count = skeleton.bone_count();
        let bone_world_transforms = vec![Mat4::IDENTITY; bone_count];
        let skinning_matrices = vec![Mat4::IDENTITY; bone_count];

        // Örnek Animasyon Klibi (Swaying / Idle)
        let mut clip = AnimationClip::new("Idle_Sway", 3.0).unwrap_or_default();
        let mut spine_track = BoneTrack::new(spine_bone);
        let mut spine_rot = AnimationCurve::new(Interpolation::Linear);
        spine_rot.add_keyframe(Keyframe::new(0.0, Quat::from_rotation_z(-0.15)));
        spine_rot.add_keyframe(Keyframe::new(1.5, Quat::from_rotation_z(0.15)));
        spine_rot.add_keyframe(Keyframe::new(3.0, Quat::from_rotation_z(-0.15)));
        spine_track.rotation = Some(spine_rot);
        clip.add_track(spine_track);

        let mut anim_player = AnimationPlayer::new();
        anim_player.play(clip, true);

        // Sahne Grafiği ve Varsayılan Varlıklar (Faz 5: tp_scene)
        let mut scene = Scene::new();
        scene.spawn_mesh("Default Cube", Transform::IDENTITY, "Cube", Vec4::new(0.82, 0.84, 0.88, 1.0));
        scene.spawn_camera("Main Camera", Transform::from_translation(Vec3::new(0.0, 2.0, 6.0)), CameraComponent::default());
        scene.spawn_light("Directional Light", Transform::from_translation(Vec3::new(4.0, 6.0, 4.0)), LightComponent::default());

        Self {
            window: None,
            gfx: None,
            editor: None,
            time: Time::new(),
            task_pool,
            mesh_objects: vec![default_obj],
            next_mesh_id: 1,
            selected_object: SceneObject::Mesh(0),
            undo_stack: UndoStack::new(50),
            camera_object_pos: Vec3::new(0.0, 2.0, 6.0),
            camera_object_visible: true,
            light_object_pos: Vec3::new(4.0, 6.0, 4.0),
            light_object_dir: Vec3::new(-0.5, -0.8, -0.5).normalize(),
            light_object_visible: true,
            camera_target: Vec3::new(0.0, 0.5, 0.0),
            camera_distance: 6.0,
            camera_yaw: 0.785,
            camera_pitch: 0.523,
            is_orbiting: false,
            is_panning: false,
            is_transforming: false,
            transform_drag_start_pos: Vec3::ZERO,
            transform_drag_start_rot: Vec3::ZERO,
            transform_drag_start_scale: Vec3::ONE,
            last_mouse_pos: None,
            skeleton,
            anim_player,
            bone_world_transforms,
            skinning_matrices,
            anim_duration: 3.0,
            scene,
            low_end_mode: false,
            is_culled: false,
            fps: 60.0,
            frame_time_ms: 16.6,
        }
    }

    /// Arka plan görev havuzuna erişim sağlar.
    #[inline]
    #[must_use]
    pub fn task_pool(&self) -> &TaskPool {
        &self.task_pool
    }

    /// Seçili mesh nesnesinin indeksini döner.
    pub fn selected_mesh_index(&self) -> Option<usize> {
        if let SceneObject::Mesh(id) = self.selected_object {
            self.mesh_objects.iter().position(|m| m.id == id)
        } else {
            None
        }
    }

    /// Tüm nesnelerin GPU tamponlarını senkronize eder.
    pub fn sync_all_meshes_to_gpu(&mut self) {
        if let Some(gfx) = &mut self.gfx {
            let device = &gfx.device.device;
            let allocator = &mut gfx.allocator;
            for obj in &mut self.mesh_objects {
                obj.sync_to_gpu(device, allocator);
            }
        }
    }

    /// Kamera View ve Projection matrislerini hesaplar.
    pub fn compute_camera_matrices(&self, width: u32, height: u32) -> (Mat4, Mat4, Vec3) {
        let h = (height.max(1)) as f32;
        let w = (width.max(1)) as f32;
        let aspect = w / h;

        let cos_pitch = self.camera_pitch.cos();
        let sin_pitch = self.camera_pitch.sin();
        let cos_yaw = self.camera_yaw.cos();
        let sin_yaw = self.camera_yaw.sin();

        let offset = Vec3::new(
            self.camera_distance * cos_pitch * sin_yaw,
            self.camera_distance * sin_pitch,
            self.camera_distance * cos_pitch * cos_yaw,
        );
        let eye = self.camera_target + offset;

        let view = Mat4::look_at_rh(eye, self.camera_target, Vec3::Y);
        let mut proj = Mat4::perspective_rh(45.0f32.to_radians(), aspect, 0.1, 1000.0);
        proj.y_axis.y *= -1.0;

        (view, proj, eye)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("3Point Studio - 3D DCC & Vulkan Engine (v0.1.2-beta)")
                .with_inner_size(LogicalSize::new(1280.0, 720.0))
                .with_min_inner_size(LogicalSize::new(800.0, 600.0));

            let window = match event_loop.create_window(window_attributes) {
                Ok(w) => Arc::new(w),
                Err(err) => {
                    tracing::error!("Pencere oluşturulamadı: {err}");
                    event_loop.exit();
                    return;
                }
            };

            let size = window.inner_size();
            tracing::info!(
                "Pencere oluşturuldu: ID={:?}, Boyut: {}x{}, Görev Havuzu: {} iş parçacığı",
                window.id(),
                size.width,
                size.height,
                self.task_pool().num_threads()
            );

            let display_handle = match event_loop.display_handle() {
                Ok(dh) => dh.as_raw(),
                Err(err) => {
                    tracing::error!("Ekran tutucusu alınamadı: {err}");
                    event_loop.exit();
                    return;
                }
            };

            let window_handle = match window.window_handle() {
                Ok(wh) => wh.as_raw(),
                Err(err) => {
                    tracing::error!("Pencere tutucusu alınamadı: {err}");
                    event_loop.exit();
                    return;
                }
            };

            match GfxContext::new(display_handle, window_handle, size.width, size.height) {
                Ok(mut gfx) => {
                    tracing::info!("Vulkan Grafik Motoru başarıyla bağlandı!");

                    // UI Editör Katmanını Başlat
                    match Editor::new(&window, &gfx.device.device, &mut gfx.allocator, gfx.render_pass.render_pass) {
                        Ok(editor) => self.editor = Some(editor),
                        Err(err) => tracing::error!("UI Editör başlatılamadı: {err}"),
                    }

                    self.gfx = Some(gfx);
                    self.sync_all_meshes_to_gpu();
                }
                Err(err) => {
                    tracing::error!("Vulkan grafik motoru başlatılamadı: {err}");
                    event_loop.exit();
                    return;
                }
            }

            self.window = Some(window);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        // 1. Önce olayı egui editörüne ilet
        let is_consumed_by_ui = if let (Some(editor), Some(window)) = (&mut self.editor, &self.window) {
            editor.handle_event(window, &event)
        } else {
            false
        };

        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Kullanıcı pencereyi kapattı. Çıkış yapılıyor...");
                event_loop.exit();
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } if !is_consumed_by_ui => match key_code {
                KeyCode::Escape => {
                    if self.is_transforming {
                        self.is_transforming = false;
                        let p = self.transform_drag_start_pos;
                        let r = self.transform_drag_start_rot;
                        let s = self.transform_drag_start_scale;
                        if let Some(idx) = self.selected_mesh_index() {
                            let obj = &mut self.mesh_objects[idx];
                            obj.position = p;
                            obj.rotation = r;
                            obj.scale = s;
                        }
                    } else if let Some(idx) = self.selected_mesh_index() {
                        self.mesh_objects[idx].selected_face = None;
                    }
                }
                KeyCode::KeyG => {
                    // Grab / Translate
                    if let Some(editor) = &mut self.editor {
                        editor.transform_tool = TransformTool::Translate;
                    }
                    if let Some(idx) = self.selected_mesh_index() {
                        let p = self.mesh_objects[idx].position;
                        let r = self.mesh_objects[idx].rotation;
                        let s = self.mesh_objects[idx].scale;
                        self.is_transforming = true;
                        self.transform_drag_start_pos = p;
                        self.transform_drag_start_rot = r;
                        self.transform_drag_start_scale = s;
                    }
                }
                KeyCode::KeyR => {
                    // Rotate
                    if let Some(editor) = &mut self.editor {
                        editor.transform_tool = TransformTool::Rotate;
                    }
                    if let Some(idx) = self.selected_mesh_index() {
                        let p = self.mesh_objects[idx].position;
                        let r = self.mesh_objects[idx].rotation;
                        let s = self.mesh_objects[idx].scale;
                        self.is_transforming = true;
                        self.transform_drag_start_pos = p;
                        self.transform_drag_start_rot = r;
                        self.transform_drag_start_scale = s;
                    }
                }
                KeyCode::KeyS => {
                    // Scale
                    if let Some(editor) = &mut self.editor {
                        editor.transform_tool = TransformTool::Scale;
                    }
                    if let Some(idx) = self.selected_mesh_index() {
                        let p = self.mesh_objects[idx].position;
                        let r = self.mesh_objects[idx].rotation;
                        let s = self.mesh_objects[idx].scale;
                        self.is_transforming = true;
                        self.transform_drag_start_pos = p;
                        self.transform_drag_start_rot = r;
                        self.transform_drag_start_scale = s;
                    }
                }
                KeyCode::Delete | KeyCode::Backspace | KeyCode::KeyX => {
                    // Seçili nesneyi sil
                    if let SceneObject::Mesh(id) = self.selected_object {
                        if let Some(idx) = self.mesh_objects.iter().position(|m| m.id == id) {
                            let mut removed = self.mesh_objects.remove(idx);
                            if let (Some(gfx), Some(mut gpu)) = (&mut self.gfx, removed.gpu.take()) {
                                gpu.vertex_buffer.destroy(&gfx.device.device, &mut gfx.allocator);
                                gpu.index_buffer.destroy(&gfx.device.device, &mut gfx.allocator);
                            }
                            if let Some(first) = self.mesh_objects.first() {
                                self.selected_object = SceneObject::Mesh(first.id);
                                if let Some(editor) = &mut self.editor {
                                    editor.selected_object = Some(SceneObject::Mesh(first.id));
                                }
                            } else {
                                self.selected_object = SceneObject::Camera;
                                if let Some(editor) = &mut self.editor {
                                    editor.selected_object = Some(SceneObject::Camera);
                                }
                            }
                        }
                    }
                }
                KeyCode::KeyE => {
                    if let Some(idx) = self.selected_mesh_index() {
                        if let Some(fid) = self.mesh_objects[idx].selected_face {
                            let cmd = SnapshotMeshCommand::new("Extrude Face", move |m| {
                                m.extrude_face(fid, 0.8)?;
                                Ok(())
                            });
                            let ok = self.undo_stack.execute(cmd, &mut self.mesh_objects[idx].mesh).is_ok();
                            if ok {
                                let obj = &mut self.mesh_objects[idx];
                                obj.selected_face = Some(FaceId(obj.mesh.faces.len() as u32 - 1));
                                if let Some(gfx) = &mut self.gfx {
                                    obj.sync_to_gpu(&gfx.device.device, &mut gfx.allocator);
                                }
                            }
                        }
                    }
                }
                KeyCode::KeyI => {
                    if let Some(idx) = self.selected_mesh_index() {
                        if let Some(fid) = self.mesh_objects[idx].selected_face {
                            let cmd = SnapshotMeshCommand::new("Inset Face", move |m| {
                                m.inset_face(fid, 0.25)?;
                                Ok(())
                            });
                            let ok = self.undo_stack.execute(cmd, &mut self.mesh_objects[idx].mesh).is_ok();
                            if ok {
                                let obj = &mut self.mesh_objects[idx];
                                obj.selected_face = Some(FaceId(obj.mesh.faces.len() as u32 - 1));
                                if let Some(gfx) = &mut self.gfx {
                                    obj.sync_to_gpu(&gfx.device.device, &mut gfx.allocator);
                                }
                            }
                        }
                    }
                }
                KeyCode::Space => {
                    if let Some(idx) = self.selected_mesh_index() {
                        let cmd = SnapshotMeshCommand::new("Subdivide Mesh", |m| {
                            *m = m.subdivide();
                            Ok(())
                        });
                        let ok = self.undo_stack.execute(cmd, &mut self.mesh_objects[idx].mesh).is_ok();
                        if ok {
                            let obj = &mut self.mesh_objects[idx];
                            obj.selected_face = None;
                            if let Some(gfx) = &mut self.gfx {
                                obj.sync_to_gpu(&gfx.device.device, &mut gfx.allocator);
                            }
                        }
                    }
                }
                KeyCode::KeyZ => {
                    if let Some(idx) = self.selected_mesh_index() {
                        let ok = self.undo_stack.undo(&mut self.mesh_objects[idx].mesh) == Ok(true);
                        if ok {
                            let obj = &mut self.mesh_objects[idx];
                            obj.selected_face = None;
                            if let Some(gfx) = &mut self.gfx {
                                obj.sync_to_gpu(&gfx.device.device, &mut gfx.allocator);
                            }
                        }
                    }
                }
                _ => {}
            },

            WindowEvent::MouseInput { button, state, .. } if !is_consumed_by_ui => {
                let is_pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Right => self.is_orbiting = is_pressed,
                    MouseButton::Middle => self.is_panning = is_pressed,
                    MouseButton::Left => {
                        if is_pressed {
                            let tool = self.editor.as_ref().map_or(TransformTool::Select, |e| e.transform_tool);
                            if tool != TransformTool::Select {
                                // Seçili nesne üzerinde transform sürüklemeyi başlat
                                if let Some(idx) = self.selected_mesh_index() {
                                    let p = self.mesh_objects[idx].position;
                                    let r = self.mesh_objects[idx].rotation;
                                    let s = self.mesh_objects[idx].scale;
                                    self.is_transforming = true;
                                    self.transform_drag_start_pos = p;
                                    self.transform_drag_start_rot = r;
                                    self.transform_drag_start_scale = s;
                                }
                            } else {
                                // Işın atımı ile nesne ve yüzey seçimi
                                let window_clone = self.window.clone();
                                if let (Some(window), Some((mx, my))) = (window_clone, self.last_mouse_pos) {
                                    let size = window.inner_size();
                                    let (view, proj, eye) = self.compute_camera_matrices(size.width, size.height);
                                    let view_proj = proj * view;
                                    let inv_view_proj = view_proj.inverse();

                                    let ndc_x = (2.0 * mx as f32 / size.width as f32) - 1.0;
                                    let ndc_y = (2.0 * my as f32 / size.height as f32) - 1.0;

                                    let p_near = inv_view_proj * Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
                                    let p_far = inv_view_proj * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

                                    let ray_origin = eye;
                                    let ray_dir = ((p_far.truncate() / p_far.w) - (p_near.truncate() / p_near.w)).normalize_or_zero();

                                    let mut closest_mesh_id = None;
                                    let mut min_dist = f32::MAX;
                                    let mut hit_face = None;

                                    for obj in &self.mesh_objects {
                                        if !obj.visible { continue; }
                                        let inv_model = obj.compute_model_matrix().inverse();
                                        let local_origin = inv_model.transform_point3(ray_origin);
                                        let local_dir = inv_model.transform_vector3(ray_dir).normalize();
                                        let local_ray = tp_math::Ray::new(local_origin, local_dir);

                                        if let Some(hit) = pick_face(&obj.mesh, &obj.bvh, &local_ray) {
                                            if hit.distance < min_dist {
                                                min_dist = hit.distance;
                                                closest_mesh_id = Some(obj.id);
                                                hit_face = Some(hit.face_id);
                                            }
                                        }
                                    }

                                    if let Some(mesh_id) = closest_mesh_id {
                                        self.selected_object = SceneObject::Mesh(mesh_id);
                                        if let Some(editor) = &mut self.editor {
                                            editor.selected_object = Some(SceneObject::Mesh(mesh_id));
                                        }
                                        if let Some(idx) = self.mesh_objects.iter().position(|m| m.id == mesh_id) {
                                            self.mesh_objects[idx].selected_face = hit_face;
                                        }
                                    }
                                }
                            }
                        } else {
                            self.is_transforming = false;
                        }
                    }
                    _ => {}
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some((last_x, last_y)) = self.last_mouse_pos {
                    let dx = (position.x - last_x) as f32;
                    let dy = (position.y - last_y) as f32;

                    if self.is_transforming {
                        let tool = self.editor.as_ref().map_or(TransformTool::Select, |e| e.transform_tool);
                        let cos_yaw = self.camera_yaw.cos();
                        let sin_yaw = self.camera_yaw.sin();
                        let right = Vec3::new(cos_yaw, 0.0, -sin_yaw);
                        let up = Vec3::Y;
                        let factor = self.camera_distance * 0.002;

                        match self.selected_object {
                            SceneObject::Mesh(_) => {
                                if let Some(idx) = self.selected_mesh_index() {
                                    let obj = &mut self.mesh_objects[idx];
                                    match tool {
                                        TransformTool::Translate => {
                                            obj.position += (right * dx - up * dy) * factor;
                                        }
                                        TransformTool::Rotate => {
                                            obj.rotation.y += dx * 0.5;
                                            obj.rotation.x += dy * 0.5;
                                        }
                                        TransformTool::Scale => {
                                            let s = (dx - dy) * 0.01;
                                            obj.scale = (obj.scale + Vec3::splat(s)).max(Vec3::splat(0.05));
                                        }
                                        TransformTool::Select => {}
                                    }
                                }
                            }
                            SceneObject::Camera => {
                                if tool == TransformTool::Translate {
                                    self.camera_object_pos += (right * dx - up * dy) * factor;
                                }
                            }
                            SceneObject::Light => {
                                if tool == TransformTool::Translate {
                                    self.light_object_pos += (right * dx - up * dy) * factor;
                                }
                            }
                        }
                    } else if self.is_orbiting {
                        self.camera_yaw -= dx * 0.006;
                        self.camera_pitch = (self.camera_pitch + dy * 0.006).clamp(-1.5, 1.5);
                    } else if self.is_panning {
                        let cos_yaw = self.camera_yaw.cos();
                        let sin_yaw = self.camera_yaw.sin();
                        let right = Vec3::new(cos_yaw, 0.0, -sin_yaw);
                        let up = Vec3::Y;
                        let factor = self.camera_distance * 0.0015;
                        self.camera_target += (-right * dx + up * dy) * factor;
                    }
                }
                self.last_mouse_pos = Some((position.x, position.y));
            }

            WindowEvent::MouseWheel { delta, .. } if !is_consumed_by_ui => {
                let zoom_amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 0.5,
                    MouseScrollDelta::PixelDelta(p) => (p.y as f32) * 0.02,
                };
                self.camera_distance = (self.camera_distance - zoom_amount).clamp(1.0, 100.0);
            }

            WindowEvent::Resized(_physical_size) => {
                if let Some(gfx) = &mut self.gfx {
                    gfx.framebuffer_resized = true;
                }
            }

            WindowEvent::RedrawRequested => {
                self.time.update();
                let dt = self.time.delta_seconds();

                // Telemetri Güncellemesi
                if dt > 0.0 {
                    self.fps = 1.0 / dt;
                    self.frame_time_ms = dt * 1000.0;
                }

                // 1. Canlı Animasyon ve Sahne Mantığını İlerlet
                let mut local_poses = vec![Transform::IDENTITY; self.skeleton.bone_count()];
                self.anim_player.update(dt, &self.skeleton, &mut local_poses);
                let _ = self.skeleton.compute_world_transforms(&local_poses, &mut self.bone_world_transforms);
                let _ = self.skeleton.compute_skinning_matrices(
                    &self.bone_world_transforms,
                    &mut self.skinning_matrices,
                );

                // Oyun/Runtime Fizik Güncellemesi (Play Modu)
                self.scene.update(dt);

                let Some(window) = self.window.clone() else { return };
                let size = window.inner_size();
                let (view, proj, eye) = self.compute_camera_matrices(size.width, size.height);
                let view_proj = proj * view;
                let inv_view_proj = view_proj.inverse();

                // 6-Düzlemli Analitik Frustum
                let frustum = Frustum::from_view_projection(view_proj);

                // 2. UI Güncellemesi (Tessellation & Material)
                let (
                    active_name,
                    mut pos,
                    mut rot,
                    mut scale,
                    mat_name,
                    mut mat_color,
                    mut mat_metallic,
                    mut mat_roughness,
                    mut mat_emissive,
                    mut mat_ao,
                    v_cnt,
                    e_cnt,
                    f_cnt,
                    sel_face,
                ) = {
                    if let Some(idx) = self.selected_mesh_index() {
                        let m = &self.mesh_objects[idx];
                        (
                            m.name.clone(),
                            m.position,
                            m.rotation,
                            m.scale,
                            m.material.name.clone(),
                            m.material.base_color.to_array(),
                            m.material.metallic,
                            m.material.roughness,
                            m.material.emissive.to_array(),
                            m.material.ao,
                            m.mesh.vertices.len(),
                            m.mesh.edges.len(),
                            m.mesh.faces.len(),
                            m.selected_face,
                        )
                    } else {
                        (
                            "Seçim Yok".to_string(),
                            Vec3::ZERO,
                            Vec3::ZERO,
                            Vec3::ONE,
                            "Yok".to_string(),
                            [0.82, 0.84, 0.88, 1.0],
                            0.15,
                            0.45,
                            [0.0, 0.0, 0.0],
                            1.0,
                            0,
                            0,
                            0,
                            None,
                        )
                    }
                };

                let mesh_infos: Vec<EditorMeshInfo<'_>> = self
                    .mesh_objects
                    .iter_mut()
                    .map(|m| EditorMeshInfo {
                        id: m.id,
                        name: &m.name,
                        visible: &mut m.visible,
                    })
                    .collect();

                let editor_event = if let Some(editor) = &mut self.editor {
                    let mut scene_ctx = EditorSceneContext {
                        mesh_infos,
                        camera_visible: &mut self.camera_object_visible,
                        light_visible: &mut self.light_object_visible,
                        active_mesh_name: &active_name,
                        position: &mut pos,
                        rotation: &mut rot,
                        scale: &mut scale,
                        material_name: &mat_name,
                        base_color: &mut mat_color,
                        metallic: &mut mat_metallic,
                        roughness: &mut mat_roughness,
                        emissive: &mut mat_emissive,
                        ao: &mut mat_ao,
                        vertex_count: v_cnt,
                        edge_count: e_cnt,
                        face_count: f_cnt,
                        selected_face: sel_face,
                        anim_time: &mut self.anim_player.current_time,
                        anim_duration: self.anim_duration,
                        anim_speed: &mut self.anim_player.speed,
                        anim_is_playing: self.anim_player.is_playing,
                        anim_looping: &mut self.anim_player.looping,
                        fps: self.fps,
                        frame_time_ms: self.frame_time_ms,
                        is_culled: self.is_culled,
                        low_end_mode: &mut self.low_end_mode,
                    };
                    editor.update(&window, &mut scene_ctx)
                } else {
                    None
                };

                // UI'dan gelen olası transform ve materyal değişikliklerini seçili nesneye uygula
                if let Some(idx) = self.selected_mesh_index() {
                    let m = &mut self.mesh_objects[idx];
                    m.position = pos;
                    m.rotation = rot;
                    m.scale = scale;
                    m.material.base_color = Vec4::from_array(mat_color);
                    m.material.metallic = mat_metallic;
                    m.material.roughness = mat_roughness;
                    m.material.emissive = Vec3::from_array(mat_emissive);
                    m.material.ao = mat_ao;
                }

                // UI Buton Olaylarını İşle
                if let Some(event) = editor_event {
                    match event {
                        EditorEvent::ApplyMaterialPreset(preset) => {
                            if let Some(idx) = self.selected_mesh_index() {
                                let m = &mut self.mesh_objects[idx];
                                m.material = match preset {
                                    MaterialPreset::Gold => PbrMaterial::gold(),
                                    MaterialPreset::Copper => PbrMaterial::copper(),
                                    MaterialPreset::Chrome => PbrMaterial::chrome(),
                                    MaterialPreset::Plastic => PbrMaterial::plastic(m.material.base_color),
                                    MaterialPreset::Rubber => PbrMaterial::rubber(),
                                    MaterialPreset::Clay => PbrMaterial::clay(),
                                };
                                tracing::info!("Seçili nesneye PBR materyali uygulandı: {}", m.material.name);
                            }
                        }
                        EditorEvent::AddPrimitive(name) => {
                            let mesh = match name {
                                "Cube" => create_cube(Vec3::splat(1.5)),
                                "Sphere" => create_uv_sphere(1.0, 16, 16),
                                "Cylinder" => create_cylinder(0.8, 1.8, 16),
                                "Plane" => create_plane(3.0, 3.0, 4, 4),
                                _ => create_cube(Vec3::splat(1.5)),
                            };

                            let new_id = self.next_mesh_id;
                            self.next_mesh_id += 1;

                            let offset = (self.mesh_objects.len() as f32) * 2.5;
                            let pos = Vec3::new(offset, 0.0, 0.0);
                            let color = match name {
                                "Cube" => Vec4::new(0.82, 0.84, 0.88, 1.0),
                                "Sphere" => Vec4::new(0.45, 0.75, 0.95, 1.0),
                                "Cylinder" => Vec4::new(0.95, 0.65, 0.35, 1.0),
                                "Plane" => Vec4::new(0.55, 0.85, 0.55, 1.0),
                                _ => Vec4::new(0.82, 0.84, 0.88, 1.0),
                            };

                            let mut obj = SceneMeshObject::new(new_id, format!("{} {}", name, new_id), mesh, pos, color);
                            obj.material = match name {
                                "Sphere" => PbrMaterial::gold(),
                                "Cylinder" => PbrMaterial::copper(),
                                "Plane" => PbrMaterial::new("Düzlem Materyali", color, 0.0, 0.8),
                                _ => PbrMaterial::new(format!("{} Materyali", name), color, 0.2, 0.45),
                            };

                            if let Some(gfx) = &mut self.gfx {
                                obj.sync_to_gpu(&gfx.device.device, &mut gfx.allocator);
                            }

                            self.mesh_objects.push(obj);
                            self.selected_object = SceneObject::Mesh(new_id);
                            if let Some(editor) = &mut self.editor {
                                editor.selected_object = Some(SceneObject::Mesh(new_id));
                            }
                            tracing::info!("Yeni nesne eklendi: {} (Toplam nesne: {})", name, self.mesh_objects.len());
                        }
                        EditorEvent::SelectObject(obj) => {
                            self.selected_object = obj;
                            if let Some(editor) = &mut self.editor {
                                editor.selected_object = Some(obj);
                            }
                        }
                        EditorEvent::DeleteSelectedObject => {
                            if let SceneObject::Mesh(id) = self.selected_object {
                                if let Some(idx) = self.mesh_objects.iter().position(|m| m.id == id) {
                                    let mut removed = self.mesh_objects.remove(idx);
                                    if let (Some(gfx), Some(mut gpu)) = (&mut self.gfx, removed.gpu.take()) {
                                        gpu.vertex_buffer.destroy(&gfx.device.device, &mut gfx.allocator);
                                        gpu.index_buffer.destroy(&gfx.device.device, &mut gfx.allocator);
                                    }
                                    if let Some(first) = self.mesh_objects.first() {
                                        self.selected_object = SceneObject::Mesh(first.id);
                                        if let Some(editor) = &mut self.editor {
                                            editor.selected_object = Some(SceneObject::Mesh(first.id));
                                        }
                                    } else {
                                        self.selected_object = SceneObject::Camera;
                                        if let Some(editor) = &mut self.editor {
                                            editor.selected_object = Some(SceneObject::Camera);
                                        }
                                    }
                                }
                            }
                        }
                        EditorEvent::LoadEmptyScene => {
                            self.scene = create_empty_scene();
                            self.mesh_objects.clear();
                            self.next_mesh_id = 0;
                            self.selected_object = SceneObject::Camera;
                        }
                        EditorEvent::LoadCharacterScene => {
                            self.scene = create_character_scene();
                        }
                        EditorEvent::LoadPhysicsScene => {
                            self.scene = create_physics_sandbox();
                        }
                        EditorEvent::ExportObj => {
                            if let Some(idx) = self.selected_mesh_index() {
                                let obj = &self.mesh_objects[idx];
                                let out = export_obj(&obj.mesh, &obj.name);
                                tracing::info!("Model Wavefront OBJ olarak hazırlandı ({} bayt)", out.len());
                            }
                        }
                        EditorEvent::Export3P => {
                            if let Some(idx) = self.selected_mesh_index() {
                                let obj = &self.mesh_objects[idx];
                                let bin = export_3p(&obj.mesh);
                                tracing::info!("Model 3P İkili formatında hazırlandı ({} bayt)", bin.len());
                            }
                        }
                        EditorEvent::PlayGame => self.scene.start_play(),
                        EditorEvent::PauseGame => self.scene.pause_play(),
                        EditorEvent::StopGame => self.scene.stop_play(),
                        EditorEvent::PlayAnimation => self.anim_player.resume(),
                        EditorEvent::PauseAnimation => self.anim_player.pause(),
                        EditorEvent::StopAnimation => self.anim_player.stop(),
                        EditorEvent::ScrubAnimation(t) => self.anim_player.current_time = t,
                        EditorEvent::Extrude(dist) => {
                            if let Some(idx) = self.selected_mesh_index() {
                                if let Some(fid) = self.mesh_objects[idx].selected_face {
                                    let cmd = SnapshotMeshCommand::new("Extrude Face", move |m| {
                                        m.extrude_face(fid, dist)?;
                                        Ok(())
                                    });
                                    let ok = self.undo_stack.execute(cmd, &mut self.mesh_objects[idx].mesh).is_ok();
                                    if ok {
                                        let obj = &mut self.mesh_objects[idx];
                                        obj.selected_face = Some(FaceId(obj.mesh.faces.len() as u32 - 1));
                                        if let Some(gfx) = &mut self.gfx {
                                            obj.sync_to_gpu(&gfx.device.device, &mut gfx.allocator);
                                        }
                                    }
                                }
                            }
                        }
                        EditorEvent::Inset(factor) => {
                            if let Some(idx) = self.selected_mesh_index() {
                                if let Some(fid) = self.mesh_objects[idx].selected_face {
                                    let cmd = SnapshotMeshCommand::new("Inset Face", move |m| {
                                        m.inset_face(fid, factor)?;
                                        Ok(())
                                    });
                                    let ok = self.undo_stack.execute(cmd, &mut self.mesh_objects[idx].mesh).is_ok();
                                    if ok {
                                        let obj = &mut self.mesh_objects[idx];
                                        obj.selected_face = Some(FaceId(obj.mesh.faces.len() as u32 - 1));
                                        if let Some(gfx) = &mut self.gfx {
                                            obj.sync_to_gpu(&gfx.device.device, &mut gfx.allocator);
                                        }
                                    }
                                }
                            }
                        }
                        EditorEvent::Subdivide => {
                            if let Some(idx) = self.selected_mesh_index() {
                                let cmd = SnapshotMeshCommand::new("Subdivide Mesh", |m| {
                                    *m = m.subdivide();
                                    Ok(())
                                });
                                let ok = self.undo_stack.execute(cmd, &mut self.mesh_objects[idx].mesh).is_ok();
                                if ok {
                                    let obj = &mut self.mesh_objects[idx];
                                    obj.selected_face = None;
                                    if let Some(gfx) = &mut self.gfx {
                                        obj.sync_to_gpu(&gfx.device.device, &mut gfx.allocator);
                                    }
                                }
                            }
                        }
                        EditorEvent::Undo => {
                            if let Some(idx) = self.selected_mesh_index() {
                                let ok = self.undo_stack.undo(&mut self.mesh_objects[idx].mesh) == Ok(true);
                                if ok {
                                    let obj = &mut self.mesh_objects[idx];
                                    obj.selected_face = None;
                                    if let Some(gfx) = &mut self.gfx {
                                        obj.sync_to_gpu(&gfx.device.device, &mut gfx.allocator);
                                    }
                                }
                            }
                        }
                        EditorEvent::Redo => {
                            if let Some(idx) = self.selected_mesh_index() {
                                let ok = self.undo_stack.redo(&mut self.mesh_objects[idx].mesh) == Ok(true);
                                if ok {
                                    let obj = &mut self.mesh_objects[idx];
                                    obj.selected_face = None;
                                    if let Some(gfx) = &mut self.gfx {
                                        obj.sync_to_gpu(&gfx.device.device, &mut gfx.allocator);
                                    }
                                }
                            }
                        }
                    }
                }

                // 3. Vulkan Çizim Döngüsü
                if let Some(gfx) = &mut self.gfx {
                    match gfx.begin_frame(size.width, size.height) {
                        Ok(Some(frame)) => {
                            let cmd = frame.command_buffer;
                            let device = &gfx.device.device;

                            // 1. Zemin Izgarası (Infinite Grid)
                            gfx.grid.draw(device, cmd, inv_view_proj, eye, 0.1, 1000.0);

                            // 2. 3D Model Çizimleri (Tüm görünür nesneler)
                            let shading_mode = self.editor.as_ref().map_or(ShadingMode::MatCap, |e| e.shading_mode);
                            let render_mode = match shading_mode {
                                ShadingMode::Wireframe => 0,
                                ShadingMode::MatCap => 1,
                                ShadingMode::Pbr => 2,
                            };

                            for obj in &self.mesh_objects {
                                if !obj.visible {
                                    continue;
                                }

                                let model_matrix = obj.compute_model_matrix();
                                let aabb = obj.mesh.compute_aabb();
                                let rot_quat = Quat::from_euler(
                                    EulerRot::XYZ,
                                    obj.rotation.x.to_radians(),
                                    obj.rotation.y.to_radians(),
                                    obj.rotation.z.to_radians(),
                                );
                                let world_aabb = aabb.transformed(&Transform::from_trs(obj.position, rot_quat, obj.scale));

                                let is_visible = frustum.intersects_aabb(&world_aabb);
                                if !is_visible && self.low_end_mode {
                                    continue;
                                }

                                if let Some(gpu) = &obj.gpu {
                                    gfx.mesh_pipeline.draw(
                                        device,
                                        cmd,
                                        gpu.vertex_buffer.buffer,
                                        gpu.index_buffer.buffer,
                                        gpu.index_count,
                                        model_matrix,
                                        view_proj,
                                        eye,
                                        render_mode,
                                        &obj.material,
                                        self.light_object_dir,
                                    );
                                }
                            }

                            // 3. Kamera & Işık Görselleştirmeleri (Gizmos)
                            if self.camera_object_visible {
                                gfx.debug_draw.draw_camera_gizmo(
                                    self.camera_object_pos,
                                    self.camera_target,
                                    1.2,
                                    Vec4::new(0.3, 0.8, 1.0, 0.9),
                                );
                            }

                            if self.light_object_visible {
                                gfx.debug_draw.draw_light_gizmo(
                                    self.light_object_pos,
                                    self.light_object_dir,
                                    1.0,
                                    Vec4::new(1.0, 0.85, 0.2, 0.95),
                                );
                            }

                            // 4. Seçili Nesne Vurgusu ve 3B Transform Gizmosu
                            let transform_tool = self.editor.as_ref().map_or(TransformTool::Select, |e| e.transform_tool);

                            match self.selected_object {
                                SceneObject::Mesh(id) => {
                                    if let Some(obj) = self.mesh_objects.iter().find(|m| m.id == id) {
                                        let model_matrix = obj.compute_model_matrix();
                                        let aabb = obj.mesh.compute_aabb();
                                        let rot_quat = Quat::from_euler(
                                            EulerRot::XYZ,
                                            obj.rotation.x.to_radians(),
                                            obj.rotation.y.to_radians(),
                                            obj.rotation.z.to_radians(),
                                        );
                                        let world_aabb = aabb.transformed(&Transform::from_trs(obj.position, rot_quat, obj.scale));

                                        // Sınırlayıcı kutu
                                        gfx.debug_draw.draw_aabb(&world_aabb, Vec4::new(0.4, 0.7, 1.0, 0.6));

                                        // Seçili yüzey vurgusu
                                        if let Some(fid) = obj.selected_face {
                                            let v_ids = obj.mesh.face_vertices(fid);
                                            for i in 0..v_ids.len() {
                                                let p0 = model_matrix.transform_point3(obj.mesh.vertices[v_ids[i].0 as usize].position);
                                                let p1 = model_matrix.transform_point3(obj.mesh.vertices[v_ids[(i + 1) % v_ids.len()].0 as usize].position);
                                                gfx.debug_draw.draw_line(p0, p1, Vec4::new(1.0, 0.8, 0.1, 1.0));
                                            }
                                        }

                                        // 3B Transform Gizmosu
                                        match transform_tool {
                                            TransformTool::Translate => gfx.debug_draw.draw_translate_gizmo(obj.position, 1.5),
                                            TransformTool::Rotate => gfx.debug_draw.draw_rotate_gizmo(obj.position, 1.5),
                                            TransformTool::Scale => gfx.debug_draw.draw_scale_gizmo(obj.position, 1.5),
                                            TransformTool::Select => gfx.debug_draw.draw_axes(&Transform::from_translation(obj.position), 1.0),
                                        }
                                    }
                                }
                                SceneObject::Camera => {
                                    match transform_tool {
                                        TransformTool::Translate => gfx.debug_draw.draw_translate_gizmo(self.camera_object_pos, 1.5),
                                        _ => gfx.debug_draw.draw_axes(&Transform::from_translation(self.camera_object_pos), 1.0),
                                    }
                                }
                                SceneObject::Light => {
                                    match transform_tool {
                                        TransformTool::Translate => gfx.debug_draw.draw_translate_gizmo(self.light_object_pos, 1.5),
                                        _ => gfx.debug_draw.draw_axes(&Transform::from_translation(self.light_object_pos), 1.0),
                                    }
                                }
                            }

                            // İskelet Kemikleri Görselleştirmesi (Skeleton Debug Draw)
                            for i in 0..self.skeleton.bone_count() {
                                let bone = &self.skeleton.bones[i];
                                let bone_world_pos = self.bone_world_transforms[i].transform_point3(Vec3::ZERO);
                                if let Some(parent_id) = bone.parent {
                                    let parent_pos = self.bone_world_transforms[parent_id.0 as usize].transform_point3(Vec3::ZERO);
                                    gfx.debug_draw.draw_line(parent_pos, bone_world_pos, Vec4::new(1.0, 0.75, 0.2, 1.0));
                                }
                            }

                            gfx.debug_draw.flush(device, cmd, view_proj);

                            // 5. Egui UI Arayüzünü Viewport Üzerine Çiz
                            if let Some(editor) = &mut self.editor {
                                if let Err(e) = editor.render(
                                    device,
                                    &mut gfx.allocator,
                                    gfx.device.graphics_queue,
                                    gfx.commands.pool,
                                    cmd,
                                    size.width as f32,
                                    size.height as f32,
                                ) {
                                    tracing::error!("Egui UI render hatası: {e}");
                                }
                            }

                            if let Err(err) = gfx.end_frame(frame, size.width, size.height) {
                                tracing::error!("Kare sunum hatası: {err}");
                            }
                        }
                        Ok(None) => {}
                        Err(err) => {
                            tracing::error!("Kare başlatma hatası: {err}");
                        }
                    }

                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}
