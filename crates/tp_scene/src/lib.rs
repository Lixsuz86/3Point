//! # `tp_scene`
//!
//! 3Point Sahne Yönetimi, Arketip Tabanlı ECS ve Canlı Oyun Motoru Runtime'ı.
//!
//! - **Varlık Bileşen Sistemi (`hecs` tabanlı):** `Name`, `TransformComponent`, `GlobalTransform`, `Hierarchy`, `MeshRenderer`, `CameraComponent`, `LightComponent`.
//! - **Sistemler (`systems`):** Sıfır dinamik tahsisli hiyerarşik dönüşüm yayılımı ve Fizik-ECS senkronizasyonu.
//! - **Sahne Yaşam Döngüsü (`scene`):** Edit, Play ve Pause modları; Play durdurulduğunda otomatik geri yükleme sağlayan Snapshot altyapısı.

pub mod components;
pub mod error;
pub mod scene;
pub mod systems;

pub use components::{
    CameraComponent, GlobalTransform, Hierarchy, LightComponent, LightType, MeshRenderer, Name,
    PhysicsBodyComponent, PhysicsColliderComponent, TransformComponent,
};
pub use error::SceneError;
pub use scene::{EngineState, Scene};
pub use systems::{
    sync_physics_to_transforms, sync_transforms_to_physics, update_hierarchy_transforms,
};
