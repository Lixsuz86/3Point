//! 3Point Lineer Cebir ve Koordinat Dönüşümleri Modülü.

use glam::{Mat4, Quat, Vec3};

/// 3D uzaydaki bir nesnenin konum (translation), rotasyon (rotation) ve ölçeğini (scale)
/// temsil eden dönüşüm yapısı.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// 3B Dünya veya ebeveyn uzayındaki konum.
    pub translation: Vec3,
    /// Yönelim (Rotasyon) kuaterniyonu (Birim kuaterniyon olmalıdır).
    pub rotation: Quat,
    /// Eksenel ölçekleme faktörleri.
    pub scale: Vec3,
}

impl Default for Transform {
    #[inline]
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    /// Birim (Identity) dönüşüm değeri.
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Yalnızca öteleme içeren bir dönüşüm oluşturur.
    #[inline]
    #[must_use]
    pub const fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// Yalnızca rotasyon içeren bir dönüşüm oluşturur.
    #[inline]
    #[must_use]
    pub const fn from_rotation(rotation: Quat) -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation,
            scale: Vec3::ONE,
        }
    }

    /// Yalnızca ölçekleme içeren bir dönüşüm oluşturur.
    #[inline]
    #[must_use]
    pub const fn from_scale(scale: Vec3) -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale,
        }
    }

    /// Yeni bir `Transform` oluşturur.
    #[inline]
    #[must_use]
    pub const fn from_trs(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Verilen hedef noktaya bakan dönüşüm oluşturur.
    #[inline]
    #[must_use]
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let forward = (target - eye).normalize_or_zero();
        let rotation = if forward.length_squared() < f32::EPSILON {
            Quat::IDENTITY
        } else {
            let right = forward.cross(up).normalize_or_zero();
            let actual_up = right.cross(forward).normalize_or_zero();
            glam::Quat::from_mat3(&glam::Mat3::from_cols(right, actual_up, -forward))
        };

        Self {
            translation: eye,
            rotation,
            scale: Vec3::ONE,
        }
    }

    /// Dönüşümü 4x4 homojen afin matrisine çevirir.
    #[inline]
    #[must_use]
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// 3B uzaydaki bir noktayı bu dönüşüm matrisiyle öteler, döndürür ve ölçekler.
    #[inline]
    #[must_use]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.translation + (self.rotation * (self.scale * point))
    }

    /// 3B uzaydaki bir vektörü döndürür ve ölçekler (öteleme uygulanmaz).
    #[inline]
    #[must_use]
    pub fn transform_vector(&self, vector: Vec3) -> Vec3 {
        self.rotation * (self.scale * vector)
    }

    /// İki dönüşümü hiyerarşik olarak birleştirir (Parent * Child).
    #[inline]
    #[must_use]
    pub fn mul_transform(&self, child: &Transform) -> Transform {
        Transform {
            translation: self.transform_point(child.translation),
            rotation: (self.rotation * child.rotation).normalize(),
            scale: self.scale * child.scale,
        }
    }

    /// Dönüşümün tersini hesaplar.
    #[inline]
    #[must_use]
    pub fn inverse(&self) -> Self {
        let inv_scale = Vec3::select(
            self.scale.cmpeq(Vec3::ZERO),
            Vec3::ZERO,
            Vec3::ONE / self.scale,
        );
        let inv_rot = self.rotation.inverse();
        let inv_trans = inv_rot * (-self.translation * inv_scale);

        Self {
            translation: inv_trans,
            rotation: inv_rot,
            scale: inv_scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn test_identity_transform() {
        let t = Transform::IDENTITY;
        let pt = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(t.transform_point(pt), pt);
        assert_eq!(t.to_matrix(), Mat4::IDENTITY);
    }

    #[test]
    fn test_translation_and_scale() {
        let t = Transform::from_trs(
            Vec3::new(10.0, 0.0, -5.0),
            Quat::IDENTITY,
            Vec3::splat(2.0),
        );
        let pt = Vec3::new(1.0, 2.0, 3.0);
        let expected = Vec3::new(12.0, 4.0, 1.0);
        assert_eq!(t.transform_point(pt), expected);
    }

    #[test]
    fn test_inverse_transform() {
        let t = Transform::from_trs(
            Vec3::new(5.0, -3.0, 2.0),
            Quat::from_rotation_y(FRAC_PI_2),
            Vec3::new(2.0, 2.0, 2.0),
        );
        let pt = Vec3::new(1.0, 2.0, 3.0);
        let transformed = t.transform_point(pt);
        let inv = t.inverse();
        let restored = inv.transform_point(transformed);

        assert!((restored - pt).length() < 1e-5);
    }
}

