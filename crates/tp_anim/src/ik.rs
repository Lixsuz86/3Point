//! 3Point Ters Kinematik (Inverse Kinematics - IK) Çözücüleri.
//!
//! İki kemikli analitik çözücü (Two-Bone IK) ve çok eklemli zincirler için FABRIK çözücüsü içerir.

use crate::error::AnimError;
use tp_math::Vec3;

/// İki kemikli analitik (Closed-form) IK çözücüsü.
///
/// Karakter kolları ve bacakları için $O(1)$ sürede sıfır döngü/iterasyonla kesin çözüm üretir.
/// Düşük donanımlı işlemcilerde (Intel Celeron / i3) dahi mikro-saniyeler içinde çalışır.
pub struct TwoBoneIk;

impl TwoBoneIk {
    /// İki kemikli zinciri hedefe (`target`) ve kutup vektörüne (`pole_vector`) göre çözer.
    ///
    /// # Dönüş Değeri
    /// Başarılı olursa `(yeni_orta_eklem_pozisyonu, yeni_uç_eklem_pozisyonu)` döner.
    pub fn solve(
        root: Vec3,
        mid: Vec3,
        tip: Vec3,
        target: Vec3,
        pole_vector: Vec3,
    ) -> Result<(Vec3, Vec3), AnimError> {
        let l1 = (mid - root).length();
        let l2 = (tip - mid).length();

        if l1 <= 1e-5 || l2 <= 1e-5 {
            return Err(AnimError::IkSolverFailed("Kemik uzunlukları sıfıra çok yakın".into()));
        }

        let to_target = target - root;
        let d = to_target.length();

        if d <= 1e-5 {
            return Err(AnimError::IkSolverFailed("Hedef kök ile çakışık".into()));
        }

        let dir_target = to_target / d;

        // Hedef zincirin erişebileceğinden daha uzaksa: Kolu/bacağı tamamen hedefe doğru uzat
        if d >= l1 + l2 - 1e-4 {
            let solved_mid = root + dir_target * l1;
            let solved_tip = root + dir_target * (l1 + l2);
            return Ok((solved_mid, solved_tip));
        }

        // Hedef kemik farkından daha yakınsa: Minimum katlanma
        let min_dist = (l1 - l2).abs();
        let effective_d = d.max(min_dist + 1e-4);

        // Kosinüs Teoremi ile alfa açısını (kök eklemindeki açı) hesapla
        let cos_alpha = ((l1 * l1 + effective_d * effective_d - l2 * l2) / (2.0 * l1 * effective_d)).clamp(-1.0, 1.0);
        let sin_alpha = (1.0 - cos_alpha * cos_alpha).sqrt();

        // Bükülme düzlemini belirle (Hedef yönü ve Kutup Vektörü çapraz çarpımı)
        let mut bend_normal = dir_target.cross(pole_vector - root);
        if bend_normal.length_squared() < 1e-5 {
            // Kutup vektörü hedef doğrultusundaysa alternatif eksen seç
            let alternative = if dir_target.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
            bend_normal = dir_target.cross(alternative);
        }
        let bend_normal = bend_normal.normalize();

        // Düzlem içi dik yön
        let bend_dir = bend_normal.cross(dir_target).normalize();

        // Orta eklemin çözülmüş konumu
        let solved_mid = root + dir_target * (l1 * cos_alpha) + bend_dir * (l1 * sin_alpha);

        // Uç eklem pozisyonu (hedef doğrultusunda l2 boyunda)
        let mid_to_target = (target - solved_mid).normalize();
        let solved_tip = solved_mid + mid_to_target * l2;

        Ok((solved_mid, solved_tip))
    }
}

/// FABRIK (Forward And Backward Reaching Inverse Kinematics) Çözücüsü.
///
/// Çok eklemli omurga, kuyruk ve dokunaçlar için matris açı tekillikleri (Gimbal lock)
/// olmadan doğrudan nokta pozisyonları üzerinden iteratif çalışan çözücü.
pub struct FabrikChain {
    /// Zincirdeki eklem pozisyonları (Kökten uca doğru).
    pub points: Vec<Vec3>,
    /// Eklemler arasındaki sabit kemik uzunlukları.
    pub lengths: Vec<f32>,
}

impl FabrikChain {
    /// Verilen eklem noktaları ile FABRIK zinciri başlatır.
    pub fn new(points: Vec<Vec3>) -> Result<Self, AnimError> {
        if points.len() < 2 {
            return Err(AnimError::IkSolverFailed("Zincir en az 2 nokta içermelidir".into()));
        }

        let mut lengths = Vec::with_capacity(points.len() - 1);
        for i in 0..points.len() - 1 {
            let len = (points[i + 1] - points[i]).length();
            lengths.push(len);
        }

        Ok(Self { points, lengths })
    }

    /// Toplam zincir uzunluğunu hesaplar.
    pub fn total_length(&self) -> f32 {
        self.lengths.iter().sum()
    }

    /// Zinciri hedefe doğru iteratif olarak çözer.
    pub fn solve(&mut self, target: Vec3, tolerance: f32, max_iterations: usize) {
        let n = self.points.len();
        let root = self.points[0];
        let total_len = self.total_length();

        // Hedef ulaşılamaz uzaklıktaysa: Tamamen hedefe doğru düzleştir
        let dist_to_target = (target - root).length();
        if dist_to_target >= total_len {
            let dir = (target - root) / dist_to_target;
            for i in 0..n - 1 {
                self.points[i + 1] = self.points[i] + dir * self.lengths[i];
            }
            return;
        }

        // İteratif FABRIK döngüsü
        for _ in 0..max_iterations {
            let tip_err = (self.points[n - 1] - target).length();
            if tip_err <= tolerance {
                break;
            }

            // AŞAMA 1: Geriye Doğru Geçiş (Backward Reaching: Tip -> Root)
            self.points[n - 1] = target;
            for i in (0..n - 1).rev() {
                let dir = (self.points[i] - self.points[i + 1]).normalize();
                self.points[i] = self.points[i + 1] + dir * self.lengths[i];
            }

            // AŞAMA 2: İleriye Doğru Geçiş (Forward Reaching: Root -> Tip)
            self.points[0] = root;
            for i in 0..n - 1 {
                let dir = (self.points[i + 1] - self.points[i]).normalize();
                self.points[i + 1] = self.points[i] + dir * self.lengths[i];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_bone_ik_stretch() {
        let root = Vec3::ZERO;
        let mid = Vec3::new(0.0, 1.0, 0.0);
        let tip = Vec3::new(0.0, 2.0, 0.0);

        // Hedef 3 birim uzakta (zincir boyu 2 birim) -> Düz uzanmalı
        let target = Vec3::new(0.0, 3.0, 0.0);
        let pole = Vec3::new(1.0, 1.0, 0.0);

        let (solved_mid, solved_tip) = TwoBoneIk::solve(root, mid, tip, target, pole).unwrap();

        assert!((solved_mid.y - 1.0).abs() < 1e-4);
        assert!((solved_tip.y - 2.0).abs() < 1e-4);
    }

    #[test]
    fn test_two_bone_ik_bend() {
        let root = Vec3::ZERO;
        let mid = Vec3::new(0.0, 1.0, 0.0);
        let tip = Vec3::new(0.0, 2.0, 0.0);

        // Hedef 90 derece bükülmüş bir noktada
        let target = Vec3::new(1.414, 0.0, 0.0);
        let pole = Vec3::new(0.0, 1.0, 0.0);

        let (solved_mid, solved_tip) = TwoBoneIk::solve(root, mid, tip, target, pole).unwrap();

        let l1 = (solved_mid - root).length();
        let l2 = (solved_tip - solved_mid).length();

        assert!((l1 - 1.0).abs() < 1e-3);
        assert!((l2 - 1.0).abs() < 1e-3);
        assert!((solved_tip - target).length() < 0.05);
    }

    #[test]
    fn test_fabrik_chain() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
        ];

        let mut chain = FabrikChain::new(points).unwrap();
        assert!((chain.total_length() - 3.0).abs() < 1e-4);

        let target = Vec3::new(1.5, 1.5, 0.0);
        chain.solve(target, 0.01, 15);

        let tip = chain.points.last().unwrap();
        assert!((*tip - target).length() < 0.05);
        assert_eq!(chain.points[0], Vec3::ZERO); // Kök sabit kalmalı
    }
}

