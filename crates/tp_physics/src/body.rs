//! 3Point Rijit Cisim (RigidBody) Tanımları.

use crate::math_conv::transform_to_isometry;
use glam::{Quat, Vec3};
use rapier3d::prelude as rapier;

/// Cisim hareket türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RigidBodyType {
    /// Dinamik: Yerçekimi, kuvvetler ve çarpışmalardan tam etkilenir.
    #[default]
    Dynamic,
    /// Sabit (Statik): Zemin, duvarlar ve çevre engelleri. Asla hareket etmez.
    Fixed,
    /// Kinematik (Pozisyon Tabanlı): Doğrudan kod veya animasyon ile hareket ettirilir.
    KinematicPositionBased,
    /// Kinematik (Hız Tabanlı): Doğrudan hız atanarak kontrol edilir.
    KinematicVelocityBased,
}

impl From<RigidBodyType> for rapier::RigidBodyType {
    fn from(t: RigidBodyType) -> Self {
        match t {
            RigidBodyType::Dynamic => rapier::RigidBodyType::Dynamic,
            RigidBodyType::Fixed => rapier::RigidBodyType::Fixed,
            RigidBodyType::KinematicPositionBased => rapier::RigidBodyType::KinematicPositionBased,
            RigidBodyType::KinematicVelocityBased => rapier::RigidBodyType::KinematicVelocityBased,
        }
    }
}

/// Rijit cisim oluşturma tanımlayıcısı (Builder pattern).
#[derive(Debug, Clone)]
pub struct RigidBodyDesc {
    pub translation: Vec3,
    pub rotation: Quat,
    pub body_type: RigidBodyType,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub gravity_scale: f32,
    pub can_sleep: bool,
    pub ccd_enabled: bool,
}

impl Default for RigidBodyDesc {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            body_type: RigidBodyType::Dynamic,
            linear_damping: 0.05,
            angular_damping: 0.05,
            gravity_scale: 1.0,
            can_sleep: true,
            ccd_enabled: false,
        }
    }
}

impl RigidBodyDesc {
    pub fn new(body_type: RigidBodyType) -> Self {
        Self {
            body_type,
            ..Default::default()
        }
    }

    pub fn dynamic() -> Self {
        Self::new(RigidBodyType::Dynamic)
    }

    pub fn fixed() -> Self {
        Self::new(RigidBodyType::Fixed)
    }

    pub fn kinematic() -> Self {
        Self::new(RigidBodyType::KinematicPositionBased)
    }

    pub fn translation(mut self, t: Vec3) -> Self {
        self.translation = t;
        self
    }

    pub fn rotation(mut self, r: Quat) -> Self {
        self.rotation = r;
        self
    }

    pub fn gravity_scale(mut self, scale: f32) -> Self {
        self.gravity_scale = scale;
        self
    }

    pub fn ccd(mut self, enabled: bool) -> Self {
        self.ccd_enabled = enabled;
        self
    }

    /// Rapier iç yapısına dönüştürür.
    pub fn build(&self) -> rapier::RigidBody {
        let iso = transform_to_isometry(self.translation, self.rotation);
        let mut builder = match self.body_type {
            RigidBodyType::Dynamic => rapier::RigidBodyBuilder::dynamic(),
            RigidBodyType::Fixed => rapier::RigidBodyBuilder::fixed(),
            RigidBodyType::KinematicPositionBased => rapier::RigidBodyBuilder::kinematic_position_based(),
            RigidBodyType::KinematicVelocityBased => rapier::RigidBodyBuilder::kinematic_velocity_based(),
        };

        builder = builder
            .position(iso)
            .linear_damping(self.linear_damping)
            .angular_damping(self.angular_damping)
            .gravity_scale(self.gravity_scale)
            .can_sleep(self.can_sleep)
            .ccd_enabled(self.ccd_enabled);

        builder.build()
    }
}

/// Tip güvenli rijit cisim anahtarı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RigidBodyHandle(pub rapier::RigidBodyHandle);
