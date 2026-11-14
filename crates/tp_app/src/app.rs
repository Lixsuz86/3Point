//! 3Point Ana Uygulama Durum Makinesi ve Vulkan 3D DCC Editörü (`winit 0.30`).

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::sync::Arc;
use tp_core::{TaskPool, Time};
use tp_gfx::{GfxContext, GpuBuffer};
use tp_math::{
    glam::{EulerRot, Mat4, Quat},
    Transform, Vec3, Vec4,
};
use tp_mesh::{
    bvh::MeshBvh,
    create_cube, create_cylinder, create_plane, create_uv_sphere,
    history::SnapshotMeshCommand,
    mesh::HalfEdgeMesh,
    picking::pick_face,
    topology::FaceId,
    UndoStack,
};
use tp_anim::{
    AnimationClip, AnimationCurve, AnimationPlayer, BoneTrack, Interpolation, Keyframe, Skeleton,
};
use tp_scene::{CameraComponent, LightComponent, Scene};
use tp_ui::{Editor, EditorEvent, EditorSceneContext, ShadingMode};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

/// GPU üzerinde saklanan aktif mesh tamponları.
struct ActiveMeshGpu {
    vertex_buffer: GpuBuffer,
    index_buffer: GpuBuffer,
    index_count: u32,
}

/// 3Point masaüstü uygulamasının çalışma zamanı durumu.
pub struct App {
    window: Option<Arc<Window>>,
    gfx: Option<GfxContext>,
    editor: Option<Editor>,
    time: Time,
    task_pool: TaskPool,

    // 3D Modelleme Verisi ve Geri Alma Yığını
    active_mesh: HalfEdgeMesh,
    mesh_bvh: MeshBvh,
    mesh_gpu: Option<ActiveMeshGpu>,
    undo_stack: UndoStack,
    selected_face: Option<FaceId>,

    // Mesh Nesne Dönüşüm Bilgileri (Transform)
    mesh_position: Vec3,
    mesh_rotation: Vec3,
    mesh_scale: Vec3,

    // Kamera Orbit & Navigasyon Durumu
    camera_target: Vec3,
    camera_distance: f32,
    camera_yaw: f32,
    camera_pitch: f32,

    // Fare Etkileşim Durumları
    is_orbiting: bool,
    is_panning: bool,
    last_mouse_pos: Option<(f64, f64)>,

    // Animasyon Sistemi (Faz 4: tp_anim)
    skeleton: Skeleton,
    anim_player: AnimationPlayer,
    bone_world_transforms: Vec<Mat4>,
    skinning_matrices: Vec<Mat4>,
    anim_duration: f32,

    // Sahne & Oyun Motoru (Faz 5: tp_scene)
    scene: Scene,
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
        let mesh = create_cube(Vec3::splat(1.5));
        let bvh = MeshBvh::build(&mesh);

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
        let mut clip = AnimationClip::new("Idle_Sway", 3.0).unwrap();
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
        scene.spawn_light("Directional Light", Transform::from_translation(Vec3::new(5.0, 10.0, 5.0)), LightComponent::default());

        Self {
            window: None,
            gfx: None,
            editor: None,
            time: Time::new(),
            task_pool,
            active_mesh: mesh,
            mesh_bvh: bvh,
            mesh_gpu: None,
            undo_stack: UndoStack::new(50),
            selected_face: Some(FaceId(3)),
            mesh_position: Vec3::ZERO,
            mesh_rotation: Vec3::ZERO,
            mesh_scale: Vec3::ONE,
            camera_target: Vec3::new(0.0, 0.5, 0.0),
            camera_distance: 6.0,
            camera_yaw: 0.785,
            camera_pitch: 0.523,
            is_orbiting: false,
            is_panning: false,
            last_mouse_pos: None,
            skeleton,
            anim_player,
            bone_world_transforms,
            skinning_matrices,
            anim_duration: 3.0,
            scene,
        }
    }

    /// Arka plan görev havuzuna erişim sağlar.
    #[inline]
    #[must_use]
    pub fn task_pool(&self) -> &TaskPool {
        &self.task_pool
    }

    /// Aktif mesh'in GPU tamponlarını yeniler.
    pub fn sync_mesh_to_gpu(&mut self) {
        let Some(gfx) = &mut self.gfx else { return };
        let (vertices, indices) = self.active_mesh.to_tri_mesh();
        let v_size = (vertices.len() * std::mem::size_of::<tp_mesh::MeshVertexData>()) as u64;
        let i_size = (indices.len() * std::mem::size_of::<u32>()) as u64;

        if v_size == 0 || i_size == 0 {
            return;
        }

        // Eski tamponları temizle
        if let Some(mut old) = self.mesh_gpu.take() {
            old.vertex_buffer.destroy(&gfx.device.device, &mut gfx.allocator);
            old.index_buffer.destroy(&gfx.device.device, &mut gfx.allocator);
        }

        let mut vb = GpuBuffer::new_vertex_buffer(&gfx.device.device, &mut gfx.allocator, v_size)
            .expect("Vertex buffer oluşturulamadı");
        let mut ib = GpuBuffer::new_index_buffer(&gfx.device.device, &mut gfx.allocator, i_size)
            .expect("Index buffer oluşturulamadı");

        vb.write_data(&vertices).expect("Vertex veri yazımı başarısız");
        ib.write_data(&indices).expect("Index veri yazımı başarısız");

        self.mesh_gpu = Some(ActiveMeshGpu {
            vertex_buffer: vb,
            index_buffer: ib,
            index_count: indices.len() as u32,
        });

        self.mesh_bvh = MeshBvh::build(&self.active_mesh);
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

    /// Mesh dönüşüm matrisini oluşturur.
    pub fn compute_model_matrix(&self) -> Mat4 {
        let rot_quat = Quat::from_euler(
            EulerRot::XYZ,
            self.mesh_rotation.x.to_radians(),
            self.mesh_rotation.y.to_radians(),
            self.mesh_rotation.z.to_radians(),
        );
        Mat4::from_scale_rotation_translation(self.mesh_scale, rot_quat, self.mesh_position)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("3Point Studio - 3D DCC & Vulkan Engine (v0.1.0-beta)")
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
                    self.sync_mesh_to_gpu();
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
                    tracing::info!("ESC basıldı, çıkış yapılıyor.");
                    event_loop.exit();
                }
                KeyCode::KeyE => {
                    if let Some(fid) = self.selected_face {
                        let cmd = SnapshotMeshCommand::new("Extrude Face", move |m| {
                            m.extrude_face(fid, 0.8)?;
                            Ok(())
                        });
                        if self.undo_stack.execute(cmd, &mut self.active_mesh).is_ok() {
                            self.selected_face = Some(FaceId(self.active_mesh.faces.len() as u32 - 1));
                            self.sync_mesh_to_gpu();
                        }
                    }
                }
                KeyCode::KeyI => {
                    if let Some(fid) = self.selected_face {
                        let cmd = SnapshotMeshCommand::new("Inset Face", move |m| {
                            m.inset_face(fid, 0.25)?;
                            Ok(())
                        });
                        if self.undo_stack.execute(cmd, &mut self.active_mesh).is_ok() {
                            self.selected_face = Some(FaceId(self.active_mesh.faces.len() as u32 - 1));
                            self.sync_mesh_to_gpu();
                        }
                    }
                }
                KeyCode::Space => {
                    let cmd = SnapshotMeshCommand::new("Subdivide Mesh", |m| {
                        *m = m.subdivide();
                        Ok(())
                    });
                    if self.undo_stack.execute(cmd, &mut self.active_mesh).is_ok() {
                        self.selected_face = None;
                        self.sync_mesh_to_gpu();
                    }
                }
                KeyCode::KeyZ => {
                    if let Ok(true) = self.undo_stack.undo(&mut self.active_mesh) {
                        self.selected_face = None;
                        self.sync_mesh_to_gpu();
                    }
                }
                _ => {}
            },

            WindowEvent::MouseInput { button, state, .. } if !is_consumed_by_ui => {
                let is_pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Right => self.is_orbiting = is_pressed,
                    MouseButton::Middle => self.is_panning = is_pressed,
                    MouseButton::Left if is_pressed => {
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
                            let ray = tp_math::Ray::new(ray_origin, ray_dir);

                            if let Some(hit) = pick_face(&self.active_mesh, &self.mesh_bvh, &ray) {
                                tracing::info!("Yüzey seçildi: ID={:?}, Mesafe={:.2}", hit.face_id, hit.distance);
                                self.selected_face = Some(hit.face_id);
                            } else {
                                self.selected_face = None;
                            }
                        }
                    }
                    _ => {}
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some((last_x, last_y)) = self.last_mouse_pos {
                    let dx = (position.x - last_x) as f32;
                    let dy = (position.y - last_y) as f32;

                    if self.is_orbiting {
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

            WindowEvent::Resized(physical_size) => {
                tracing::debug!("Viewport boyutu: {}x{}", physical_size.width, physical_size.height);
                if let Some(gfx) = &mut self.gfx {
                    gfx.notify_resize();
                }
            }

            WindowEvent::RedrawRequested => {
                self.time.update();
                let dt = self.time.delta_seconds();

                // Sahne ECS & Fizik Simülasyonu Güncelle (Faz 5)
                self.scene.update(dt);

                // Animasyon Oynatıcısını ve İskelet Matrislerini Güncelle (Faz 4)
                let mut local_poses = vec![Transform::IDENTITY; self.skeleton.bone_count()];
                self.anim_player.update(dt, &self.skeleton, &mut local_poses);
                let _ = self.skeleton.compute_world_transforms(&local_poses, &mut self.bone_world_transforms);
                let _ = self.skeleton.compute_skinning_matrices(&self.bone_world_transforms, &mut self.skinning_matrices);

                let Some(window) = self.window.clone() else { return };
                let size = window.inner_size();
                let (view, proj, eye) = self.compute_camera_matrices(size.width, size.height);
                let model_matrix = self.compute_model_matrix();

                // 2. UI Güncellemesi (Tessellation)
                let editor_event = if let Some(editor) = &mut self.editor {
                    let mut scene_ctx = EditorSceneContext {
                        position: &mut self.mesh_position,
                        rotation: &mut self.mesh_rotation,
                        scale: &mut self.mesh_scale,
                        vertex_count: self.active_mesh.vertices.len(),
                        edge_count: self.active_mesh.edges.len(),
                        face_count: self.active_mesh.faces.len(),
                        selected_face: self.selected_face,
                        anim_time: &mut self.anim_player.current_time,
                        anim_duration: self.anim_duration,
                        anim_speed: &mut self.anim_player.speed,
                        anim_is_playing: self.anim_player.is_playing,
                        anim_looping: &mut self.anim_player.looping,
                    };
                    editor.update(&window, &mut scene_ctx)
                } else {
                    None
                };

                // UI Buton Olaylarını İşle
                if let Some(event) = editor_event {
                    match event {
                        EditorEvent::PlayGame => self.scene.start_play(),
                        EditorEvent::PauseGame => self.scene.pause_play(),
                        EditorEvent::StopGame => self.scene.stop_play(),
                        EditorEvent::PlayAnimation => self.anim_player.resume(),
                        EditorEvent::PauseAnimation => self.anim_player.pause(),
                        EditorEvent::StopAnimation => self.anim_player.stop(),
                        EditorEvent::ScrubAnimation(t) => self.anim_player.current_time = t,
                        EditorEvent::Extrude(dist) => {
                            if let Some(fid) = self.selected_face {
                                let cmd = SnapshotMeshCommand::new("Extrude Face", move |m| {
                                    m.extrude_face(fid, dist)?;
                                    Ok(())
                                });
                                if self.undo_stack.execute(cmd, &mut self.active_mesh).is_ok() {
                                    self.selected_face = Some(FaceId(self.active_mesh.faces.len() as u32 - 1));
                                    self.sync_mesh_to_gpu();
                                }
                            }
                        }
                        EditorEvent::Inset(factor) => {
                            if let Some(fid) = self.selected_face {
                                let cmd = SnapshotMeshCommand::new("Inset Face", move |m| {
                                    m.inset_face(fid, factor)?;
                                    Ok(())
                                });
                                if self.undo_stack.execute(cmd, &mut self.active_mesh).is_ok() {
                                    self.selected_face = Some(FaceId(self.active_mesh.faces.len() as u32 - 1));
                                    self.sync_mesh_to_gpu();
                                }
                            }
                        }
                        EditorEvent::Subdivide => {
                            let cmd = SnapshotMeshCommand::new("Subdivide Mesh", |m| {
                                *m = m.subdivide();
                                Ok(())
                            });
                            if self.undo_stack.execute(cmd, &mut self.active_mesh).is_ok() {
                                self.selected_face = None;
                                self.sync_mesh_to_gpu();
                            }
                        }
                        EditorEvent::Undo => {
                            if let Ok(true) = self.undo_stack.undo(&mut self.active_mesh) {
                                self.selected_face = None;
                                self.sync_mesh_to_gpu();
                            }
                        }
                        EditorEvent::Redo => {
                            if let Ok(true) = self.undo_stack.redo(&mut self.active_mesh) {
                                self.selected_face = None;
                                self.sync_mesh_to_gpu();
                            }
                        }
                        EditorEvent::AddPrimitive(name) => {
                            let new_m = match name {
                                "Cube" => create_cube(Vec3::splat(1.5)),
                                "Sphere" => create_uv_sphere(1.0, 16, 16),
                                "Cylinder" => create_cylinder(0.8, 1.8, 16),
                                "Plane" => create_plane(3.0, 3.0, 4, 4),
                                _ => create_cube(Vec3::splat(1.5)),
                            };
                            let cmd = SnapshotMeshCommand::new("Add Primitive", move |m| {
                                *m = new_m;
                                Ok(())
                            });
                            if self.undo_stack.execute(cmd, &mut self.active_mesh).is_ok() {
                                self.selected_face = None;
                                self.sync_mesh_to_gpu();
                            }
                        }
                    }
                }

                // 3. Vulkan Çizim Döngüsü
                if let Some(gfx) = &mut self.gfx {
                    let view_proj = proj * view;
                    let inv_view_proj = view_proj.inverse();

                    match gfx.begin_frame(size.width, size.height) {
                        Ok(Some(frame)) => {
                            let cmd = frame.command_buffer;
                            let device = &gfx.device.device;

                            // 1. Zemin Izgarası (Infinite Grid)
                            gfx.grid.draw(device, cmd, inv_view_proj, eye, 0.1, 1000.0);

                            // 2. 3D Model Çizimi
                            let mesh_visible = self.editor.as_ref().is_none_or(|e| e.mesh_visible);
                            let shading_mode = self.editor.as_ref().map_or(ShadingMode::MatCap, |e| e.shading_mode);

                            if mesh_visible {
                                if let Some(gpu) = &self.mesh_gpu {
                                    let render_mode = match shading_mode {
                                        ShadingMode::Wireframe => 0,
                                        ShadingMode::MatCap => 1,
                                        ShadingMode::Pbr => 2,
                                    };

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
                                        Vec4::new(0.82, 0.84, 0.88, 1.0),
                                        0.2,
                                        0.4,
                                    );
                                }
                            }

                            // 3. Seçili Yüzey Vurgusu ve Bounding Box (DebugDraw)
                            let aabb = self.active_mesh.compute_aabb();
                            gfx.debug_draw.draw_aabb(&aabb, Vec4::new(0.4, 0.7, 1.0, 0.6));
                            gfx.debug_draw.draw_axes(&Transform::IDENTITY, 1.2);

                            if let Some(fid) = self.selected_face {
                                let v_ids = self.active_mesh.face_vertices(fid);
                                for i in 0..v_ids.len() {
                                    let p0 = model_matrix.transform_point3(self.active_mesh.vertices[v_ids[i].0 as usize].position);
                                    let p1 = model_matrix.transform_point3(self.active_mesh.vertices[v_ids[(i + 1) % v_ids.len()].0 as usize].position);
                                    gfx.debug_draw.draw_line(p0, p1, Vec4::new(1.0, 0.8, 0.1, 1.0));
                                }
                            }

                            // İskelet Kemikleri Görselleştirmesi (Skeleton Debug Draw)
                            for i in 0..self.skeleton.bone_count() {
                                let bone = &self.skeleton.bones[i];
                                let bone_world_pos = model_matrix.transform_point3(
                                    self.bone_world_transforms[i].transform_point3(Vec3::ZERO),
                                );
                                if let Some(parent_id) = bone.parent {
                                    let parent_pos = model_matrix.transform_point3(
                                        self.bone_world_transforms[parent_id.0 as usize].transform_point3(Vec3::ZERO),
                                    );
                                    // Kemik segmenti (Parlak Turuncu/Altın)
                                    gfx.debug_draw.draw_line(parent_pos, bone_world_pos, Vec4::new(1.0, 0.75, 0.2, 1.0));
                                }
                            }

                            gfx.debug_draw.flush(device, cmd, view_proj);

                            // 4. Egui UI Arayüzünü Viewport Üzerine Çiz
                            if let Some(editor) = &mut self.editor {
                                if let Err(e) = editor.render(device, cmd, size.width as f32, size.height as f32) {
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
