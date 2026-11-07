//! 3Point Yarım-Kenar Manifold Mesh Topoloji Yöneticisi (`HalfEdgeMesh`).

use crate::{
    error::MeshError,
    topology::{Edge, EdgeId, Face, FaceId, HalfEdge, HalfEdgeId, Vertex, VertexId},
};
use glam::{Vec2, Vec3};
use std::collections::HashMap;
use tp_math::Aabb;

/// GPU render hattı ile doğrudan uyumlu köşe formatı.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertexData {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

/// Tüm modelleme operasyonlarının üzerinde çalıştığı yarım-kenar topoloji yapısı.
#[derive(Debug, Clone, Default)]
pub struct HalfEdgeMesh {
    pub vertices: Vec<Vertex>,
    pub half_edges: Vec<HalfEdge>,
    pub edges: Vec<Edge>,
    pub faces: Vec<Face>,
}

impl HalfEdgeMesh {
    /// Boş bir mesh oluşturur.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Yeni bir köşe ekler ve kimliğini döner.
    pub fn add_vertex(&mut self, position: Vec3) -> VertexId {
        let id = VertexId(self.vertices.len() as u32);
        self.vertices.push(Vertex::new(position));
        id
    }

    /// UV ve Normal bilgileriyle köşe ekler.
    pub fn add_vertex_full(&mut self, position: Vec3, normal: Vec3, uv: Vec2) -> VertexId {
        let id = VertexId(self.vertices.len() as u32);
        let mut v = Vertex::new(position);
        v.normal = normal;
        v.uv = uv;
        self.vertices.push(v);
        id
    }

    /// Saat yönünün tersine sıralanmış köşe kimliklerinden yeni bir poligon yüzeyi oluşturur.
    pub fn add_face(&mut self, v_ids: &[VertexId]) -> Result<FaceId, MeshError> {
        let n = v_ids.len();
        if n < 3 {
            return Err(MeshError::DegenerateFace(n));
        }

        for &vid in v_ids {
            if vid.0 as usize >= self.vertices.len() || self.vertices[vid.0 as usize].is_deleted {
                return Err(MeshError::InvalidVertexId(vid.0));
            }
        }

        let face_id = FaceId(self.faces.len() as u32);
        let first_he_idx = self.half_edges.len() as u32;

        // Yarım-kenarları ve yönsüz kenarları oluştur
        for (i, &v_curr) in v_ids.iter().enumerate() {
            let he_idx = HalfEdgeId(first_he_idx + i as u32);
            let next_idx = HalfEdgeId(first_he_idx + ((i + 1) % n) as u32);
            let prev_idx = HalfEdgeId(first_he_idx + ((i + n - 1) % n) as u32);

            let edge_id = EdgeId(self.edges.len() as u32);
            self.edges.push(Edge {
                half_edge: he_idx,
                is_deleted: false,
            });

            self.half_edges.push(HalfEdge {
                origin: v_curr,
                twin: None,
                next: next_idx,
                prev: prev_idx,
                face: Some(face_id),
                edge: edge_id,
                is_deleted: false,
            });

            // Köşenin referans yarım-kenarını ayarla
            self.vertices[v_curr.0 as usize].half_edge = Some(he_idx);
        }

        let first_he = HalfEdgeId(first_he_idx);
        self.faces.push(Face {
            half_edge: first_he,
            normal: Vec3::Y,
            is_deleted: false,
        });

        // Yüzey normalini anında hesapla
        let normal = self.compute_face_normal(face_id);
        self.faces[face_id.0 as usize].normal = normal;

        Ok(face_id)
    }

    /// Bir yüzeyin sınır köşelerini sırasıyla listeler.
    pub fn face_vertices(&self, face: FaceId) -> Vec<VertexId> {
        let mut result = Vec::with_capacity(4);
        let start_he = self.faces[face.0 as usize].half_edge;
        let mut curr_he = start_he;

        loop {
            let he = &self.half_edges[curr_he.0 as usize];
            result.push(he.origin);
            curr_he = he.next;
            if curr_he == start_he {
                break;
            }
        }
        result
    }

    /// Bir yüzeyin etrafındaki yarım-kenarları listeler.
    pub fn face_half_edges(&self, face: FaceId) -> Vec<HalfEdgeId> {
        let mut result = Vec::with_capacity(4);
        let start_he = self.faces[face.0 as usize].half_edge;
        let mut curr_he = start_he;

        loop {
            result.push(curr_he);
            let he = &self.half_edges[curr_he.0 as usize];
            curr_he = he.next;
            if curr_he == start_he {
                break;
            }
        }
        result
    }

    /// Bir köşeye bağlı komşu köşeleri listeler (1-ring komşuluk).
    pub fn vertex_neighbors(&self, vertex: VertexId) -> Vec<VertexId> {
        let mut neighbors = Vec::new();
        let Some(start_he_id) = self.vertices[vertex.0 as usize].half_edge else {
            return neighbors;
        };

        let mut curr_he_id = start_he_id;
        loop {
            let he = &self.half_edges[curr_he_id.0 as usize];
            let next_he = &self.half_edges[he.next.0 as usize];
            neighbors.push(next_he.origin);

            // Twin üzerinden saat yönünde dön
            if let Some(twin_id) = he.twin {
                let twin = &self.half_edges[twin_id.0 as usize];
                curr_he_id = twin.next;
                if curr_he_id == start_he_id {
                    break;
                }
            } else {
                break; // Sınır köşesi
            }
        }
        neighbors
    }

    /// Bir yüzeyin geometrik normal vektörünü hesaplar (Newell metodu - N-gonlar için kararlı).
    pub fn compute_face_normal(&self, face: FaceId) -> Vec3 {
        let v_ids = self.face_vertices(face);
        if v_ids.len() < 3 {
            return Vec3::Y;
        }

        let mut normal = Vec3::ZERO;
        for i in 0..v_ids.len() {
            let current = self.vertices[v_ids[i].0 as usize].position;
            let next = self.vertices[v_ids[(i + 1) % v_ids.len()].0 as usize].position;

            normal.x += (current.y - next.y) * (current.z + next.z);
            normal.y += (current.z - next.z) * (current.x + next.x);
            normal.z += (current.x - next.x) * (current.y + next.y);
        }

        normal.normalize_or_zero()
    }

    /// Tüm köşelerin pürüzsüz (smooth) normal vektörlerini hesaplar.
    pub fn compute_vertex_normals(&mut self) {
        let mut accum_normals = vec![Vec3::ZERO; self.vertices.len()];

        for face in &self.faces {
            if face.is_deleted {
                continue;
            }
            let start_he = face.half_edge;
            let mut curr = start_he;
            loop {
                let he = &self.half_edges[curr.0 as usize];
                accum_normals[he.origin.0 as usize] += face.normal;
                curr = he.next;
                if curr == start_he {
                    break;
                }
            }
        }

        for (i, v) in self.vertices.iter_mut().enumerate() {
            if !v.is_deleted {
                v.normal = accum_normals[i].normalize_or_zero();
            }
        }
    }

    /// Eşleşen kenarlar arasında Twin (İkiz) yarım-kenar bağlantılarını kurar.
    pub fn link_twins(&mut self) {
        let mut edge_map: HashMap<(u32, u32), HalfEdgeId> = HashMap::with_capacity(self.half_edges.len());

        for i in 0..self.half_edges.len() {
            let he_id = HalfEdgeId(i as u32);
            let he = &self.half_edges[i];
            if he.is_deleted {
                continue;
            }

            let next_he = &self.half_edges[he.next.0 as usize];
            let v_from = he.origin.0;
            let v_to = next_he.origin.0;

            // Zıt yönlü kenar var mı kontrol et
            if let Some(&opposite_he_id) = edge_map.get(&(v_to, v_from)) {
                self.half_edges[he_id.0 as usize].twin = Some(opposite_he_id);
                self.half_edges[opposite_he_id.0 as usize].twin = Some(he_id);
            } else {
                edge_map.insert((v_from, v_to), he_id);
            }
        }
    }

    /// Tüm mesh'i kapsayan eksen hizalı sınırlayıcı kutuyu (AABB) hesaplar.
    pub fn compute_aabb(&self) -> Aabb {
        let mut aabb = Aabb::EMPTY;
        for v in &self.vertices {
            if !v.is_deleted {
                aabb.encapsulate_point(v.position);
            }
        }
        aabb
    }

    /// Mesh yüzeylerini GPU çizimi için üçgen indeksli tamponlara (Tri-Mesh) dönüştürür.
    pub fn to_tri_mesh(&self) -> (Vec<MeshVertexData>, Vec<u32>) {
        let mut gpu_vertices = Vec::new();
        let mut gpu_indices = Vec::new();

        for (idx, face) in self.faces.iter().enumerate() {
            if face.is_deleted {
                continue;
            }

            let v_ids = self.face_vertices(FaceId(idx as u32));

            if v_ids.len() < 3 {
                continue;
            }

            // Fan üçgenleme (N-gon to Triangles)
            let base_vertex_idx = gpu_vertices.len() as u32;
            for &vid in &v_ids {
                let v = &self.vertices[vid.0 as usize];
                gpu_vertices.push(MeshVertexData {
                    position: v.position.to_array(),
                    normal: v.normal.to_array(),
                    uv: v.uv.to_array(),
                    color: [0.8, 0.82, 0.86, 1.0],
                });
            }

            for i in 1..(v_ids.len() - 1) {
                gpu_indices.push(base_vertex_idx);
                gpu_indices.push(base_vertex_idx + i as u32);
                gpu_indices.push(base_vertex_idx + (i + 1) as u32);
            }
        }

        (gpu_vertices, gpu_indices)
    }

    /// Üçgen dizisinden yeni bir HalfEdgeMesh oluşturur.
    pub fn from_triangles(positions: &[Vec3], indices: &[u32]) -> Result<Self, MeshError> {
        let mut mesh = Self::new();
        let mut vertex_ids = Vec::with_capacity(positions.len());

        for &pos in positions {
            vertex_ids.push(mesh.add_vertex(pos));
        }

        for &[i0, i1, i2] in indices.as_chunks::<3>().0 {
            let v0 = vertex_ids[i0 as usize];
            let v1 = vertex_ids[i1 as usize];
            let v2 = vertex_ids[i2 as usize];
            mesh.add_face(&[v0, v1, v2])?;
        }

        mesh.link_twins();
        mesh.compute_vertex_normals();
        Ok(mesh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_triangle_creation() {
        let mut mesh = HalfEdgeMesh::new();
        let v0 = mesh.add_vertex(Vec3::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Vec3::new(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(Vec3::new(0.0, 1.0, 0.0));

        let face = mesh.add_face(&[v0, v1, v2]).expect("Yüzey eklenemedi");
        let vertices = mesh.face_vertices(face);
        assert_eq!(vertices, vec![v0, v1, v2]);

        let normal = mesh.compute_face_normal(face);
        assert!((normal.z - 1.0).abs() < 1e-4);
    }
}
