//! # `tp_physics`
//!
//! 3Point 3D Fizik ve Çarpışma Motoru (`rapier3d` tabanlı).
//!
//! - **Rijit Cisimler (`body`):** Dinamik, Statik (Fixed) ve Kinematik hareket modelleri.
//! - **Çarpıştırıcılar (`collider`):** Cuboid, Sphere, Capsule, Cylinder ve TriMesh geometrileri.
//! - **Fizik Dünyası (`world`):** Eşzamanlı rijit cisim simülasyonu, yerçekimi ve hızlandırılmış Raycast sorguları.

pub mod body;
pub mod collider;
pub mod error;
pub mod math_conv;
pub mod world;

pub use body::{RigidBodyDesc, RigidBodyHandle, RigidBodyType};
pub use collider::{ColliderDesc, ColliderHandle, ColliderShape};
pub use error::PhysicsError;
pub use world::{PhysicsWorld, RayHit};
