//! 3Point Masaüstü Başlatıcı (Launcher).

mod app;

use app::App;
use winit::event_loop::{ControlFlow, EventLoop};

fn main() -> anyhow::Result<()> {
    // 1. Loglama altyapısını başlat
    tp_core::init_logging();
    tracing::info!(">>> 3Point Studio Başlatılıyor <<<");

    // 2. OS Olay Döngüsünü oluştur
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    // 3. Uygulamayı çalıştır
    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    tracing::info!("3Point Studio sorunsuz kapatıldı.");
    Ok(())
}

