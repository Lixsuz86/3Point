//! 3Point Yarım-Kenar (Half-Edge / DCEL) Veri Yapısı ve Tip Tanımlamaları.

use glam::{Vec2, Vec3};

/// Bir Köşeyi (Vertex) temsil eden benzersiz tip güvenli kimlik.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexId(pub u32);

/// Bir Yarım-Kenarı (Half-Edge) temsil eden benzersiz tip güvenli kimlik.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HalfEdgeId(pub u32);

/// Çift yönlü bir Kenarı (Edge) temsil eden benzersiz tip güvenli kimlik.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(pub u32);

/// Bir Yüzeyi (Face / N-gon) temsil eden benzersiz tip güvenli kimlik.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceId(pub u32);

/// 3B uzayda bir köşe noktası.
#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    /// 3B Dünya/Yerel uzay koordinatı.
    pub position: Vec3,
    /// Köşe normal vektörü.
    pub normal: Vec3,
    /// Doku koordinatı (UV).
    pub uv: Vec2,
    /// Bu köşeden çıkan yarım-kenarlardan biri.
    pub half_edge: Option<HalfEdgeId>,
    /// Silinmişlik durumu (Bellek geri dönüşümü için).
    pub is_deleted: bool,
}

impl Vertex {
    #[inline]
    #[must_use]
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            normal: Vec3::Y,
            uv: Vec2::ZERO,
            half_edge: None,
            is_deleted: false,
        }
    }
}

/// Yönlendirilmiş yarım-kenar (Half-Edge). DCEL topolojisinin temel yapı taşıdır.
#[derive(Debug, Clone, PartialEq)]
pub struct HalfEdge {
    /// Bu yarım-kenarın başladığı köşe noktası.
    pub origin: VertexId,
    /// Zıt yöndeki eş yarım-kenar (Sınır kenarlarında `None` olabilir).
    pub twin: Option<HalfEdgeId>,
    /// Yüzey etrafında saat yönünün tersindeki bir sonraki yarım-kenar.
    pub next: HalfEdgeId,
    /// Yüzey etrafında bir önceki yarım-kenar.
    pub prev: HalfEdgeId,
    /// Bu kenarın sınırlandırdığı yüzey.
    pub face: Option<FaceId>,
    /// Bağlı olduğu yönsüz üst kenar.
    pub edge: EdgeId,
    /// Silinmişlik durumu.
    pub is_deleted: bool,
}

/// İki zıt yarım-kenardan oluşan yönsüz kenar.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// Kenarı oluşturan yarım-kenarlardan birincisi.
    pub half_edge: HalfEdgeId,
    /// Silinmişlik durumu.
    pub is_deleted: bool,
}

/// Poligon yüzeyi (Üçgen, Dörtgen veya N-gon).
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    /// Yüzeyin sınırındaki yarım-kenarlardan biri.
    pub half_edge: HalfEdgeId,
    /// Yüzey normal vektörü.
    pub normal: Vec3,
    /// Silinmişlik durumu.
    pub is_deleted: bool,
}

