//! 3Point Sahne Yöneticisi (`Scene`) ve Motor Çalışma Durumu (`EngineState`).

use crate::{
    components::{
        CameraComponent, GlobalTransform, Hierarchy, LightComponent, MeshRenderer, Name,
        TransformComponent,
    },
    error::SceneError,
    systems::{sync_physics_to_transforms, sync_transforms_to_physics, update_hierarchy_transforms},
};
use glam::Vec4;
use hecs::{Entity, World};
use tp_math::Transform;
use tp_physics::PhysicsWorld;

/// Motorun çalışma modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EngineState {
    /// Düzenleme/Modelleme Modu: Fizik durdurulur, sahne serbestçe düzenlenebilir.
    #[default]
    Edit,
    /// Oyun/Canlı Motor Modu: Fizik simülasyonu, mantık ve animasyonlar eşzamanlı akar.
    Play,
    /// Duraklatılmış Oyun Modu: Fizik ve mantık donar, kare kare inceleme yapılabilir.
    Pause,
}

/// Tüm ECS dünyasını, fizik dünyasını ve sahne yaşam döngüsünü barındıran ana yapı.
pub struct Scene {
    /// Varlık Bileşen Sistemi dünyası.
    pub world: World,
    /// 3D Fizik dünyası.
    pub physics: PhysicsWorld,
    /// Motorun çalışma durumu.
    pub state: EngineState,
    /// Play moduna geçildiğinde alınan geri yükleme enstantanesi (Snapshot).
    snapshot: Option<Vec<(Entity, Transform)>>,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    /// Yeni ve boş bir sahne oluşturur.
    pub fn new() -> Self {
        let mut scene = Self {
            world: World::new(),
            physics: PhysicsWorld::default(),
            state: EngineState::Edit,
            snapshot: None,
        };

        // Başlangıç dönüşüm sistemini çalıştır
        update_hierarchy_transforms(&mut scene.world);
        scene
    }

    /// Boş bir sahne nesnesi (Entity) oluşturur.
    pub fn spawn_empty(&mut self, name: impl Into<String>) -> Entity {
        self.world.spawn((
            Name::new(name),
            TransformComponent::default(),
            GlobalTransform::default(),
            Hierarchy::default(),
        ))
    }

    /// 3D Geometriye (MeshRenderer) sahip bir nesne oluşturur.
    pub fn spawn_mesh(
        &mut self,
        name: impl Into<String>,
        transform: Transform,
        primitive: &'static str,
        color: Vec4,
    ) -> Entity {
        self.world.spawn((
            Name::new(name),
            TransformComponent(transform),
            GlobalTransform(transform.to_matrix()),
            Hierarchy::default(),
            MeshRenderer {
                primitive,
                visible: true,
                color,
                metallic: 0.1,
                roughness: 0.4,
            },
        ))
    }

    /// Kamera nesnesi oluşturur.
    pub fn spawn_camera(
        &mut self,
        name: impl Into<String>,
        transform: Transform,
        camera: CameraComponent,
    ) -> Entity {
        self.world.spawn((
            Name::new(name),
            TransformComponent(transform),
            GlobalTransform(transform.to_matrix()),
            Hierarchy::default(),
            camera,
        ))
    }

    /// Işık nesnesi oluşturur.
    pub fn spawn_light(
        &mut self,
        name: impl Into<String>,
        transform: Transform,
        light: LightComponent,
    ) -> Entity {
        self.world.spawn((
            Name::new(name),
            TransformComponent(transform),
            GlobalTransform(transform.to_matrix()),
            Hierarchy::default(),
            light,
        ))
    }

    /// İki varlık arasında ebeveyn-çocuk ilişkisi kurar veya kaldırır.
    pub fn set_parent(&mut self, child: Entity, parent: Option<Entity>) -> Result<(), SceneError> {
        if let Some(p) = parent {
            if child == p {
                return Err(SceneError::CyclicParenting(child));
            }
        }

        // Önceki ebeveynden çocuğu çıkar
        let old_parent = {
            let mut hierarchy = self
                .world
                .get::<&mut Hierarchy>(child)
                .map_err(|_| SceneError::EntityNotFound(child))?;
            let old = hierarchy.parent;
            hierarchy.parent = parent;
            old
        };

        if let Some(old_p) = old_parent {
            if let Ok(mut old_hierarchy) = self.world.get::<&mut Hierarchy>(old_p) {
                old_hierarchy.children.retain(|&c| c != child);
            }
        }

        // Yeni ebeveyne çocuğu ekle
        if let Some(new_p) = parent {
            let mut new_hierarchy = self
                .world
                .get::<&mut Hierarchy>(new_p)
                .map_err(|_| SceneError::EntityNotFound(new_p))?;
            if !new_hierarchy.children.contains(&child) {
                new_hierarchy.children.push(child);
            }
        }

        // Hiyerarşik matrisleri anında güncelle
        update_hierarchy_transforms(&mut self.world);
        Ok(())
    }

    /// Play (Oyun Çalışma) modunu başlatır.
    ///
    /// Edit modundaki mevcut dönüşümleri kaydeder (Snapshot).
    pub fn start_play(&mut self) {
        if self.state == EngineState::Edit {
            let mut snapshot_data = Vec::new();
            for (entity, transform) in self.world.query::<&TransformComponent>().iter() {
                snapshot_data.push((entity, transform.0));
            }
            self.snapshot = Some(snapshot_data);
            self.state = EngineState::Play;

            // Düzenleme pozisyonlarını fizik dünyasına aktar
            sync_transforms_to_physics(&self.world, &mut self.physics);
            tracing::info!("Oyun Motoru Canlı Çalışma Moduna (PLAY) geçti.");
        }
    }

    /// Play modunu duraklatır.
    pub fn pause_play(&mut self) {
        if self.state == EngineState::Play {
            self.state = EngineState::Pause;
        }
    }

    /// Duraklatılmış oyunu devam ettirir.
    pub fn resume_play(&mut self) {
        if self.state == EngineState::Pause {
            self.state = EngineState::Play;
        }
    }

    /// Play modunu durdurur ve sahneyi Play öncesi orijinal haline eksiksiz geri yükler.
    pub fn stop_play(&mut self) {
        if self.state != EngineState::Edit {
            if let Some(snapshot_data) = self.snapshot.take() {
                for (entity, original_transform) in snapshot_data {
                    if let Ok(mut transform) = self.world.get::<&mut TransformComponent>(entity) {
                        transform.0 = original_transform;
                    }
                }
            }

            self.state = EngineState::Edit;
            sync_transforms_to_physics(&self.world, &mut self.physics);
            update_hierarchy_transforms(&mut self.world);
            tracing::info!("Oyun Motoru Düzenleme Moduna (EDIT) döndü, sahne geri yüklendi.");
        }
    }

    /// Bir karelik sahne güncellemesi çalıştırır.
    pub fn update(&mut self, dt: f32) {
        match self.state {
            EngineState::Play => {
                // 1. Fizik dünyasını simüle et
                self.physics.step(dt);

                // 2. Fizik dönüşümlerini ECS varlıklarına yansıt
                sync_physics_to_transforms(&mut self.world, &self.physics);

                // 3. Ebeveyn-çocuk hiyerarşisini güncelle
                update_hierarchy_transforms(&mut self.world);
            }
            EngineState::Edit => {
                // Edit modunda sadece hiyerarşi matrislerini güncel tut
                update_hierarchy_transforms(&mut self.world);
            }
            EngineState::Pause => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::PhysicsBodyComponent;
    use glam::Vec3;
    use tp_physics::{ColliderDesc, RigidBodyDesc};

    #[test]
    fn test_scene_hierarchy_propagation() {
        let mut scene = Scene::new();

        // Ebeveyn (Parent): (0, 5, 0)
        let parent = scene.spawn_mesh(
            "Parent",
            Transform::from_translation(Vec3::new(0.0, 5.0, 0.0)),
            "Cube",
            Vec4::ONE,
        );

        // Çocuk (Child): (0, 2, 0) yerel öteleme
        let child = scene.spawn_mesh(
            "Child",
            Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
            "Sphere",
            Vec4::ONE,
        );

        scene.set_parent(child, Some(parent)).unwrap();

        // Dünya matrisinde çocuğun dünya pozisyonu 5 + 2 = 7 olmalıdır!
        let child_global = *scene.world.get::<&GlobalTransform>(child).unwrap();
        let child_world_pos = child_global.0.transform_point3(Vec3::ZERO);

        assert!((child_world_pos.y - 7.0).abs() < 1e-4);
    }

    #[test]
    fn test_scene_play_mode_and_restore() {
        let mut scene = Scene::new();

        // Düşen kutu
        let box_entity = scene.spawn_mesh(
            "FallingBox",
            Transform::from_translation(Vec3::new(0.0, 10.0, 0.0)),
            "Cube",
            Vec4::ONE,
        );

        let rb = scene.physics.add_body(
            RigidBodyDesc::dynamic().translation(Vec3::new(0.0, 10.0, 0.0)),
        );
        scene.physics.add_collider(ColliderDesc::cuboid(Vec3::splat(0.5)), Some(rb));
        scene.world.insert_one(box_entity, PhysicsBodyComponent(rb)).unwrap();

        // 1. Play Modunu Başlat
        scene.start_play();
        assert_eq!(scene.state, EngineState::Play);

        // 60 kare simüle et
        for _ in 0..60 {
            scene.update(1.0 / 60.0);
        }

        // Kutu aşağı düşmüş olmalı (Y < 10.0)
        let fallen_y = scene.world.get::<&TransformComponent>(box_entity).unwrap().0.translation.y;
        assert!(fallen_y < 9.0);

        // 2. Stop Moduna Dön
        scene.stop_play();
        assert_eq!(scene.state, EngineState::Edit);

        // Orijinal pozisyonuna (Y = 10.0) eksiksiz dönmüş olmalı!
        let restored_y = scene.world.get::<&TransformComponent>(box_entity).unwrap().0.translation.y;
        assert!((restored_y - 10.0).abs() < 1e-4);
    }
}
