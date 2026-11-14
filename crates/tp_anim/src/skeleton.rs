//! 3Point Kemik İskeleti ve Armatür Hiyerarşisi (`Skeleton`).

use crate::error::AnimError;
use tp_math::{Mat4, Transform};

/// İskelet içerisindeki tekil kemik kimliği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoneId(pub u16);

/// İskeletteki tek bir kemik düğümü.
#[derive(Debug, Clone, PartialEq)]
pub struct Bone {
    /// Kemiğin adı (Örn: "Hips", "Spine", "LeftArm").
    pub name: String,
    /// Ebeveyn kemik kimliği (`None` ise kök kemiktir).
    pub parent: Option<BoneId>,
    /// Referans (Bind Pose) duruşundaki yerel dönüşüm.
    pub local_bind_transform: Transform,
    /// Bind Pose'daki dünya matrisinin tersi ($M_{bind}^{-1}$).
    /// Köşe noktalarını model uzayından kemik uzayına taşır.
    pub inverse_bind_matrix: Mat4,
}

/// Düz (flat-array) tabanlı, yüksek performanslı kemik iskeleti.
///
/// **Önemli Kural:** Sıfır ek bellek maliyeti ve $O(N)$ ileri kinematik için,
/// ebeveyn kemikler her zaman çocuk kemiklerden daha küçük bir indekste yer alır (Topological Ordering).
#[derive(Debug, Clone, Default)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

impl Skeleton {
    /// Boş bir iskelet oluşturur.
    pub fn new() -> Self {
        Self { bones: Vec::new() }
    }

    /// İskelete yeni bir kemik ekler.
    ///
    /// Ebeveyn kemik mevcut olmalı ve indeksi yeni kemikten küçük olmalıdır.
    pub fn add_bone(
        &mut self,
        name: impl Into<String>,
        parent: Option<BoneId>,
        local_bind_transform: Transform,
    ) -> Result<BoneId, AnimError> {
        let new_id = BoneId(self.bones.len() as u16);

        if let Some(p) = parent {
            if p.0 >= new_id.0 {
                return Err(AnimError::NonTopologicalOrder {
                    parent: p.0,
                    child: new_id.0,
                });
            }
        }

        self.bones.push(Bone {
            name: name.into(),
            parent,
            local_bind_transform,
            inverse_bind_matrix: Mat4::IDENTITY,
        });

        // Inverse bind matrisini anında güncelle
        self.update_inverse_bind_matrix(new_id)?;

        Ok(new_id)
    }

    /// Kemik sayısını döndürür.
    #[inline]
    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    /// Kemik adına göre arama yapar.
    pub fn find_bone_by_name(&self, name: &str) -> Option<BoneId> {
        self.bones
            .iter()
            .position(|b| b.name == name)
            .map(|idx| BoneId(idx as u16))
    }

    /// İskeletin varsayılan referans (Bind Pose) yerel dönüşümlerini döndürür.
    pub fn default_bind_pose(&self) -> Vec<Transform> {
        self.bones.iter().map(|b| b.local_bind_transform).collect()
    }

    /// Tek bir kemiğin ters bağlama matrisini (`inverse_bind_matrix`) hesaplar.
    fn update_inverse_bind_matrix(&mut self, bone_id: BoneId) -> Result<(), AnimError> {
        let idx = bone_id.0 as usize;
        if idx >= self.bones.len() {
            return Err(AnimError::InvalidBoneId(bone_id.0));
        }

        // Kökten bu kemiğe kadar olan dünya matrisini hesapla
        let mut world_mat = self.bones[idx].local_bind_transform.to_matrix();
        let mut curr_parent = self.bones[idx].parent;

        while let Some(p_id) = curr_parent {
            let p_idx = p_id.0 as usize;
            let parent_local = self.bones[p_idx].local_bind_transform.to_matrix();
            world_mat = parent_local * world_mat;
            curr_parent = self.bones[p_idx].parent;
        }

        self.bones[idx].inverse_bind_matrix = world_mat.inverse();
        Ok(())
    }

    /// Tüm iskeletin ters bağlama matrislerini yeniden hesaplar.
    pub fn recompute_all_inverse_bind_matrices(&mut self) -> Result<(), AnimError> {
        let count = self.bones.len();
        let mut world_bind_matrices = vec![Mat4::IDENTITY; count];

        for i in 0..count {
            let local_mat = self.bones[i].local_bind_transform.to_matrix();
            if let Some(parent_id) = self.bones[i].parent {
                let parent_idx = parent_id.0 as usize;
                world_bind_matrices[i] = world_bind_matrices[parent_idx] * local_mat;
            } else {
                world_bind_matrices[i] = local_mat;
            }
            self.bones[i].inverse_bind_matrix = world_bind_matrices[i].inverse();
        }

        Ok(())
    }

    /// Yerel poz dönüşümlerinden $O(N)$ hızında dünya dönüşüm matrislerini üretir (Forward Kinematics).
    ///
    /// Sıfır heap ayırması ile verilen `out_world` tamponunu doldurur.
    pub fn compute_world_transforms(
        &self,
        local_poses: &[Transform],
        out_world: &mut [Mat4],
    ) -> Result<(), AnimError> {
        if local_poses.len() != self.bones.len() || out_world.len() != self.bones.len() {
            return Err(AnimError::PoseBufferMismatch {
                expected: self.bones.len(),
                actual: local_poses.len(),
            });
        }

        for i in 0..self.bones.len() {
            let local_mat = local_poses[i].to_matrix();
            if let Some(parent_id) = self.bones[i].parent {
                let parent_idx = parent_id.0 as usize;
                out_world[i] = out_world[parent_idx] * local_mat;
            } else {
                out_world[i] = local_mat;
            }
        }

        Ok(())
    }

    /// GPU veya CPU Skinning için nihai Skinning Matris Paletini hesaplar.
    ///
    /// Formül: $M_{skin} = M_{world} \cdot M_{bind}^{-1}$
    pub fn compute_skinning_matrices(
        &self,
        world_transforms: &[Mat4],
        out_skinning: &mut [Mat4],
    ) -> Result<(), AnimError> {
        if world_transforms.len() != self.bones.len() || out_skinning.len() != self.bones.len() {
            return Err(AnimError::PoseBufferMismatch {
                expected: self.bones.len(),
                actual: world_transforms.len(),
            });
        }

        for i in 0..self.bones.len() {
            out_skinning[i] = world_transforms[i] * self.bones[i].inverse_bind_matrix;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tp_math::Vec3;

    #[test]
    fn test_skeleton_creation_and_fk() {
        let mut skel = Skeleton::new();

        // 1. Kök Kemik (Pelvis)
        let root = skel
            .add_bone("Pelvis", None, Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)))
            .unwrap();

        // 2. Omurga Kemiği (Spine)
        let spine = skel
            .add_bone("Spine", Some(root), Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)))
            .unwrap();

        // 3. Baş Kemiği (Head)
        let head = skel
            .add_bone("Head", Some(spine), Transform::from_translation(Vec3::new(0.0, 0.4, 0.0)))
            .unwrap();

        assert_eq!(skel.bone_count(), 3);
        assert_eq!(skel.find_bone_by_name("Spine"), Some(spine));

        // Bind Pose üzerinden dünya matrislerini hesapla
        let bind_pose = skel.default_bind_pose();
        let mut world_mats = vec![Mat4::IDENTITY; 3];
        skel.compute_world_transforms(&bind_pose, &mut world_mats).unwrap();

        // Head kemiğinin dünya pozisyonu: 1.0 + 0.5 + 0.4 = 1.9
        let head_world_pos = world_mats[head.0 as usize].transform_point3(Vec3::ZERO);
        assert!((head_world_pos.y - 1.9).abs() < 1e-4);

        // Bind Pose'da Skinning Matrisleri birim matris (IDENTITY) olmalıdır!
        let mut skin_mats = vec![Mat4::IDENTITY; 3];
        skel.compute_skinning_matrices(&world_mats, &mut skin_mats).unwrap();

        for mat in skin_mats {
            assert!((mat - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 1e-4));
        }
    }
}

