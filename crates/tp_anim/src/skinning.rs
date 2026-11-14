//! 3Point Deri Kaplama (GPU/CPU Skeletal Skinning).
//!
//! Linear Blend Skinning (LBS) ve hacim kaybını engelleyen Dual Quaternion Skinning (DQS) desteklenir.

use bytemuck::{Pod, Zeroable};
use tp_math::{Mat4, Quat, Vec3, Vec4};

/// Bir GPU UBO/SSBO tamponunda saklanabilecek maksimum kemik sayısı.
///
/// 256 kemik $\times$ 64 bayt = 16 KB (Vulkan standart 64 KB UBO sınırının rahatça içindedir).
pub const MAX_BONES: usize = 256;

/// Bir tepe noktasına etki eden kemik indeksleri ve normalize ağırlıkları.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct JointWeights {
    /// Etki eden kemik indeksleri.
    pub joints: [u16; 4],
    /// Normalize edilmiş ağırlık değerleri ($\sum w_i = 1.0$).
    pub weights: [f32; 4],
}

impl Default for JointWeights {
    fn default() -> Self {
        Self {
            joints: [0; 4],
            weights: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

impl JointWeights {
    /// Yeni bir ağırlık seti oluşturur ve otomatik olarak toplamını 1.0'a normalize eder.
    pub fn new(joints: [u16; 4], mut weights: [f32; 4]) -> Self {
        let sum = weights[0] + weights[1] + weights[2] + weights[3];
        if sum > 1e-6 {
            let inv_sum = 1.0 / sum;
            weights[0] *= inv_sum;
            weights[1] *= inv_sum;
            weights[2] *= inv_sum;
            weights[3] *= inv_sum;
        } else {
            weights = [1.0, 0.0, 0.0, 0.0];
        }

        Self { joints, weights }
    }

    /// Tek bir kemiğe katı bağlı (%100 ağırlıklı) köşe oluşturur.
    pub fn rigid(joint: u16) -> Self {
        Self {
            joints: [joint, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

/// Linear Blend Skinning (LBS) ile tek bir köşe pozisyonunu ve normal vektörünü dönüştürür.
///
/// Düşük donanımlı sistemlerde CPU hesaplaması veya yazılımsal önizleme için idealdir.
#[inline]
pub fn skin_vertex_lbs(
    position: Vec3,
    normal: Vec3,
    weights: &JointWeights,
    skinning_palette: &[Mat4],
) -> (Vec3, Vec3) {
    let mut skinned_pos = Vec3::ZERO;
    let mut skinned_norm = Vec3::ZERO;

    for i in 0..4 {
        let w = weights.weights[i];
        if w > 0.0 {
            let bone_idx = weights.joints[i] as usize;
            if let Some(matrix) = skinning_palette.get(bone_idx) {
                let transformed_p = matrix.transform_point3(position);
                let transformed_n = matrix.transform_vector3(normal);

                skinned_pos += transformed_p * w;
                skinned_norm += transformed_n * w;
            }
        }
    }

    let norm_len = skinned_norm.length();
    let final_norm = if norm_len > 1e-6 {
        skinned_norm / norm_len
    } else {
        normal
    };

    (skinned_pos, final_norm)
}

/// Dual Quaternion Skinning (DQS) temsili: Gerçek dönme kuaterniyonu ve sanal öteleme kuaterniyonu.
///
/// Eklem bükülmelerindeki hacim daralmasını (candy-wrapper artifact) tamamen ortadan kaldırır.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DualQuat {
    /// Gerçek bileşen (Rotation): $q_r$
    pub real: Quat,
    /// Sanal/Dual bileşen (Translation): $q_d = \frac{1}{2} \cdot t \cdot q_r$
    pub dual: Quat,
}

impl DualQuat {
    /// Bir dönüşüm matrisinden (Mat4) Dual Quaternion türetir.
    pub fn from_mat4(m: Mat4) -> Self {
        let (_scale, rotation, translation) = m.to_scale_rotation_translation();
        Self::from_rotation_translation(rotation, translation)
    }

    /// Rotasyon ve ötelemeden Dual Quaternion oluşturur.
    pub fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Self {
        let real = rotation.normalize();
        let t_quat = Quat::from_xyzw(translation.x, translation.y, translation.z, 0.0);
        let dual = (t_quat * real) * 0.5;

        Self { real, dual }
    }

    /// Bir noktayı Dual Quaternion ile dönüştürür.
    pub fn transform_point3(&self, point: Vec3) -> Vec3 {
        // Öteleme vektörü: 2 * (dual * real^-1)
        let t_quat = (self.dual * 2.0) * self.real.conjugate();
        let translation = Vec3::new(t_quat.x, t_quat.y, t_quat.z);
        self.real * point + translation
    }
}

/// Birden fazla Dual Quaternion'ı ağırlıklarla harmanlar (DLB - Dual Quaternion Linear Blending).
pub fn blend_dual_quats(dqs: &[DualQuat], weights: &[f32]) -> DualQuat {
    let mut blended_real = Vec4::ZERO;
    let mut blended_dual = Vec4::ZERO;

    let pivot_real = dqs[0].real;

    for (dq, &w) in dqs.iter().zip(weights.iter()) {
        if w > 0.0 {
            // Antipodal flip kontrolü: En yakın yolu seç
            let sign = if dq.real.dot(pivot_real) < 0.0 { -1.0 } else { 1.0 };
            let r_vec = Vec4::from(dq.real) * (w * sign);
            let d_vec = Vec4::from(dq.dual) * (w * sign);

            blended_real += r_vec;
            blended_dual += d_vec;
        }
    }

    let real_quat = Quat::from_vec4(blended_real).normalize();
    let dual_quat = Quat::from_vec4(blended_dual);

    DualQuat {
        real: real_quat,
        dual: dual_quat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joint_weights_normalization() {
        let weights = JointWeights::new([0, 1, 2, 3], [2.0, 2.0, 0.0, 0.0]);
        assert!((weights.weights[0] - 0.5).abs() < 1e-5);
        assert!((weights.weights[1] - 0.5).abs() < 1e-5);
        assert_eq!(weights.weights[2], 0.0);
        assert_eq!(weights.weights[3], 0.0);
    }

    #[test]
    fn test_skin_vertex_lbs() {
        let mat0 = Mat4::from_translation(Vec3::new(0.0, 10.0, 0.0));
        let mat1 = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let palette = vec![mat0, mat1];

        let weights = JointWeights::new([0, 1, 0, 0], [0.5, 0.5, 0.0, 0.0]);
        let original_pos = Vec3::ZERO;
        let original_norm = Vec3::Y;

        let (skinned_pos, skinned_norm) = skin_vertex_lbs(original_pos, original_norm, &weights, &palette);

        // Beklenen ortalama öteleme: (0, 10, 0)*0.5 + (10, 0, 0)*0.5 = (5, 5, 0)
        assert!((skinned_pos.x - 5.0).abs() < 1e-4);
        assert!((skinned_pos.y - 5.0).abs() < 1e-4);
        assert!((skinned_pos.z - 0.0).abs() < 1e-4);
        assert!((skinned_norm.y - 1.0).abs() < 1e-4);
    }
}

