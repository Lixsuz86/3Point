//! 3Point Sahne ECS Sistemleri (Transform hiyerarşisi ve Fizik eşleme).

use crate::components::{GlobalTransform, Hierarchy, PhysicsBodyComponent, TransformComponent};
use glam::Mat4;
use hecs::{Entity, World};
use tp_physics::PhysicsWorld;

/// Tüm sahne grafiğini tarayarak yerel dönüşümleri dünya uzayı matrislerine yayar (Transform Propagation).
pub fn update_hierarchy_transforms(world: &mut World) {
    // 1. Kök nesneleri (ebeveyni olmayanları) bul
    let mut root_entities = Vec::new();
    for (entity, (_, hierarchy)) in world.query::<(&TransformComponent, Option<&Hierarchy>)>().iter() {
        let is_root = hierarchy.is_none_or(|h| h.parent.is_none());
        if is_root {
            root_entities.push(entity);
        }
    }

    // 2. Kökten yapraklara doğru özyinelemesiz (stack tabanlı) hiyerarşik yayılım
    let mut stack: Vec<(Entity, Mat4)> = Vec::with_capacity(root_entities.len());

    for root in root_entities {
        if let Ok(mut transform_q) = world.query_one::<(&TransformComponent, &mut GlobalTransform)>(root) {
            if let Some((local, global)) = transform_q.get() {
                let mat = local.0.to_matrix();
                global.0 = mat;
                stack.push((root, mat));
            }
        }
    }

    while let Some((parent_entity, parent_world_mat)) = stack.pop() {
        // Ebeveynin çocuklarını al
        let children = if let Ok(hierarchy) = world.get::<&Hierarchy>(parent_entity) {
            hierarchy.children.clone()
        } else {
            Vec::new()
        };

        for child in children {
            if let Ok(mut child_q) = world.query_one::<(&TransformComponent, &mut GlobalTransform)>(child) {
                if let Some((child_local, child_global)) = child_q.get() {
                    let child_world_mat = parent_world_mat * child_local.0.to_matrix();
                    child_global.0 = child_world_mat;
                    stack.push((child, child_world_mat));
                }
            }
        }
    }
}

/// Fizik dünyasındaki simüle edilen konum ve rotasyonları ECS bileşenlerine yansıtır.
pub fn sync_physics_to_transforms(world: &mut World, physics: &PhysicsWorld) {
    for (_entity, (body, transform)) in world.query_mut::<(&PhysicsBodyComponent, &mut TransformComponent)>() {
        if let Some((pos, rot)) = physics.body_transform(body.0) {
            transform.0.translation = pos;
            transform.0.rotation = rot;
        }
    }
}

/// ECS dönüşümlerini fizik dünyasına zorla aktarır (Örn: Edit modunda elle taşıma yapıldığında).
pub fn sync_transforms_to_physics(world: &World, physics: &mut PhysicsWorld) {
    for (_entity, (body, transform)) in world.query::<(&PhysicsBodyComponent, &TransformComponent)>().iter() {
        physics.set_body_transform(body.0, transform.0.translation, transform.0.rotation);
    }
}
