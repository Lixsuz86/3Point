//! 3Point Sahne ve ECS Hata Türleri (`SceneError`).

use thiserror::Error;

/// Sahne hiyerarşisi ve varlık manipülasyon hataları.
#[derive(Debug, Error, PartialEq)]
pub enum SceneError {
    #[error("Varlık bulunamadı: {0:?}")]
    EntityNotFound(hecs::Entity),

    #[error("Hiyerarşide döngüsel ebeveynlik tespit edildi: Varlık {0:?}")]
    CyclicParenting(hecs::Entity),

    #[error("Bileşen eksik veya bulunamadı: {0}")]
    ComponentMissing(&'static str),
}
