//! 3Point 3D Fizik Motoru Hata Türleri (`PhysicsError`).

use thiserror::Error;

/// Fizik simülasyonu ve cisim manipülasyonu hataları.
#[derive(Debug, Error, PartialEq)]
pub enum PhysicsError {
    #[error("Geçersiz rijit cisim tanıtıcısı (RigidBodyHandle)")]
    InvalidRigidBody,

    #[error("Geçersiz çarpıştırıcı tanıtıcısı (ColliderHandle)")]
    InvalidCollider,

    #[error("Üçgen ağ (TriMesh) geometrisi boş veya geçersiz")]
    InvalidTriMeshGeometry,

    #[error("Fizik simülasyon adımı başarısız: {0}")]
    SimulationFailed(String),
}
