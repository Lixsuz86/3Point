//! 3Point Loglama ve Teşhis Altyapısı (`tracing`).

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Uygulamanın loglama katmanını başlatır.
/// `RUST_LOG` ortam değişkeni tanımlıysa onu kullanır, tanımlı değilse varsayılan
/// filtre kurallarını uygular (`info`, motor crate'leri için `debug`).
pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,tp_core=debug,tp_math=debug,tp_app=debug,tp_gfx=debug")
    });

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .compact();

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .try_init();

    tracing::info!("3Point Çekirdek Loglama Sistemi Devreye Alındı.");
}

