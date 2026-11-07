//! 3Point 3D Modelleme ve Topoloji Hata Türleri.

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum MeshError {
    #[error("Geçersiz yüzey topolojisi: En az 3 köşe gereklidir, bulunan: {0}")]
    DegenerateFace(usize),

    #[error("Manifold olmayan kenar tespit edildi: Aynı kenarı paylaşan ikiden fazla yüzey olamaz.")]
    NonManifoldEdge,

    #[error("Geçersiz köşe indeksi: {0}")]
    InvalidVertexId(u32),

    #[error("Geçersiz yüzey indeksi: {0}")]
    InvalidFaceId(u32),

    #[error("Geçersiz yarım-kenar indeksi: {0}")]
    InvalidHalfEdgeId(u32),

    #[error("Boş mesh üzerinde bu operasyon gerçekleştirilemez.")]
    EmptyMesh,

    #[error("Topolojik işlem hatası: {0}")]
    TopologyError(String),
}
