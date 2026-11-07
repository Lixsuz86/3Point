//! 3Point Poligon İçe Daraltma (Inset) Operatörü.

use crate::{error::MeshError, mesh::HalfEdgeMesh, topology::FaceId};
use glam::Vec3;

impl HalfEdgeMesh {
    /// Seçili bir yüzeyin sınırlarını merkezine doğru daraltarak iç içe yeni bir yüzey (Inset) oluşturur.
    /// `thickness_factor`: [0.0..1.0] aralığında merkeze yaklaşma oranıdır.
    pub fn inset_face(&mut self, face: FaceId, thickness_factor: f32) -> Result<FaceId, MeshError> {
        if face.0 as usize >= self.faces.len() || self.faces[face.0 as usize].is_deleted {
            return Err(MeshError::InvalidFaceId(face.0));
        }

        let orig_vertices = self.face_vertices(face);
        let n = orig_vertices.len();
        if n < 3 {
            return Err(MeshError::DegenerateFace(n));
        }

        let factor = thickness_factor.clamp(0.01, 0.95);

        // Yüzey merkez noktasını (centroid) hesapla
        let mut center = Vec3::ZERO;
        for &vid in &orig_vertices {
            center += self.vertices[vid.0 as usize].position;
        }
        center /= n as f32;

        let normal = self.compute_face_normal(face);

        // Merkeze doğru daraltılmış yeni iç köşeleri oluştur
        let mut inner_vertices = Vec::with_capacity(n);
        for &vid in &orig_vertices {
            let pos = self.vertices[vid.0 as usize].position;
            let uv = self.vertices[vid.0 as usize].uv;
            let inset_pos = pos + (center - pos) * factor;

            let inner_vid = self.add_vertex_full(inset_pos, normal, uv);
            inner_vertices.push(inner_vid);
        }

        // Çevreleyen dörtgen (Quad) yüzeyleri oluştur: (orig[i], orig[i+1], inner[i+1], inner[i])
        for i in 0..n {
            let next_i = (i + 1) % n;
            let b0 = orig_vertices[i];
            let b1 = orig_vertices[next_i];
            let t1 = inner_vertices[next_i];
            let t0 = inner_vertices[i];

            self.add_face(&[b0, b1, t1, t0])?;
        }

        // Eski yüzeyi silinmiş işaretle ve iç yüzeyi ekle
        self.faces[face.0 as usize].is_deleted = true;
        let inner_face = self.add_face(&inner_vertices)?;

        self.link_twins();
        self.compute_vertex_normals();

        Ok(inner_face)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::create_cube;

    #[test]
    fn test_cube_face_inset() {
        let mut cube = create_cube(Vec3::ONE);
        let initial_face_count = cube.faces.len();

        let inner_face = cube.inset_face(FaceId(0), 0.2).expect("Inset başarısız");
        assert_eq!(inner_face.0, initial_face_count as u32 + 4); // 4 çevre yüzeyi + 1 iç yüzey
    }
}
