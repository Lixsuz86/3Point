//! 3Point Poligon Ekstrüzyon (Extrude) Operatörü.

use crate::{error::MeshError, mesh::HalfEdgeMesh, topology::FaceId};

impl HalfEdgeMesh {
    /// Seçili bir yüzeyi normali doğrultusunda dışa doğru uzatır (Extrude).
    /// Yeni yan yüzeyler oluşturur ve ötelenmiş tavan yüzeyinin kimliğini döner.
    pub fn extrude_face(&mut self, face: FaceId, distance: f32) -> Result<FaceId, MeshError> {
        if face.0 as usize >= self.faces.len() || self.faces[face.0 as usize].is_deleted {
            return Err(MeshError::InvalidFaceId(face.0));
        }

        let orig_vertices = self.face_vertices(face);
        let n = orig_vertices.len();
        if n < 3 {
            return Err(MeshError::DegenerateFace(n));
        }

        let normal = self.compute_face_normal(face);
        let offset = normal * distance;

        // Yeni tavan köşelerini oluştur
        let mut new_vertices = Vec::with_capacity(n);
        for &vid in &orig_vertices {
            let old_pos = self.vertices[vid.0 as usize].position;
            let old_uv = self.vertices[vid.0 as usize].uv;
            let new_vid = self.add_vertex_full(old_pos + offset, normal, old_uv);
            new_vertices.push(new_vid);
        }

        // Yan dörtgen (Quad) yüzeyleri oluştur: (orig[i], orig[i+1], new[i+1], new[i])
        for i in 0..n {
            let next_i = (i + 1) % n;
            let b0 = orig_vertices[i];
            let b1 = orig_vertices[next_i];
            let t1 = new_vertices[next_i];
            let t0 = new_vertices[i];

            self.add_face(&[b0, b1, t1, t0])?;
        }

        // Eski yüzeyi silinmiş olarak işaretle ve yeni tavan yüzeyi ekle
        self.faces[face.0 as usize].is_deleted = true;
        let top_face = self.add_face(&new_vertices)?;

        self.link_twins();
        self.compute_vertex_normals();

        Ok(top_face)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::create_cube;
    use glam::Vec3;

    #[test]
    fn test_cube_face_extrude() {
        let mut cube = create_cube(Vec3::ONE);
        let initial_face_count = cube.faces.len();

        let top_face = cube.extrude_face(FaceId(3), 1.0).expect("Ekstrüzyon başarısız");
        assert_eq!(top_face.0, initial_face_count as u32 + 4); // 4 yan yüzey + 1 tavan yüzeyi
    }
}
