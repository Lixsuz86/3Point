//! 3Point 3D Fizik Dünyası (`PhysicsWorld`).
//!
//! Rijit cisimlerin, çarpıştırıcıların, yerçekiminin ve raycast sorgularının yönetildiği ana motor.

use crate::{
    body::{RigidBodyDesc, RigidBodyHandle},
    collider::{ColliderDesc, ColliderHandle},
    math_conv::{isometry_to_transform, na_to_vec3, transform_to_isometry, vec3_to_na},
};
use glam::{Quat, Vec3};
use rapier3d::prelude as rapier;

/// Işın fırlatma (Raycast) temas sonucu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    /// Çarpan çarpıştırıcı.
    pub collider: ColliderHandle,
    /// Işın boyunca temas mesafesi ($TOI - Time Of Impact$).
    pub toi: f32,
    /// Temasın gerçekleştiği 3D dünya noktası.
    pub point: Vec3,
    /// Temas noktasındaki yüzey normali.
    pub normal: Vec3,
}

/// 3D fizik simülasyon dünyası.
pub struct PhysicsWorld {
    pub gravity: Vec3,
    pub pipeline: rapier::PhysicsPipeline,
    pub island_manager: rapier::IslandManager,
    pub broad_phase: rapier::DefaultBroadPhase,
    pub narrow_phase: rapier::NarrowPhase,
    pub bodies: rapier::RigidBodySet,
    pub colliders: rapier::ColliderSet,
    pub impulse_joints: rapier::ImpulseJointSet,
    pub multibody_joints: rapier::MultibodyJointSet,
    pub ccd_solver: rapier::CCDSolver,
    pub query_pipeline: rapier::QueryPipeline,
    pub integration_parameters: rapier::IntegrationParameters,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, -9.81, 0.0))
    }
}

impl PhysicsWorld {
    /// Belirtilen yerçekimi vektörü ile fizik dünyasını başlatır.
    pub fn new(gravity: Vec3) -> Self {
        Self {
            gravity,
            pipeline: rapier::PhysicsPipeline::new(),
            island_manager: rapier::IslandManager::new(),
            broad_phase: rapier::DefaultBroadPhase::new(),
            narrow_phase: rapier::NarrowPhase::new(),
            bodies: rapier::RigidBodySet::new(),
            colliders: rapier::ColliderSet::new(),
            impulse_joints: rapier::ImpulseJointSet::new(),
            multibody_joints: rapier::MultibodyJointSet::new(),
            ccd_solver: rapier::CCDSolver::new(),
            query_pipeline: rapier::QueryPipeline::new(),
            integration_parameters: rapier::IntegrationParameters::default(),
        }
    }

    /// Fizik dünyasını `dt` süresi kadar simüle eder.
    pub fn step(&mut self, dt: f32) {
        self.integration_parameters.dt = dt;

        let gravity_na = vec3_to_na(self.gravity);
        let physics_hooks = ();
        let event_handler = ();

        self.pipeline.step(
            &gravity_na,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &physics_hooks,
            &event_handler,
        );
    }

    /// Yeni bir rijit cisim ekler.
    pub fn add_body(&mut self, desc: RigidBodyDesc) -> RigidBodyHandle {
        let rb = desc.build();
        let handle = self.bodies.insert(rb);
        RigidBodyHandle(handle)
    }

    /// Bir rijit cismi ve bağlı çarpıştırıcılarını dünyadan kaldırır.
    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.bodies.remove(
            handle.0,
            &mut self.island_manager,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
    }

    /// Bir çarpıştırıcı ekler (opsiyonel olarak bir rijit cisme bağlar).
    pub fn add_collider(
        &mut self,
        desc: ColliderDesc,
        parent_body: Option<RigidBodyHandle>,
    ) -> ColliderHandle {
        let col = desc.build();
        let handle = if let Some(parent) = parent_body {
            self.colliders.insert_with_parent(col, parent.0, &mut self.bodies)
        } else {
            self.colliders.insert(col)
        };
        ColliderHandle(handle)
    }

    /// Çarpıştırıcıyı kaldırır.
    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        self.colliders
            .remove(handle.0, &mut self.island_manager, &mut self.bodies, true);
    }

    /// Bir rijit cismin güncel konum ve rotasyonunu döndürür.
    pub fn body_transform(&self, handle: RigidBodyHandle) -> Option<(Vec3, Quat)> {
        self.bodies.get(handle.0).map(|rb| isometry_to_transform(rb.position()))
    }

    /// Bir rijit cismin konum ve rotasyonunu zorla günceller (teleport).
    pub fn set_body_transform(&mut self, handle: RigidBodyHandle, translation: Vec3, rotation: Quat) {
        if let Some(rb) = self.bodies.get_mut(handle.0) {
            let iso = transform_to_isometry(translation, rotation);
            rb.set_position(iso, true);
        }
    }

    /// Cisme anlık itme impulsu uygular.
    pub fn apply_impulse(&mut self, handle: RigidBodyHandle, impulse: Vec3) {
        if let Some(rb) = self.bodies.get_mut(handle.0) {
            rb.apply_impulse(vec3_to_na(impulse), true);
        }
    }

    /// Cisme sürekli kuvvet uygular.
    pub fn apply_force(&mut self, handle: RigidBodyHandle, force: Vec3) {
        if let Some(rb) = self.bodies.get_mut(handle.0) {
            rb.add_force(vec3_to_na(force), true);
        }
    }

    /// Fizik dünyasında ışın fırlatarak (Raycast) ilk çarpan nesneyi bulur.
    pub fn cast_ray(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_toi: f32,
        solid: bool,
    ) -> Option<RayHit> {
        let ray = rapier::Ray::new(
            rapier3d::na::Point3::new(origin.x, origin.y, origin.z),
            vec3_to_na(direction.normalize()),
        );

        let filter = rapier::QueryFilter::default();

        self.query_pipeline
            .cast_ray_and_get_normal(&self.bodies, &self.colliders, &ray, max_toi, solid, filter)
            .map(|(handle, intersection)| {
                let hit_point = origin + direction.normalize() * intersection.time_of_impact;
                RayHit {
                    collider: ColliderHandle(handle),
                    toi: intersection.time_of_impact,
                    point: hit_point,
                    normal: na_to_vec3(intersection.normal),
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_falling_body_simulation() {
        let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        // 1. Zemin (Statik)
        let floor_body = world.add_body(RigidBodyDesc::fixed().translation(Vec3::new(0.0, 0.0, 0.0)));
        world.add_collider(ColliderDesc::cuboid(Vec3::new(10.0, 0.5, 10.0)), Some(floor_body));

        // 2. Düşen Kutu (Dinamik) Y = 5.0
        let box_body = world.add_body(RigidBodyDesc::dynamic().translation(Vec3::new(0.0, 5.0, 0.0)));
        world.add_collider(ColliderDesc::cuboid(Vec3::splat(0.5)), Some(box_body));

        // 60 kare simüle et (1 saniye)
        for _ in 0..60 {
            world.step(1.0 / 60.0);
        }

        // Kutu zemine (Y ~ 1.0 = zemin 0.5 + kutu 0.5) çarpmış ve durmuş olmalı
        let (pos, _) = world.body_transform(box_body).unwrap();
        assert!(pos.y < 5.0);
        assert!((pos.y - 1.0).abs() < 0.15);
    }

    #[test]
    fn test_raycast_hit() {
        let mut world = PhysicsWorld::new(Vec3::ZERO);

        let box_body = world.add_body(RigidBodyDesc::fixed().translation(Vec3::new(0.0, 0.0, 5.0)));
        world.add_collider(ColliderDesc::cuboid(Vec3::splat(1.0)), Some(box_body));

        // Query pipeline'ı güncelle
        world.step(0.01);

        // Z ekseninde + yönlü ışın fırlat
        let hit = world.cast_ray(Vec3::ZERO, Vec3::Z, 10.0, true);
        assert!(hit.is_some());

        let h = hit.unwrap();
        // Kutunun ön yüzü Z = 5.0 - 1.0 = 4.0
        assert!((h.toi - 4.0).abs() < 1e-3);
        assert!((h.point.z - 4.0).abs() < 1e-3);
    }
}
