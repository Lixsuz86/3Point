//! # `tp_anim`
//!
//! 3Point Karakter & Nesne Animasyon Sistemi.
//!
//! - **İskelet & Armatür (`Skeleton`, `Bone`):** $O(N)$ hızlı ileri kinematik (FK) ve topolojik düz dizilimli kemik hiyerarşisi.
//! - **Deri Kaplama (`skinning`):** Linear Blend Skinning (LBS) ve hacim koruyucu Dual Quaternion Skinning (DQS).
//! - **Animasyon Eğrileri (`curve`):** Step, Linear ve Hermite Cubic Spline interpolasyonu.
//! - **Klipler & Kanallar (`clip`):** Kemik bazlı öteleme, dönme (slerp) ve ölçekleme izleri.
//! - **Ters Kinematik (`ik`):** Karakter uzuvları için $O(1)$ mikro-saniyelik Two-Bone IK ve çoklu zincirler için FABRIK.
//! - **Karıştırma & Oynatıcı (`blend`, `player`):** Klipler arası yumuşak Cross-fade ve kemik maskeleme.

pub mod blend;
pub mod clip;
pub mod curve;
pub mod error;
pub mod ik;
pub mod player;
pub mod skeleton;
pub mod skinning;

pub use blend::{additive_blend, blend_poses, blend_poses_masked, blend_transforms};
pub use clip::{AnimationClip, BoneTrack};
pub use curve::{AnimationCurve, Interpolate, Interpolation, Keyframe};
pub use error::AnimError;
pub use ik::{FabrikChain, TwoBoneIk};
pub use player::AnimationPlayer;
pub use skeleton::{Bone, BoneId, Skeleton};
pub use skinning::{
    blend_dual_quats, skin_vertex_lbs, DualQuat, JointWeights, MAX_BONES,
};
