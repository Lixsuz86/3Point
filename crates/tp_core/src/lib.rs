//! # `tp_core`
//!
//! 3Point motorunun temelini oluşturan asenkron loglama, paralel görev havuzu,
//! kare zamanlayıcısı (frame timing) ve GPU halka tamponu kütüphanesi.

pub mod error;
pub mod logging;
pub mod ring_buffer;
pub mod tasks;
pub mod time;

pub use error::CoreError;
pub use logging::init_logging;
pub use ring_buffer::FrameRingBuffer;
pub use tasks::TaskPool;
pub use time::Time;

