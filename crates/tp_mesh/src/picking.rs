//! 3Point 3D Uzay Seçim (Ray-Picking) ve Seçim Seti Yönetimi.

use crate::{
    bvh::MeshBvh,
    mesh::HalfEdgeMesh,
    topology::{EdgeId, FaceId, VertexId},
};
use glam::Vec3;
use std::collections::HashSet;
use tp_math::Ray;

/// Editör seçim modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    Vertex,
    Edge,
    #[default]
    Face,
}

/// Yüzey seçim sonucu bilgisi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceHit {
    pub face_id: FaceId,
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

/// Kullanıcının aktif seçimlerini tutan yapı.
#[derive(Debug, Default, Clone)]
pub struct SelectionSet {
    pub mode: SelectionMode,
    pub selected_vertices: HashSet<VertexId>,
    pub selected_edges: HashSet<EdgeId>,
    pub selected_faces: HashSet<FaceId>,
}

impl SelectionSet {
    /// Yeni bir seçim seti oluşturur.
    #[must_use]
    pub fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            selected_vertices: HashSet::new(),
            selected_edges: HashSet::new(),
            selected_faces: HashSet::new(),
        }
    }

    /// Tüm seçimleri temizler.
    pub fn clear(&mut self) {
        self.selected_vertices.clear();
        self.selected_edges.clear();
        self.selected_faces.clear();
    }

    /// Seçili öğe olup olmadığını döner.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self.mode {
            SelectionMode::Vertex => self.selected_vertices.is_empty(),
            SelectionMode::Edge => self.selected_edges.is_empty(),
            SelectionMode::Face => self.selected_faces.is_empty(),
        }
    }
}

/// Işın kullanarak mesh üzerindeki en yakın yüzeyi seçer.
pub fn pick_face(mesh: &HalfEdgeMesh, bvh: &MeshBvh, ray: &Ray) -> Option<FaceHit> {
    let (face_id, distance, point) = bvh.ray_intersect(mesh, ray)?;
    let normal = mesh.compute_face_normal(face_id);

    Some(FaceHit {
        face_id,
        distance,
        point,
        normal,
    })
}

/// Işına en yakın köşe noktasını seçer (Mesafe eşiği ile).
pub fn pick_vertex(mesh: &HalfEdgeMesh, ray: &Ray, threshold: f32) -> Option<(VertexId, f32)> {
    let mut closest: Option<(VertexId, f32)> = None;

    for (idx, vertex) in mesh.vertices.iter().enumerate() {
        if vertex.is_deleted {
            continue;
        }

        // Işın doğrusu ile nokta arasındaki en kısa mesafeyi hesapla
        let v_pos = vertex.position;
        let v_to_origin = v_pos - ray.origin;
        let t = v_to_origin.dot(ray.direction);

        if t < 0.0 {
            continue; // Işının arkasında
        }

        let closest_point_on_ray = ray.at(t);
        let dist = (v_pos - closest_point_on_ray).length();

        if dist <= threshold {
            let is_closer = match closest {
                Some((_, min_t)) => t < min_t,
                None => true,
            };
            if is_closer {
                closest = Some((VertexId(idx as u32), t));
            }
        }
    }

    closest
}
