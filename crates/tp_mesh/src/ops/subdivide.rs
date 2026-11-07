//! 3Point Poligon Yüzey Alt Bölümleme (Subdivision Surfaces).

use crate::mesh::HalfEdgeMesh;
use glam::Vec3;
use std::collections::HashMap;

impl HalfEdgeMesh {
    /// Mesh yüzeylerini dörtgenlere alt-bölerek (Subdivide) poligon çözünürlüğünü ve yumuşaklığını artırır.
    #[must_use]
    pub fn subdivide(&self) -> HalfEdgeMesh {
        let mut new_mesh = HalfEdgeMesh::new();

        // 1. Orijinal köşeleri yeni mesh'e aktar
        let mut old_to_new_vertex = HashMap::new();
        for (i, v) in self.vertices.iter().enumerate() {
            if !v.is_deleted {
                let new_vid = new_mesh.add_vertex_full(v.position, v.normal, v.uv);
                old_to_new_vertex.insert(i as u32, new_vid);
            }
        }

        // 2. Her yüzey için bir merkez noktası (Face Point) üret
        let mut face_points = HashMap::new();
        for (f_idx, face) in self.faces.iter().enumerate() {
            if face.is_deleted {
                continue;
            }
            let v_ids = self.face_vertices(crate::topology::FaceId(f_idx as u32));
            let mut center = Vec3::ZERO;
            for &vid in &v_ids {
                center += self.vertices[vid.0 as usize].position;
            }
            center /= v_ids.len() as f32;

            let face_point_vid = new_mesh.add_vertex(center);
            face_points.insert(f_idx as u32, face_point_vid);
        }

        // 3. Her kenar için bir orta nokta (Edge Point) üret
        let mut edge_points = HashMap::new();
        for edge in &self.edges {
            if edge.is_deleted {
                continue;
            }
            let he = &self.half_edges[edge.half_edge.0 as usize];
            let next_he = &self.half_edges[he.next.0 as usize];
            let v0 = he.origin;
            let v1 = next_he.origin;

            let p0 = self.vertices[v0.0 as usize].position;
            let p1 = self.vertices[v1.0 as usize].position;
            let mid = (p0 + p1) * 0.5;

            let edge_point_vid = new_mesh.add_vertex(mid);
            let key = if v0.0 < v1.0 { (v0.0, v1.0) } else { (v1.0, v0.0) };
            edge_points.insert(key, edge_point_vid);
        }

        // 4. Her yüzeyin köşelerinden yeni 4'genler (Quads) oluştur
        for (f_idx, face) in self.faces.iter().enumerate() {
            if face.is_deleted {
                continue;
            }
            let v_ids = self.face_vertices(crate::topology::FaceId(f_idx as u32));
            let face_point = face_points[&(f_idx as u32)];
            let n = v_ids.len();

            for i in 0..n {
                let v_curr = v_ids[i].0;
                let v_next = v_ids[(i + 1) % n].0;
                let v_prev = v_ids[(i + n - 1) % n].0;

                let orig_new_vid = old_to_new_vertex[&v_curr];

                let e_curr_key = if v_curr < v_next { (v_curr, v_next) } else { (v_next, v_curr) };
                let e_prev_key = if v_prev < v_curr { (v_prev, v_curr) } else { (v_curr, v_prev) };

                let edge_curr = edge_points[&e_curr_key];
                let edge_prev = edge_points[&e_prev_key];

                // Yeni 4'gen yüzey: (orig, edge_curr, face_point, edge_prev)
                let _ = new_mesh.add_face(&[orig_new_vid, edge_curr, face_point, edge_prev]);
            }
        }

        new_mesh.link_twins();
        new_mesh.compute_vertex_normals();
        new_mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::create_cube;

    #[test]
    fn test_cube_subdivide() {
        let cube = create_cube(Vec3::ONE);
        assert_eq!(cube.faces.len(), 6);

        let sub_cube = cube.subdivide();
        // 6 dörtgen yüzeyin her biri 4 yeni dörtgene bölünür: 6 * 4 = 24 yüzey
        assert_eq!(sub_cube.faces.len(), 24);
    }
}
