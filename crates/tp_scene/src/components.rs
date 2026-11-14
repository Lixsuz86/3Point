//! 3Point Varlık Bileşen Sistemi (ECS) Temel Bileşenleri.

use glam::{Mat4, Vec3, Vec4};
use tp_math::Transform;
use tp_physics::{ColliderHandle, RigidBodyHandle};

/// Sahne nesnesi için okunabilir isim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(pub String);

impl Name {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Nesnenin yerel uzaydaki dönüşümü (Position, Rotation, Scale).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformComponent(pub Transform);

impl Default for TransformComponent {
    fn default() -> Self {
        Self(Transform::IDENTITY)
    }
}

/// Hiyerarşik hesaplamalar sonucu elde edilen dünya uzayı matrisi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalTransform(pub Mat4);

impl Default for GlobalTransform {
    fn default() -> Self {
        Self(Mat4::IDENTITY)
    }
}

/// Sahne grafiği ağaç hiyerarşisi (Ebeveyn - Çocuk ilişkisi).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Hierarchy {
    pub parent: Option<hecs::Entity>,
    pub children: Vec<hecs::Entity>,
}

/// 3D Çizim ve Materyal Özellikleri Bileşeni.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshRenderer {
    /// Çizilecek ilkel geometri tipi ("Cube", "Sphere", "Cylinder", "Plane").
    pub primitive: &'static str,
    /// Modelin sahnede görünür olup olmadığı.
    pub visible: bool,
    /// Ana yüzey rengi (RGBA).
    pub color: Vec4,
    /// Metalik katsayısı (0.0 = dielektrik, 1.0 = tam metal).
    pub metallic: f32,
    /// Pürüzlülük katsayısı (0.0 = ayna gibi pürüzsüz, 1.0 = mat).
    pub roughness: f32,
}

impl Default for MeshRenderer {
    fn default() -> Self {
        Self {
            primitive: "Cube",
            visible: true,
            color: Vec4::new(0.8, 0.8, 0.85, 1.0),
            metallic: 0.1,
            roughness: 0.4,
        }
    }
}

/// Kamera Projeksiyon Türü.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraComponent {
    pub fov_y_rad: f32,
    pub near: f32,
    pub far: f32,
    pub is_active: bool,
    pub is_orthographic: bool,
    pub ortho_size: f32,
}

impl Default for CameraComponent {
    fn default() -> Self {
        Self {
            fov_y_rad: 60.0_f32.to_radians(),
            near: 0.1,
            far: 1000.0,
            is_active: true,
            is_orthographic: false,
            ortho_size: 5.0,
        }
    }
}

/// Işık Kaynağı Türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LightType {
    /// Yönsüz Güneş Işığı (Directional Light).
    #[default]
    Directional,
    /// Noktasal Işık (Point Light).
    Point,
    /// Spot Işık (Spot Light).
    Spot,
}

/// Sahne Aydınlatma Bileşeni.
#[derive(Debug, Clone, PartialEq)]
pub struct LightComponent {
    pub kind: LightType,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
}

impl Default for LightComponent {
    fn default() -> Self {
        Self {
            kind: LightType::Directional,
            color: Vec3::ONE,
            intensity: 1.0,
            range: 20.0,
        }
    }
}

/// Fizik dünyasındaki rijit cisim bağlantısı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsBodyComponent(pub RigidBodyHandle);

/// Fizik dünyasındaki çarpıştırıcı bağlantısı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsColliderComponent(pub ColliderHandle);
