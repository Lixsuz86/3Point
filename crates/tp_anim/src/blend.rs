//! 3Point Animasyon Karıştırma (Pose Blending) ve Katmanlama.
//!
//! Doğrusal poz harmanlama (Linear Blend), kemik maskeleme (Bone Masking) ve eklemeli katmanlama (Additive Blending).

use tp_math::Transform;

/// İki dönüşüm arasındaki ağırlıklı geçişi hesaplar.
#[inline]
pub fn blend_transforms(a: &Transform, b: &Transform, weight: f32) -> Transform {
    let w = weight.clamp(0.0, 1.0);
    Transform {
        translation: a.translation.lerp(b.translation, w),
        rotation: a.rotation.slerp(b.rotation, w),
        scale: a.scale.lerp(b.scale, w),
    }
}

/// İki iskelet pozu arasındaki tüm kemik dönüşümlerini harmanlar.
///
/// $Pose_{out} = \text{lerp}(Pose_A, Pose_B, weight)$
pub fn blend_poses(
    pose_a: &[Transform],
    pose_b: &[Transform],
    weight: f32,
    out_pose: &mut [Transform],
) {
    let count = pose_a.len().min(pose_b.len()).min(out_pose.len());
    let w = weight.clamp(0.0, 1.0);

    for i in 0..count {
        out_pose[i] = blend_transforms(&pose_a[i], &pose_b[i], w);
    }
}

/// Kemik bazlı maske ağırlıkları (`bone_weights`) kullanarak iki pozu harmanlar.
///
/// Örn: Üst gövde ateş ederken alt gövdenin yürümesi için kullanılır.
pub fn blend_poses_masked(
    pose_a: &[Transform],
    pose_b: &[Transform],
    bone_weights: &[f32],
    out_pose: &mut [Transform],
) {
    let count = pose_a.len().min(pose_b.len()).min(out_pose.len()).min(bone_weights.len());

    for i in 0..count {
        let w = bone_weights[i].clamp(0.0, 1.0);
        out_pose[i] = blend_transforms(&pose_a[i], &pose_b[i], w);
    }
}

/// Referans pozuna göre farkı (delta) hesaplayıp temel poza ekler (Additive Blend).
///
/// Örn: Yürüme veya koşma üzerine eklenen nefes alma veya silah tepme animasyonları.
pub fn additive_blend(
    base_pose: &[Transform],
    add_pose: &[Transform],
    ref_pose: &[Transform],
    weight: f32,
    out_pose: &mut [Transform],
) {
    let count = base_pose
        .len()
        .min(add_pose.len())
        .min(ref_pose.len())
        .min(out_pose.len());
    let w = weight.clamp(0.0, 1.0);

    for i in 0..count {
        let trans_delta = add_pose[i].translation - ref_pose[i].translation;
        let rot_delta = add_pose[i].rotation * ref_pose[i].rotation.conjugate();
        let scale_delta = add_pose[i].scale - ref_pose[i].scale;

        out_pose[i] = Transform {
            translation: base_pose[i].translation + trans_delta * w,
            rotation: (rot_delta * w).slerp(base_pose[i].rotation, 1.0 - w),
            scale: base_pose[i].scale + scale_delta * w,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tp_math::Vec3;

    #[test]
    fn test_blend_poses() {
        let pose_a = vec![Transform::from_translation(Vec3::new(0.0, 0.0, 0.0))];
        let pose_b = vec![Transform::from_translation(Vec3::new(10.0, 20.0, 30.0))];
        let mut out = vec![Transform::IDENTITY];

        blend_poses(&pose_a, &pose_b, 0.5, &mut out);

        assert!((out[0].translation.x - 5.0).abs() < 1e-4);
        assert!((out[0].translation.y - 10.0).abs() < 1e-4);
        assert!((out[0].translation.z - 15.0).abs() < 1e-4);
    }
}

