//! 3Point Hiyerarşik Sınırlayıcı Hacim Ağacı (Bounding Volume Hierarchy - BVH).

use crate::{mesh::HalfEdgeMesh, topology::FaceId};
use glam::Vec3;
use tp_math::{Aabb, Ray};

/// BVH ağacının bir düğümü.
#[derive(Debug, Clone)]
pub enum BvhNode {
    Leaf {
        aabb: Aabb,
        face_ids: Vec<FaceId>,
    },
    Internal {
        aabb: Aabb,
        left: Box<BvhNode>,
        right: Box<BvhNode>,
    },
}

impl BvhNode {
    /// Düğümün AABB sınırını döner.
    #[inline]
    #[must_use]
    pub fn aabb(&self) -> &Aabb {
        match self {
            Self::Leaf { aabb, .. } | Self::Internal { aabb, .. } => aabb,
        }
    }
}

/// Yüzeyler üzerinde $O(\log N)$ hızında ışın kesişimini sağlayan BVH yapısı.
pub struct MeshBvh {
    pub root: Option<BvhNode>,
}

impl MeshBvh {
    /// Mesh yüzeylerinden dengeli bir BVH ağacı inşa eder.
    pub fn build(mesh: &HalfEdgeMesh) -> Self {
        let mut face_data: Vec<(FaceId, Aabb, Vec3)> = Vec::new();

        for (idx, face) in mesh.faces.iter().enumerate() {
            if face.is_deleted {
                continue;
            }
            let fid = FaceId(idx as u32);
            let v_ids = mesh.face_vertices(fid);
            if v_ids.len() < 3 {
                continue;
            }

            let mut aabb = Aabb::EMPTY;
            let mut center = Vec3::ZERO;
            for &vid in &v_ids {
                let p = mesh.vertices[vid.0 as usize].position;
                aabb.encapsulate_point(p);
                center += p;
            }
            center /= v_ids.len() as f32;

            face_data.push((fid, aabb, center));
        }

        if face_data.is_empty() {
            return Self { root: None };
        }

        let root = Self::build_recursive(&mut face_data);
        Self { root: Some(root) }
    }

    fn build_recursive(faces: &mut [(FaceId, Aabb, Vec3)]) -> BvhNode {
        let mut total_aabb = Aabb::EMPTY;
        for (_, aabb, _) in faces.iter() {
            total_aabb.encapsulate_aabb(aabb);
        }

        if faces.len() <= 4 {
            let ids = faces.iter().map(|(fid, _, _)| *fid).collect();
            return BvhNode::Leaf {
                aabb: total_aabb,
                face_ids: ids,
            };
        }

        // En geniş ekseni bul (X, Y veya Z)
        let extents = total_aabb.extents();
        let axis = if extents.x >= extents.y && extents.x >= extents.z {
            0
        } else if extents.y >= extents.z {
            1
        } else {
            2
        };

        // Merkezlere göre sırala ve ikiye böl
        faces.sort_by(|a, b| {
            let ca = match axis {
                0 => a.2.x,
                1 => a.2.y,
                _ => a.2.z,
            };
            let cb = match axis {
                0 => b.2.x,
                1 => b.2.y,
                _ => b.2.z,
            };
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mid = faces.len() / 2;
        let (left_slice, right_slice) = faces.split_at_mut(mid);

        let left_node = Self::build_recursive(left_slice);
        let right_node = Self::build_recursive(right_slice);

        BvhNode::Internal {
            aabb: total_aabb,
            left: Box::new(left_node),
            right: Box::new(right_node),
        }
    }

    /// Işın ile en yakın yüzey kesişimini $O(\log N)$ sürede sorgular.
    pub fn ray_intersect(&self, mesh: &HalfEdgeMesh, ray: &Ray) -> Option<(FaceId, f32, Vec3)> {
        let root = self.root.as_ref()?;
        let mut closest_hit: Option<(FaceId, f32, Vec3)> = None;

        Self::intersect_node(root, mesh, ray, &mut closest_hit);
        closest_hit
    }

    fn intersect_node(
        node: &BvhNode,
        mesh: &HalfEdgeMesh,
        ray: &Ray,
        closest: &mut Option<(FaceId, f32, Vec3)>,
    ) {
        if node.aabb().intersects(&Aabb::new(ray.origin, ray.origin)).not()
            && ray.intersect_aabb(node.aabb()).is_none()
        {
            return;
        }

        // Eğer mevcut en yakın mesafeden daha uzaktaysa bu dalı buda (Branch Culling)
        if let Some((_, min_t, _)) = *closest {
            if let Some((t_enter, _)) = ray.intersect_aabb(node.aabb()) {
                if t_enter > min_t {
                    return;
                }
            }
        }

        match node {
            BvhNode::Leaf { face_ids, .. } => {
                for &fid in face_ids {
                    let v_ids = mesh.face_vertices(fid);
                    if v_ids.len() < 3 {
                        continue;
                    }

                    // Fan üçgenleme üzerinden kesişim
                    let v0 = mesh.vertices[v_ids[0].0 as usize].position;
                    for i in 1..(v_ids.len() - 1) {
                        let v1 = mesh.vertices[v_ids[i].0 as usize].position;
                        let v2 = mesh.vertices[v_ids[i + 1].0 as usize].position;

                        if let Some((t, _)) = ray.intersect_triangle(v0, v1, v2) {
                            let is_closer = match closest {
                                Some((_, curr_min, _)) => t < *curr_min,
                                None => true,
                            };
                            if is_closer {
                                let hit_point = ray.at(t);
                                *closest = Some((fid, t, hit_point));
                            }
                        }
                    }
                }
            }
            BvhNode::Internal { left, right, .. } => {
                Self::intersect_node(left, mesh, ray, closest);
                Self::intersect_node(right, mesh, ray, closest);
            }
        }
    }
}

trait NotExt {
    fn not(self) -> bool;
}
impl NotExt for bool {
    #[inline]
    fn not(self) -> bool {
        !self
    }
}

