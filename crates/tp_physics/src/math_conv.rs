//! `glam` ile `rapier3d` (nalgebra) matematik türleri arasında sıfır maliyetli dönüştürücüler.

use glam::{Quat, Vec3};
use rapier3d::na::{Isometry3, Quaternion, Translation3, UnitQuaternion, Vector3};

/// `glam::Vec3` -> `rapier3d::na::Vector3<f32>`
#[inline]
pub fn vec3_to_na(v: Vec3) -> Vector3<f32> {
    Vector3::new(v.x, v.y, v.z)
}

/// `rapier3d::na::Vector3<f32>` -> `glam::Vec3`
#[inline]
pub fn na_to_vec3(v: Vector3<f32>) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

/// `glam::Quat` -> `rapier3d::na::UnitQuaternion<f32>`
#[inline]
pub fn quat_to_na(q: Quat) -> UnitQuaternion<f32> {
    let quat = Quaternion::new(q.w, q.x, q.y, q.z);
    UnitQuaternion::new_normalize(quat)
}

/// `rapier3d::na::UnitQuaternion<f32>` -> `glam::Quat`
#[inline]
pub fn na_to_quat(q: UnitQuaternion<f32>) -> Quat {
    Quat::from_xyzw(q.i, q.j, q.k, q.w)
}

/// `(glam::Vec3, glam::Quat)` -> `rapier3d::na::Isometry3<f32>`
#[inline]
pub fn transform_to_isometry(translation: Vec3, rotation: Quat) -> Isometry3<f32> {
    let tra = Translation3::new(translation.x, translation.y, translation.z);
    let rot = quat_to_na(rotation);
    Isometry3::from_parts(tra, rot)
}

/// `rapier3d::na::Isometry3<f32>` -> `(glam::Vec3, glam::Quat)`
#[inline]
pub fn isometry_to_transform(iso: &Isometry3<f32>) -> (Vec3, Quat) {
    let t = Vec3::new(iso.translation.x, iso.translation.y, iso.translation.z);
    let q = na_to_quat(iso.rotation);
    (t, q)
}
