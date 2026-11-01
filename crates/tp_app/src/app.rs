//! 3Point Ana Uygulama Durum Makinesi ve Pencere Yönetimi (`winit 0.30`).

use std::sync::Arc;
use tp_core::{TaskPool, Time};
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
                .with_title("3Point Studio (v0.1.0-alpha)")
                .with_inner_size(LogicalSize::new(1280.0, 720.0))
                .with_min_inner_size(LogicalSize::new(800.0, 600.0));

            match event_loop.create_window(window_attributes) {
                Ok(window) => {
                    let window = Arc::new(window);
                    tracing::info!(
                        "Pencere başarıyla oluşturuldu: ID={:?}, Boyut: {:?}, Görev Havuzu: {} iş parçacığı",
                        window.id(),
                        window.inner_size(),
                        self.task_pool().num_threads()
                    );
                    self.window = Some(window);
                }
                Err(err) => {
                    tracing::error!("Pencere oluşturulamadı: {err}");
                    event_loop.exit();
                }
            }
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
            }

            WindowEvent::RedrawRequested => {
                self.time.update();

                // Her 120 karede bir FPS ve kamera bilgisi logla (debug)
                if self.time.frame_count().is_multiple_of(120) {
                    tracing::debug!(
                        "FPS: {:.1} | Frame: {} | Kamera Pos: {:?}",
                        self.time.fps(),
                        self.time.frame_count(),
                        self.camera_transform().translation
                    );
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}
