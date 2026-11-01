//! 3Point Ana Uygulama Durum Makinesi ve Vulkan Pencere Entegrasyonu (`winit 0.30`).

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::sync::Arc;
use tp_core::{TaskPool, Time};
use tp_gfx::GfxContext;
use tp_math::{Transform, Vec3};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
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
    camera_transform: Transform,
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
            camera_transform: Transform::from_translation(Vec3::new(0.0, 2.0, 5.0)),
        }
    }

    /// Arka plan görev havuzuna erişim sağlar.
    #[inline]
    #[must_use]
    pub fn task_pool(&self) -> &TaskPool {
        &self.task_pool
    }

    /// Kamera dönüşümüne erişim sağlar.
    #[inline]
    #[must_use]
    pub fn camera_transform(&self) -> &Transform {
        &self.camera_transform
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("3Point Studio - Vulkan (v0.1.0-alpha)")
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

            // Vulkan Grafik Bağlamını (GfxContext) Başlat
            let display_handle = match event_loop.display_handle() {
                Ok(dh) => dh.as_raw(),
                Err(err) => {
                    tracing::error!("Ekran tutucusu (Display Handle) alınamadı: {err}");
                    event_loop.exit();
                    return;
                }
            };

            let window_handle = match window.window_handle() {
                Ok(wh) => wh.as_raw(),
                Err(err) => {
                    tracing::error!("Pencere tutucusu (Window Handle) alınamadı: {err}");
                    event_loop.exit();
                    return;
                }
            };

            match GfxContext::new(display_handle, window_handle, size.width, size.height) {
                Ok(gfx) => {
                    tracing::info!("Vulkan Grafik Motoru (tp_gfx) başarıyla pencereye bağlandı!");
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
                tracing::info!("ESC tuşuna basıldı. Uygulamadan çıkılıyor...");
                event_loop.exit();
            }

            WindowEvent::Resized(physical_size) => {
                tracing::debug!(
                    "Viewport yeniden boyutlandırıldı: {}x{}",
                    physical_size.width,
                    physical_size.height
                );
                if let Some(gfx) = &mut self.gfx {
                    gfx.notify_resize();
                }
            }

            WindowEvent::RedrawRequested => {
                self.time.update();

                // Her 120 karede bir FPS ve kamera durumu logla
                if self.time.frame_count().is_multiple_of(120) {
                    tracing::debug!(
                        "Vulkan Viewport | FPS: {:.1} | Frame: {} | Kamera: {:?}",
                        self.time.fps(),
                        self.time.frame_count(),
                        self.camera_transform().translation
                    );
                }

                // Vulkan Çizim Döngüsü
                if let (Some(gfx), Some(window)) = (&mut self.gfx, &self.window) {
                    let size = window.inner_size();
                    match gfx.begin_frame(size.width, size.height) {
                        Ok(Some(frame)) => {
                            // Çizim komutları (RenderPass) burada yürütülür
                            if let Err(err) = gfx.end_frame(frame, size.width, size.height) {
                                tracing::error!("Kare sunum hatası: {err}");
                            }
                        }
                        Ok(None) => {
                            // Swapchain yeniden boyutlandırıldı veya pencere simge durumunda
                        }
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
