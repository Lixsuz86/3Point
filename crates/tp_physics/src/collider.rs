//! 3Point Çarpıştırıcı (Collider) Tanımları ve Geometrik Şekiller.

use crate::math_conv::vec3_to_na;
use glam::Vec3;
use rapier3d::prelude as rapier;

/// Çarpışma geometrisi şekilleri.
#[derive(Debug, Clone, PartialEq)]
pub enum ColliderShape {
    /// Kutu (Yarı boyutlar: half_extents X, Y, Z).
    Cuboid(Vec3),
    /// Küre (Yarıçap).
    Sphere(f32),
    /// Kapsül (Yarı yükseklik ve yarıçap).
    Capsule { half_height: f32, radius: f32 },
    /// Silindir (Yarı yükseklik ve yarıçap).
    Cylinder { half_height: f32, radius: f32 },
    /// Statik Üçgen Ağ (TriMesh: Karmaşık zemin/arazi mesh'leri için).
    TriMesh {
        vertices: Vec<Vec3>,
        indices: Vec<[u32; 3]>,
    },
}

/// Çarpıştırıcı oluşturma tanımlayıcısı (Builder pattern).
#[derive(Debug, Clone)]
pub struct ColliderDesc {
    pub shape: ColliderShape,
    pub friction: f32,
    pub restitution: f32,
    pub density: f32,
    pub is_sensor: bool,
}

impl Default for ColliderDesc {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Cuboid(Vec3::splat(0.5)),
            friction: 0.5,
            restitution: 0.0,
            density: 1.0,
            is_sensor: false,
        }
    }
}

impl ColliderDesc {
    pub fn cuboid(half_extents: Vec3) -> Self {
        Self {
            shape: ColliderShape::Cuboid(half_extents),
            ..Default::default()
        }
    }

    pub fn sphere(radius: f32) -> Self {
        Self {
            shape: ColliderShape::Sphere(radius),
            ..Default::default()
        }
    }

    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self {
            shape: ColliderShape::Capsule { half_height, radius },
            ..Default::default()
        }
    }

    pub fn cylinder(half_height: f32, radius: f32) -> Self {
        Self {
            shape: ColliderShape::Cylinder { half_height, radius },
            ..Default::default()
        }
    }

    pub fn trimesh(vertices: Vec<Vec3>, indices: Vec<[u32; 3]>) -> Self {
        Self {
            shape: ColliderShape::TriMesh { vertices, indices },
            ..Default::default()
        }
    }

    pub fn friction(mut self, f: f32) -> Self {
        self.friction = f;
        self
    }

    pub fn restitution(mut self, r: f32) -> Self {
        self.restitution = r;
        self
    }

    pub fn sensor(mut self, s: bool) -> Self {
        self.is_sensor = s;
        self
    }

    /// Rapier iç çarpıştırıcı nesnesine dönüştürür.
    pub fn build(&self) -> rapier::Collider {
        let builder = match &self.shape {
            ColliderShape::Cuboid(half) => rapier::ColliderBuilder::cuboid(half.x, half.y, half.z),
            ColliderShape::Sphere(r) => rapier::ColliderBuilder::ball(*r),
            ColliderShape::Capsule { half_height, radius } => {
                rapier::ColliderBuilder::capsule_y(*half_height, *radius)
            }
            ColliderShape::Cylinder { half_height, radius } => {
                rapier::ColliderBuilder::cylinder(*half_height, *radius)
            }
            ColliderShape::TriMesh { vertices, indices } => {
                let na_pts: Vec<_> = vertices.iter().map(|&v| vec3_to_na(v).into()).collect();
                rapier::ColliderBuilder::trimesh(na_pts, indices.clone())
            }
        };

        builder
            .friction(self.friction)
            .restitution(self.restitution)
            .density(self.density)
            .sensor(self.is_sensor)
            .build()
    }
}

/// Tip güvenli çarpıştırıcı anahtarı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColliderHandle(pub rapier::ColliderHandle);
