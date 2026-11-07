//! # `tp_mesh`
//!
//! 3Point 3D Modelleme, Yarım-Kenar (Half-Edge / DCEL) topolojisi,
//! parametrik geometriler, modelleme operatörleri ve BVH ray-picking kütüphanesi.

pub mod bvh;
pub mod error;
pub mod history;
pub mod mesh;
pub mod ops;
pub mod picking;
pub mod primitives;
pub mod topology;

pub use bvh::{BvhNode, MeshBvh};
pub use error::MeshError;
pub use history::{MeshCommand, SnapshotMeshCommand, UndoStack};
pub use mesh::{HalfEdgeMesh, MeshVertexData};
pub use picking::{pick_face, pick_vertex, FaceHit, SelectionMode, SelectionSet};
pub use primitives::{
    create_cube, create_cylinder, create_plane, create_uv_sphere,
};
pub use topology::{EdgeId, FaceId, HalfEdgeId, VertexId};
