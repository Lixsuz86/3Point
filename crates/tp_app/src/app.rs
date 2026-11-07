//! 3Point Ana Uygulama Durum Makinesi ve Vulkan Viewport Entegrasyonu (`winit 0.30`).

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::sync::Arc;
use tp_core::{TaskPool, Time};
use tp_gfx::GfxContext;
use tp_math::{Aabb, Mat4, Transform, Vec3, Vec4};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

/// 3Point masaüstü uygulamasının çalışma zamanı durumu.
pub struct App {
    window: Option<Arc<Window>>,
    gfx: Option<GfxContext>,
    time: Time,
    task_pool: TaskPool,

    // Kamera Orbit & Navigasyon Durumu
    camera_target: Vec3,
    camera_distance: f32,
    camera_yaw: f32,
    camera_pitch: f32,

    // Fare Etkileşim Durumları
    is_orbiting: bool,
    is_panning: bool,
    last_mouse_pos: Option<(f64, f64)>,
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
        tracing::info!(
            "3Point App nesnesi başlatılıyor (İş parçacığı havuzu: {} çekirdek)...",
            task_pool.num_threads()
        );

        Self {
            window: None,
            gfx: None,
            time: Time::new(),
            task_pool,
            camera_target: Vec3::new(0.0, 0.5, 0.0),
            camera_distance: 6.0,
            camera_yaw: 0.785,  // 45 derece
            camera_pitch: 0.523, // 30 derece
            is_orbiting: false,
            is_panning: false,
            last_mouse_pos: None,
        }
    }

    /// Arka plan görev havuzuna erişim sağlar.
    #[inline]
    #[must_use]
    pub fn task_pool(&self) -> &TaskPool {
        &self.task_pool
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
        // Vulkan için Y ekseni ters çevrilmiş sağ el perspektif matrisi
        let mut proj = Mat4::perspective_rh(45.0f32.to_radians(), aspect, 0.1, 1000.0);
        proj.y_axis.y *= -1.0;

        (view, proj, eye)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("3Point Studio - Vulkan Viewport (v0.1.0-alpha)")
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
                "Pencere başarıyla oluşturuldu: ID={:?}, Boyut: {}x{}, Görev Havuzu: {} iş parçacığı",
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
                Ok(gfx) => {
                    tracing::info!("Vulkan Grafik Motoru (Infinite Grid, DebugDraw, MeshPipeline) pencereye bağlandı!");
                    self.gfx = Some(gfx);
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
        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Kullanıcı pencereyi kapattı. Çıkış yapılıyor...");
                event_loop.exit();
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                tracing::info!("ESC basıldı, çıkış yapılıyor.");
                event_loop.exit();
            }

            WindowEvent::MouseInput { button, state, .. } => {
                let is_pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Right => self.is_orbiting = is_pressed,
                    MouseButton::Middle => self.is_panning = is_pressed,
                    _ => {}
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some((last_x, last_y)) = self.last_mouse_pos {
                    let dx = (position.x - last_x) as f32;
                    let dy = (position.y - last_y) as f32;

                    if self.is_orbiting {
                        // Kamera etrafında dönme (Orbit)
                        self.camera_yaw -= dx * 0.006;
                        self.camera_pitch = (self.camera_pitch + dy * 0.006).clamp(-1.5, 1.5);
                    } else if self.is_panning {
                        // Kamerayı odak noktasıyla kaydırma (Pan)
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

            WindowEvent::MouseWheel { delta, .. } => {
                let zoom_amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 0.5,
                    MouseScrollDelta::PixelDelta(p) => (p.y as f32) * 0.02,
                };
                self.camera_distance = (self.camera_distance - zoom_amount).clamp(1.0, 100.0);
            }

            WindowEvent::Resized(physical_size) => {
                tracing::debug!("Viewport boyutu güncellendi: {}x{}", physical_size.width, physical_size.height);
                if let Some(gfx) = &mut self.gfx {
                    gfx.notify_resize();
                }
            }

            WindowEvent::RedrawRequested => {
                self.time.update();

                if self.time.frame_count().is_multiple_of(120) {
                    tracing::debug!(
                        "3Point Viewport | FPS: {:.1} | Frame: {} | Kamera Mesafe: {:.2}",
                        self.time.fps(),
                        self.time.frame_count(),
                        self.camera_distance
                    );
                }

                let Some(window) = &self.window else { return; };
                let size = window.inner_size();
                let (view, proj, eye) = self.compute_camera_matrices(size.width, size.height);

                if let Some(gfx) = &mut self.gfx {
                    let view_proj = proj * view;
                    let inv_view_proj = view_proj.inverse();

                    match gfx.begin_frame(size.width, size.height) {
                        Ok(Some(frame)) => {
                            let cmd = frame.command_buffer;
                            let device = &gfx.device.device;

                            // 1. Sonsuz Zemin Izgarası Çizimi (Infinite Grid)
                            gfx.grid.draw(device, cmd, inv_view_proj, eye, 0.1, 1000.0);

                            // 2. 3D Koordinat Eksenleri ve Örnek Bounding Box Çizimi (DebugDraw)
                            gfx.debug_draw.draw_axes(&Transform::IDENTITY, 1.0);
                            let demo_box = Aabb::new(Vec3::new(-0.5, 0.0, -0.5), Vec3::new(0.5, 1.0, 0.5));
                            gfx.debug_draw.draw_aabb(&demo_box, Vec4::new(0.9, 0.75, 0.2, 1.0));
                            gfx.debug_draw.flush(device, cmd, view_proj);

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
