//! 3Point Animasyon Oynatıcısı (`AnimationPlayer`) ve Klipler Arası Yumuşak Geçiş (Cross-Fade).

use crate::{
    blend::blend_poses,
    clip::AnimationClip,
    skeleton::Skeleton,
};
use tp_math::Transform;

/// İki klip arasındaki geçiş durumu (Cross-fade transition).
#[derive(Debug, Clone)]
struct Transition {
    from_clip: AnimationClip,
    from_time: f32,
    from_looping: bool,
    duration: f32,
    elapsed: f32,
}

/// İskelet animasyonlarını zaman içinde ilerleten, döngüye sokan ve klipler arası geçişi yöneten oynatıcı.
#[derive(Debug, Clone)]
pub struct AnimationPlayer {
    /// Aktif oynatılan ana animasyon klibi.
    pub current_clip: Option<AnimationClip>,
    /// O anki zaman damgası (saniye).
    pub current_time: f32,
    /// Oynatma hızı (1.0 = normal, 0.5 = yarım hız, 2.0 = çift hız, -1.0 = geri).
    pub speed: f32,
    /// Oynatma durumu (aktif mi duraklatıldı mı).
    pub is_playing: bool,
    /// Animasyon bittiğinde başa dönüp dönmeyeceği.
    pub looping: bool,
    /// Yumuşak klip geçişi verisi.
    transition: Option<Transition>,
    /// Çapraz geçiş hesaplaması için geçici tampon.
    temp_pose: Vec<Transform>,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationPlayer {
    /// Yeni bir animasyon oynatıcısı oluşturur.
    pub fn new() -> Self {
        Self {
            current_clip: None,
            current_time: 0.0,
            speed: 1.0,
            is_playing: true,
            looping: true,
            transition: None,
            temp_pose: Vec::new(),
        }
    }

    /// Yeni bir klip oynatmaya başlar (anında geçiş).
    pub fn play(&mut self, clip: AnimationClip, looping: bool) {
        self.current_clip = Some(clip);
        self.current_time = 0.0;
        self.looping = looping;
        self.is_playing = true;
        self.transition = None;
    }

    /// Başka bir animasyon klibine belirlenen sürede yumuşak geçiş yapar (Cross-fade).
    pub fn cross_fade(&mut self, next_clip: AnimationClip, duration: f32, looping: bool) {
        if let Some(current) = self.current_clip.take() {
            if duration > 1e-4 {
                self.transition = Some(Transition {
                    from_clip: current,
                    from_time: self.current_time,
                    from_looping: self.looping,
                    duration,
                    elapsed: 0.0,
                });
            }
        }

        self.current_clip = Some(next_clip);
        self.current_time = 0.0;
        self.looping = looping;
        self.is_playing = true;
    }

    /// Oynatmayı duraklatır.
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Duraklatılmış oynatmayı devam ettirir.
    pub fn resume(&mut self) {
        self.is_playing = true;
    }

    /// Oynatmayı durdurur ve başa sarar.
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_time = 0.0;
        self.transition = None;
    }

    /// Oynatıcıyı `dt` süresi kadar ilerletir ve çıktıyı `out_pose` tamponuna yazar.
    pub fn update(&mut self, dt: f32, skeleton: &Skeleton, out_pose: &mut [Transform]) {
        // İskelet kemik sayısı kadar başlangıç pozu (Bind pose) ile doldur
        if out_pose.len() != skeleton.bone_count() {
            return;
        }

        for (i, bone) in skeleton.bones.iter().enumerate() {
            out_pose[i] = bone.local_bind_transform;
        }

        let Some(ref current_clip) = self.current_clip else {
            return;
        };

        if self.is_playing {
            self.current_time += dt * self.speed;
        }

        // Aktif klibi pozla
        current_clip.sample_into_pose(self.current_time, self.looping, out_pose);

        // Çapraz geçiş aktifse önceki klibi örnekleyip harmanla
        if let Some(ref mut trans) = self.transition {
            if self.temp_pose.len() != skeleton.bone_count() {
                self.temp_pose = vec![Transform::IDENTITY; skeleton.bone_count()];
            }

            for (i, bone) in skeleton.bones.iter().enumerate() {
                self.temp_pose[i] = bone.local_bind_transform;
            }

            if self.is_playing {
                trans.elapsed += dt;
                trans.from_time += dt * self.speed;
            }

            trans
                .from_clip
                .sample_into_pose(trans.from_time, trans.from_looping, &mut self.temp_pose);

            let weight = (trans.elapsed / trans.duration).clamp(0.0, 1.0);

            // temp_pose (from) ile out_pose (to) arasında ağırlıklı geçiş yap
            let mut blended = vec![Transform::IDENTITY; skeleton.bone_count()];
            blend_poses(&self.temp_pose, out_pose, weight, &mut blended);
            out_pose.copy_from_slice(&blended);

            if trans.elapsed >= trans.duration {
                self.transition = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::{AnimationCurve, Interpolation, Keyframe};
    use crate::skeleton::BoneId;
    use tp_math::Vec3;

    #[test]
    fn test_animation_player_playback_and_crossfade() {
        let mut skel = Skeleton::new();
        skel.add_bone("Root", None, Transform::IDENTITY).unwrap();

        // 1. Klip: Y pozisyonu 0.0 -> 10.0
        let mut clip1 = AnimationClip::new("Clip1", 2.0).unwrap();
        let mut track1 = crate::clip::BoneTrack::new(BoneId(0));
        let mut c1 = AnimationCurve::new(Interpolation::Linear);
        c1.add_keyframe(Keyframe::new(0.0, Vec3::new(0.0, 0.0, 0.0)));
        c1.add_keyframe(Keyframe::new(2.0, Vec3::new(0.0, 10.0, 0.0)));
        track1.translation = Some(c1);
        clip1.add_track(track1);

        // 2. Klip: Y pozisyonu 20.0 -> 30.0
        let mut clip2 = AnimationClip::new("Clip2", 2.0).unwrap();
        let mut track2 = crate::clip::BoneTrack::new(BoneId(0));
        let mut c2 = AnimationCurve::new(Interpolation::Linear);
        c2.add_keyframe(Keyframe::new(0.0, Vec3::new(0.0, 20.0, 0.0)));
        c2.add_keyframe(Keyframe::new(2.0, Vec3::new(0.0, 30.0, 0.0)));
        track2.translation = Some(c2);
        clip2.add_track(track2);

        let mut player = AnimationPlayer::new();
        let mut pose = vec![Transform::IDENTITY];

        player.play(clip1, true);
        player.update(1.0, &skel, &mut pose);
        // 1 saniye sonra Clip1 değeri: Y = 5.0
        assert!((pose[0].translation.y - 5.0).abs() < 1e-4);

        // Clip2'ye 1 saniyelik geçiş başlat
        player.cross_fade(clip2, 1.0, true);
        // 0.5 saniye sonra geçiş tam ortasında:
        // Clip1 (from_time 1.5 sn -> Y = 7.5)
        // Clip2 (current_time 0.5 sn -> Y = 22.5)
        // Blend (0.5 ağırlık): (7.5 + 22.5) / 2 = 15.0
        player.update(0.5, &skel, &mut pose);
        assert!((pose[0].translation.y - 15.0).abs() < 1e-3);
    }
}

