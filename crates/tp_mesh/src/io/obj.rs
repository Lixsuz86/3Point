//! Wavefront .OBJ 3D Model Formatı İçe/Dışa Aktarıcısı (Importer & Exporter).

use crate::{
    error::MeshError,
    mesh::HalfEdgeMesh,
};
use tp_math::Vec3;

/// Bir HalfEdgeMesh'i standart Wavefront OBJ metin formatına dönüştürür.
pub fn export_obj(mesh: &HalfEdgeMesh, object_name: &str) -> String {
    let mut out = String::with_capacity(mesh.vertices.len() * 32 + mesh.faces.len() * 24);
    out.push_str("# 3Point 3D Studio Wavefront OBJ Export\n");
    out.push_str(&format!("o {}\n\n", object_name));

    // 1. Köşeler (Vertices: v x y z)
    for v in &mesh.vertices {
        if !v.is_deleted {
            out.push_str(&format!("v {:.6} {:.6} {:.6}\n", v.position.x, v.position.y, v.position.z));
        }
    }

    // 2. Normaller (Normals: vn x y z)
    for v in &mesh.vertices {
        if !v.is_deleted {
            out.push_str(&format!("vn {:.6} {:.6} {:.6}\n", v.normal.x, v.normal.y, v.normal.z));
        }
    }

    // 3. Yüzeyler (Faces: f v1//vn1 v2//vn2 v3//vn3 ...)
    // OBJ indeksleri 1-tabanlıdır.
    for face_id in 0..mesh.faces.len() {
        if !mesh.faces[face_id].is_deleted {
            let v_ids = mesh.face_vertices(crate::topology::FaceId(face_id as u32));
            if v_ids.len() >= 3 {
                out.push('f');
                for vid in v_ids {
                    let idx = vid.0 + 1;
                    out.push_str(&format!(" {}//{}", idx, idx));
                }
                out.push('\n');
            }
        }
    }

    out
}

/// Wavefront OBJ metnini ayrıştırarak yeni bir `HalfEdgeMesh` oluşturur.
pub fn import_obj(obj_str: &str) -> Result<HalfEdgeMesh, MeshError> {
    let mut raw_positions = Vec::new();
    let mut face_indices: Vec<Vec<u32>> = Vec::new();

    for line in obj_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(tag) = parts.next() else { continue };

        match tag {
            "v" => {
                let x = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                let y = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                let z = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                raw_positions.push(Vec3::new(x, y, z));
            }
            "f" => {
                let mut face_verts = Vec::new();
                for vertex_spec in parts {
                    // "v/vt/vn" veya "v//vn" veya "v" formatları
                    if let Some(v_str) = vertex_spec.split('/').next() {
                        if let Ok(idx_1_based) = v_str.parse::<i32>() {
                            let idx = if idx_1_based > 0 {
                                (idx_1_based - 1) as u32
                            } else {
                                // Negatif göreceli indeks desteği
                                (raw_positions.len() as i32 + idx_1_based) as u32
                            };
                            face_verts.push(idx);
                        }
                    }
                }
                if face_verts.len() >= 3 {
                    face_indices.push(face_verts);
                }
            }
            _ => {}
        }
    }

    if raw_positions.is_empty() {
        return Err(MeshError::TopologyError("OBJ dosyasında hiç köşe bulunamadı".into()));
    }

    // Çokgenleri (N-gon) üçgenlere bölerek HalfEdgeMesh'e aktar
    let mut flat_indices = Vec::with_capacity(face_indices.len() * 3);
    for f in face_indices {
        for i in 1..f.len() - 1 {
            flat_indices.push(f[0]);
            flat_indices.push(f[i]);
            flat_indices.push(f[i + 1]);
        }
    }

    let mut mesh = HalfEdgeMesh::from_triangles(&raw_positions, &flat_indices)?;
    mesh.compute_vertex_normals();
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::create_cube;

    #[test]
    fn test_obj_export_and_import() {
        let original_cube = create_cube(Vec3::splat(2.0));
        let obj_text = export_obj(&original_cube, "TestCube");

        assert!(obj_text.contains("o TestCube"));
        assert!(obj_text.contains("v 1.000000 1.000000 1.000000"));
        assert!(obj_text.contains("f "));

        let imported_cube = import_obj(&obj_text).unwrap();
        assert_eq!(imported_cube.vertices.len(), original_cube.vertices.len());
        assert!(!imported_cube.faces.is_empty());
    }
}
