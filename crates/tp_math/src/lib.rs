//! # `tp_math`
//!
//! 3Point motoru ve editörü için SIMD destekli temel matematik, dönüşüm hiyerarşisi
//! ve 3B geometri/hacim kesişim algoritmaları kütüphanesi.

pub mod geometry;
pub mod transform;

// Yeniden dışa aktarılan yaygın glam türleri
pub use glam::{
    self, Affine3A, EulerRot, Mat2, Mat3, Mat4, Quat, Vec2, Vec3, Vec3A, Vec4,
};

// Geometri ve dönüşüm yapılarının doğrudan dışa aktarımı
pub use geometry::{Aabb, Frustum, Plane, Ray};
pub use transform::Transform;

