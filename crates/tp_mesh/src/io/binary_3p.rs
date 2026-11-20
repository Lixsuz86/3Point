//! 3Point Yerel Kompakt İkili (Binary) Model Formatı (`.3p`).
//!
//! Düşük donanımlı sistemlerde metin ayrıştırma (parsing) yükünü tamamen ortadan kaldıran,
//! doğrudan GPU belleğine kopyalanabilir sıfır kopyalı (Zero-Copy) ikili dosya yapısı.

use crate::{
    error::MeshError,
    mesh::{HalfEdgeMesh, MeshVertexData},
};
use bytemuck::{cast_slice, Pod, Zeroable};
use tp_math::Aabb;

/// `.3p` dosya başlığı.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Binary3pHeader {
    pub magic: [u8; 4], // b"3PNT"
    pub version: u32,   // 1
    pub vertex_count: u32,
    pub index_count: u32,
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
}

pub const MAGIC_3PNT: [u8; 4] = *b"3PNT";
pub const VERSION_3PNT: u32 = 1;

/// Bir HalfEdgeMesh'i kompakt `.3p` ikili bayt dizisine serileştirir.
pub fn export_3p(mesh: &HalfEdgeMesh) -> Vec<u8> {
    let (vertices, indices) = mesh.to_tri_mesh();
    let aabb = mesh.compute_aabb();

    let header = Binary3pHeader {
        magic: MAGIC_3PNT,
        version: VERSION_3PNT,
        vertex_count: vertices.len() as u32,
        index_count: indices.len() as u32,
        aabb_min: aabb.min.to_array(),
        aabb_max: aabb.max.to_array(),
    };

    let header_bytes = bytemuck::bytes_of(&header);
    let vertex_bytes = cast_slice::<MeshVertexData, u8>(&vertices);
    let index_bytes = cast_slice::<u32, u8>(&indices);

    let mut out = Vec::with_capacity(header_bytes.len() + vertex_bytes.len() + index_bytes.len());
    out.extend_from_slice(header_bytes);
    out.extend_from_slice(vertex_bytes);
    out.extend_from_slice(index_bytes);
    out
}

/// `.3p` ikili bayt dizisini ayrıştırarak doğrudan GPU tamponlarına uygun tepe ve indeks listesi döndürür.
pub fn import_3p(bytes: &[u8]) -> Result<(Vec<MeshVertexData>, Vec<u32>, Aabb), MeshError> {
    let header_size = std::mem::size_of::<Binary3pHeader>();
    if bytes.len() < header_size {
        return Err(MeshError::TopologyError("Geçersiz veya eksik .3p dosya boyutu".into()));
    }

    let header: Binary3pHeader = *bytemuck::from_bytes(&bytes[..header_size]);
    if header.magic != MAGIC_3PNT {
        return Err(MeshError::TopologyError("Geçersiz .3p sihirli baytları (Magic Mismatch)".into()));
    }
    if header.version != VERSION_3PNT {
        return Err(MeshError::TopologyError(format!("Desteklenmeyen .3p sürümü: {}", header.version)));
    }

    let v_size = header.vertex_count as usize * std::mem::size_of::<MeshVertexData>();
    let i_size = header.index_count as usize * std::mem::size_of::<u32>();

    if bytes.len() < header_size + v_size + i_size {
        return Err(MeshError::TopologyError(".3p veri akışı beklenenden kısa".into()));
    }

    let v_bytes = &bytes[header_size..header_size + v_size];
    let i_bytes = &bytes[header_size + v_size..header_size + v_size + i_size];

    let vertices: Vec<MeshVertexData> = cast_slice::<u8, MeshVertexData>(v_bytes).to_vec();
    let indices: Vec<u32> = cast_slice::<u8, u32>(i_bytes).to_vec();
    let aabb = Aabb::new(header.aabb_min.into(), header.aabb_max.into());

    Ok((vertices, indices, aabb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::create_cube;
    use tp_math::Vec3;

    #[test]
    fn test_binary_3p_roundtrip() {
        let original_cube = create_cube(Vec3::splat(1.5));
        let bytes = export_3p(&original_cube);

        assert!(bytes.len() > std::mem::size_of::<Binary3pHeader>());

        let (vertices, indices, aabb) = import_3p(&bytes).unwrap();
        let (orig_v, orig_i) = original_cube.to_tri_mesh();

        assert_eq!(vertices.len(), orig_v.len());
        assert_eq!(indices.len(), orig_i.len());
        assert!((aabb.min.x - -0.75).abs() < 1e-4);
        assert!((aabb.max.x - 0.75).abs() < 1e-4);
    }
}
