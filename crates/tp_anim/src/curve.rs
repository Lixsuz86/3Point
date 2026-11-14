//! 3Point Animasyon Eğrileri ve Anahtar Kare İnterpolasyonu (`Curve`).

use tp_math::{Quat, Vec3};

/// Eğri interpolasyon yöntemleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Basamaklı interpolasyon: Değer bir sonraki anahtar kareye kadar sabit kalır.
    Step,
    /// Doğrusal interpolasyon (Linear Lerp / Slerp).
    #[default]
    Linear,
    /// Pürüzsüz kübik eğri (Hermite / Bezier cubic spline).
    CubicSpline,
}

/// Zamana bağlı tek bir anahtar kare (Keyframe).
#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe<T> {
    /// Anahtar karenin zaman damgası (saniye cinsinden).
    pub time: f32,
    /// Bu karedeki değer.
    pub value: T,
    /// Giriş teğeti (CubicSpline için opsiyonel).
    pub in_tangent: Option<T>,
    /// Çıkış teğeti (CubicSpline için opsiyonel).
    pub out_tangent: Option<T>,
}

impl<T> Keyframe<T> {
    /// Basit bir anahtar kare oluşturur.
    pub fn new(time: f32, value: T) -> Self {
        Self {
            time,
            value,
            in_tangent: None,
            out_tangent: None,
        }
    }

    /// Teğetli kübik anahtar kare oluşturur.
    pub fn with_tangents(time: f32, value: T, in_tangent: T, out_tangent: T) -> Self {
        Self {
            time,
            value,
            in_tangent: Some(in_tangent),
            out_tangent: Some(out_tangent),
        }
    }
}

/// Değerler arasında interpolasyon sağlayan yetenek.
pub trait Interpolate: Clone {
    fn lerp_val(&self, other: &Self, t: f32) -> Self;
    fn cubic_hermite(&self, other: &Self, out_tan: &Self, in_tan: &Self, t: f32, dt: f32) -> Self;
}

impl Interpolate for f32 {
    #[inline]
    fn lerp_val(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t
    }

    #[inline]
    fn cubic_hermite(&self, other: &Self, out_tan: &Self, in_tan: &Self, t: f32, dt: f32) -> Self {
        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        h00 * self + h10 * dt * out_tan + h01 * other + h11 * dt * in_tan
    }
}

impl Interpolate for Vec3 {
    #[inline]
    fn lerp_val(&self, other: &Self, t: f32) -> Self {
        self.lerp(*other, t)
    }

    #[inline]
    fn cubic_hermite(&self, other: &Self, out_tan: &Self, in_tan: &Self, t: f32, dt: f32) -> Self {
        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        *self * h00 + *out_tan * (dt * h10) + *other * h01 + *in_tan * (dt * h11)
    }
}

impl Interpolate for Quat {
    #[inline]
    fn lerp_val(&self, other: &Self, t: f32) -> Self {
        self.slerp(*other, t)
    }

    #[inline]
    fn cubic_hermite(&self, other: &Self, _out_tan: &Self, _in_tan: &Self, t: f32, _dt: f32) -> Self {
        // Kuaterniyonlarda kübik slerp yaklaşımı
        self.slerp(*other, t)
    }
}

/// Sıralı anahtar karelerden oluşan animasyon kanalı/eğrisi.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationCurve<T> {
    pub keyframes: Vec<Keyframe<T>>,
    pub interpolation: Interpolation,
}

impl<T: Interpolate> AnimationCurve<T> {
    /// Boş eğri oluşturur.
    pub fn new(interpolation: Interpolation) -> Self {
        Self {
            keyframes: Vec::new(),
            interpolation,
        }
    }

    /// Anahtar kare ekler (zaman sırasını korur).
    pub fn add_keyframe(&mut self, keyframe: Keyframe<T>) {
        let idx = self
            .keyframes
            .partition_point(|k| k.time < keyframe.time);
        self.keyframes.insert(idx, keyframe);
    }

    /// Belirtilen zaman damgasındaki değeri $O(\log N)$ sürede hesaplar.
    pub fn sample(&self, time: f32) -> Option<T> {
        if self.keyframes.is_empty() {
            return None;
        }

        if self.keyframes.len() == 1 || time <= self.keyframes[0].time {
            return Some(self.keyframes[0].value.clone());
        }

        let last_idx = self.keyframes.len() - 1;
        if time >= self.keyframes[last_idx].time {
            return Some(self.keyframes[last_idx].value.clone());
        }

        // İkili arama (Binary search) ile zaman dilimini bul
        let next_idx = self.keyframes.partition_point(|k| k.time <= time);
        let prev_idx = next_idx - 1;

        let prev = &self.keyframes[prev_idx];
        let next = &self.keyframes[next_idx];

        let dt = next.time - prev.time;
        if dt <= 1e-6 {
            return Some(prev.value.clone());
        }

        let t = ((time - prev.time) / dt).clamp(0.0, 1.0);

        match self.interpolation {
            Interpolation::Step => Some(prev.value.clone()),
            Interpolation::Linear => Some(prev.value.lerp_val(&next.value, t)),
            Interpolation::CubicSpline => {
                let default_tan = prev.value.clone();
                let out_tan = prev.out_tangent.as_ref().unwrap_or(&default_tan);
                let in_tan = next.in_tangent.as_ref().unwrap_or(&default_tan);
                Some(prev.value.cubic_hermite(&next.value, out_tan, in_tan, t, dt))
            }
        }
    }

    /// Eğrinin toplam süresini döndürür.
    pub fn duration(&self) -> f32 {
        self.keyframes.last().map(|k| k.time).unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_linear_curve_sampling() {
        let mut curve = AnimationCurve::new(Interpolation::Linear);
        curve.add_keyframe(Keyframe::new(0.0, Vec3::new(0.0, 0.0, 0.0)));
        curve.add_keyframe(Keyframe::new(2.0, Vec3::new(10.0, 20.0, 30.0)));

        assert_eq!(curve.sample(0.0), Some(Vec3::new(0.0, 0.0, 0.0)));
        assert_eq!(curve.sample(1.0), Some(Vec3::new(5.0, 10.0, 15.0)));
        assert_eq!(curve.sample(2.0), Some(Vec3::new(10.0, 20.0, 30.0)));
        assert_eq!(curve.sample(3.0), Some(Vec3::new(10.0, 20.0, 30.0))); // Clamp end
    }

    #[test]
    fn test_quat_slerp_curve() {
        let mut curve = AnimationCurve::new(Interpolation::Linear);
        curve.add_keyframe(Keyframe::new(0.0, Quat::IDENTITY));
        curve.add_keyframe(Keyframe::new(1.0, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)));

        let mid = curve.sample(0.5).unwrap();
        let expected_mid = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
        assert!((mid.dot(expected_mid).abs() - 1.0).abs() < 1e-4);
    }
}

