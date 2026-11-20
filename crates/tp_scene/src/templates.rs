//! 3Point Başlangıç Sahne Şablonları (Templates).
//!
//! - **Empty:** Zemin ızgarası, kamera ve yönlü güneş ışığı.
//! - **Character:** 3 kemikli iskelet armatürü ve animasyon test alanı.
//! - **PhysicsSandbox:** Zemin çarpıştırıcısı ve düşen, yuvarlanan fizik nesneleri.

use crate::{
    components::{CameraComponent, LightComponent, LightType, PhysicsBodyComponent},
    scene::Scene,
};
use glam::{Vec3, Vec4};
use tp_math::Transform;
use tp_physics::{ColliderDesc, RigidBodyDesc};

/// 1. Temel Boş Sahne (Empty Scene).
pub fn create_empty_scene() -> Scene {
    let mut scene = Scene::new();

    scene.spawn_camera(
        "Main Camera",
        Transform::from_translation(Vec3::new(0.0, 2.0, 6.0)),
        CameraComponent::default(),
    );

    scene.spawn_light(
        "Directional Light",
        Transform::from_translation(Vec3::new(5.0, 10.0, 5.0)),
        LightComponent {
            kind: LightType::Directional,
            color: Vec3::ONE,
            intensity: 1.0,
            range: 50.0,
        },
    );

    scene
}

/// 2. Karakter & Animasyon Şablonu (Character Studio Scene).
pub fn create_character_scene() -> Scene {
    let mut scene = create_empty_scene();

    scene.spawn_mesh(
        "Character Torso",
        Transform::from_translation(Vec3::new(0.0, 1.5, 0.0)),
        "Cube",
        Vec4::new(0.3, 0.6, 0.9, 1.0),
    );

    scene.spawn_mesh(
        "Character Head",
        Transform::from_translation(Vec3::new(0.0, 2.5, 0.0)),
        "Sphere",
        Vec4::new(0.9, 0.8, 0.7, 1.0),
    );

    scene
}

/// 3. Fizik Test Alanı (Physics Sandbox Scene).
pub fn create_physics_sandbox() -> Scene {
    let mut scene = create_empty_scene();

    // Statik Zemin (Static Floor)
    let floor_entity = scene.spawn_mesh(
        "Static Floor",
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        "Plane",
        Vec4::new(0.4, 0.45, 0.5, 1.0),
    );
    let floor_rb = scene.physics.add_body(
        RigidBodyDesc::fixed().translation(Vec3::new(0.0, 0.0, 0.0)),
    );
    scene.physics.add_collider(
        ColliderDesc::cuboid(Vec3::new(10.0, 0.2, 10.0)).friction(0.6),
        Some(floor_rb),
    );
    scene.world.insert_one(floor_entity, PhysicsBodyComponent(floor_rb)).ok();

    // Dinamik Düşen Kutular & Küreler
    for i in 0..4 {
        let y_pos = 3.0 + (i as f32) * 1.5;
        let is_sphere = i % 2 == 1;

        let name = if is_sphere { format!("Dynamic Sphere {i}") } else { format!("Dynamic Box {i}") };
        let prim = if is_sphere { "Sphere" } else { "Cube" };
        let color = if is_sphere { Vec4::new(0.9, 0.3, 0.3, 1.0) } else { Vec4::new(0.3, 0.8, 0.4, 1.0) };

        let entity = scene.spawn_mesh(
            name,
            Transform::from_translation(Vec3::new(0.0, y_pos, 0.0)),
            prim,
            color,
        );

        let rb = scene.physics.add_body(
            RigidBodyDesc::dynamic().translation(Vec3::new(0.0, y_pos, 0.0)),
        );

        let col_desc = if is_sphere {
            ColliderDesc::sphere(0.5).restitution(0.3)
        } else {
            ColliderDesc::cuboid(Vec3::splat(0.5)).restitution(0.1)
        };

        scene.physics.add_collider(col_desc, Some(rb));
        scene.world.insert_one(entity, PhysicsBodyComponent(rb)).ok();
    }

    scene
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_instantiation() {
        let empty = create_empty_scene();
        assert!(!empty.world.is_empty());

        let char_scene = create_character_scene();
        assert!(char_scene.world.len() > empty.world.len());

        let sandbox = create_physics_sandbox();
        assert!(sandbox.world.len() >= 5);
        assert_eq!(sandbox.physics.bodies.len(), 5);
    }
}
