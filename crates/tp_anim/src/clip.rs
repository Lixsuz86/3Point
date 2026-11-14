//! 3Point Animasyon Klibi (`AnimationClip`) ve Kemik Kanalları (`BoneTrack`).

use crate::{
    curve::AnimationCurve,
    error::AnimError,
    skeleton::BoneId,
};
use tp_math::{Quat, Transform, Vec3};

/// Tek bir kemiğin zaman içindeki öteleme, dönme ve ölçekleme kanalları.
#[derive(Debug, Clone, PartialEq)]
pub struct BoneTrack {
    pub bone_id: BoneId,
    pub translation: Option<AnimationCurve<Vec3>>,
    pub rotation: Option<AnimationCurve<Quat>>,
    pub scale: Option<AnimationCurve<Vec3>>,
}

impl BoneTrack {
    /// Yeni bir kemik kanalı oluşturur.
    pub fn new(bone_id: BoneId) -> Self {
        Self {
            bone_id,
            translation: None,
            rotation: None,
            scale: None,
        }
    }

    /// Verilen zamanda dönüşüm bileşenlerini örnekler ve hedef Transform'a uygular.
    pub fn sample(&self, time: f32, out_transform: &mut Transform) {
        if let Some(ref trans_curve) = self.translation {
            if let Some(pos) = trans_curve.sample(time) {
                out_transform.translation = pos;
            }
        }
        if let Some(ref rot_curve) = self.rotation {
            if let Some(rot) = rot_curve.sample(time) {
                out_transform.rotation = rot;
            }
        }
        if let Some(ref scale_curve) = self.scale {
            if let Some(scale) = scale_curve.sample(time) {
                out_transform.scale = scale;
            }
        }
    }
}

/// Bağımsız bir animasyon klibi (Örn: "Walk", "Run", "Idle", "Attack").
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<BoneTrack>,
}

impl AnimationClip {
    /// Yeni bir animasyon klibi oluşturur.
    pub fn new(name: impl Into<String>, duration: f32) -> Result<Self, AnimError> {
        if duration <= 0.0 {
            return Err(AnimError::InvalidClipDuration(duration));
        }

        Ok(Self {
            name: name.into(),
            duration,
            tracks: Vec::new(),
        })
    }

    /// Bir kemik kanalı ekler.
    pub fn add_track(&mut self, track: BoneTrack) {
        // Eğer aynı kemik zaten varsa üzerine yaz veya güncelle
        if let Some(existing) = self.tracks.iter_mut().find(|t| t.bone_id == track.bone_id) {
            *existing = track;
        } else {
            self.tracks.push(track);
        }
    }

    /// Verilen zamanda klibi örnekler ve mevcut yerel poz tamponunu günceller.
    pub fn sample_into_pose(&self, time: f32, looping: bool, pose: &mut [Transform]) {
        let sample_time = if looping {
            time.rem_euclid(self.duration)
        } else {
            time.clamp(0.0, self.duration)
        };

        for track in &self.tracks {
            let idx = track.bone_id.0 as usize;
            if let Some(bone_transform) = pose.get_mut(idx) {
                track.sample(sample_time, bone_transform);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::{Interpolation, Keyframe};

    #[test]
    fn test_animation_clip_sampling() {
        let mut clip = AnimationClip::new("TestWalk", 2.0).unwrap();

        let mut track = BoneTrack::new(BoneId(0));
        let mut trans_curve = AnimationCurve::new(Interpolation::Linear);
        trans_curve.add_keyframe(Keyframe::new(0.0, Vec3::new(0.0, 0.0, 0.0)));
        trans_curve.add_keyframe(Keyframe::new(2.0, Vec3::new(0.0, 2.0, 0.0)));
        track.translation = Some(trans_curve);

        clip.add_track(track);

        let mut pose = vec![Transform::IDENTITY];
        clip.sample_into_pose(1.0, true, &mut pose);

        assert!((pose[0].translation.y - 1.0).abs() < 1e-4);

        // Looping testi (time = 3.0 -> rem_euclid(2.0) = 1.0)
        clip.sample_into_pose(3.0, true, &mut pose);
        assert!((pose[0].translation.y - 1.0).abs() < 1e-4);
    }
}

