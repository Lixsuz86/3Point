//! 3Point Temel Geometri Üreticileri (Parametrik Primitives).

use crate::mesh::HalfEdgeMesh;
use glam::{Vec2, Vec3};
use std::f32::consts::PI;

/// Parametrik bir Küp (Cube) oluşturur.
pub fn create_cube(size: Vec3) -> HalfEdgeMesh {
    let mut mesh = HalfEdgeMesh::new();
    let h = size * 0.5;

    // 8 Köşe noktası
    let v0 = mesh.add_vertex_full(Vec3::new(-h.x, -h.y, -h.z), -Vec3::Z, Vec2::new(0.0, 0.0));
    let v1 = mesh.add_vertex_full(Vec3::new( h.x, -h.y, -h.z), -Vec3::Z, Vec2::new(1.0, 0.0));
    let v2 = mesh.add_vertex_full(Vec3::new( h.x,  h.y, -h.z), -Vec3::Z, Vec2::new(1.0, 1.0));
    let v3 = mesh.add_vertex_full(Vec3::new(-h.x,  h.y, -h.z), -Vec3::Z, Vec2::new(0.0, 1.0));

    let v4 = mesh.add_vertex_full(Vec3::new(-h.x, -h.y,  h.z), Vec3::Z, Vec2::new(0.0, 0.0));
    let v5 = mesh.add_vertex_full(Vec3::new( h.x, -h.y,  h.z), Vec3::Z, Vec2::new(1.0, 0.0));
    let v6 = mesh.add_vertex_full(Vec3::new( h.x,  h.y,  h.z), Vec3::Z, Vec2::new(1.0, 1.0));
    let v7 = mesh.add_vertex_full(Vec3::new(-h.x,  h.y,  h.z), Vec3::Z, Vec2::new(0.0, 1.0));

    // 6 Dörtgen Yüzey (CCW)
    let _ = mesh.add_face(&[v3, v2, v1, v0]); // Ön (-Z)
    let _ = mesh.add_face(&[v4, v5, v6, v7]); // Arka (+Z)
    let _ = mesh.add_face(&[v0, v1, v5, v4]); // Alt (-Y)
    let _ = mesh.add_face(&[v7, v6, v2, v3]); // Üst (+Y)
    let _ = mesh.add_face(&[v0, v4, v7, v3]); // Sol (-X)
    let _ = mesh.add_face(&[v6, v5, v1, v2]); // Sağ (+X)

    mesh.link_twins();
    mesh.compute_vertex_normals();
    mesh
}

/// Parametrik bir Düzlem (Plane) ızgarası üretir.
pub fn create_plane(width: f32, depth: f32, sub_x: u32, sub_z: u32) -> HalfEdgeMesh {
    let mut mesh = HalfEdgeMesh::new();
    let sub_x = sub_x.max(1);
    let sub_z = sub_z.max(1);

    let half_w = width * 0.5;
    let half_d = depth * 0.5;

    let mut grid = Vec::with_capacity(((sub_x + 1) * (sub_z + 1)) as usize);

    for z in 0..=sub_z {
        let fz = (z as f32) / (sub_z as f32);
        let pos_z = -half_d + fz * depth;

        for x in 0..=sub_x {
            let fx = (x as f32) / (sub_x as f32);
            let pos_x = -half_w + fx * width;

            let vid = mesh.add_vertex_full(
                Vec3::new(pos_x, 0.0, pos_z),
                Vec3::Y,
                Vec2::new(fx, fz),
            );
            grid.push(vid);
        }
    }

    let stride = sub_x + 1;
    for z in 0..sub_z {
        for x in 0..sub_x {
            let top_left = grid[(z * stride + x) as usize];
            let top_right = grid[(z * stride + x + 1) as usize];
            let bottom_left = grid[((z + 1) * stride + x) as usize];
            let bottom_right = grid[((z + 1) * stride + x + 1) as usize];

            let _ = mesh.add_face(&[top_left, bottom_left, bottom_right, top_right]);
        }
    }

    mesh.link_twins();
    mesh.compute_vertex_normals();
    mesh
}

/// Parametrik bir UV Küresi (UV Sphere) üretir.
pub fn create_uv_sphere(radius: f32, rings: u32, segments: u32) -> HalfEdgeMesh {
    let mut mesh = HalfEdgeMesh::new();
    let rings = rings.max(3);
    let segments = segments.max(3);

    // Üst kutup
    let north_pole = mesh.add_vertex_full(Vec3::new(0.0, radius, 0.0), Vec3::Y, Vec2::new(0.5, 1.0));

    // Ara halkalar
    let mut ring_vertices = Vec::new();
    for r in 1..rings {
        let theta = (r as f32 / rings as f32) * PI;
        let y = radius * theta.cos();
        let ring_r = radius * theta.sin();

        let mut current_ring = Vec::new();
        for s in 0..segments {
            let phi = (s as f32 / segments as f32) * 2.0 * PI;
            let x = ring_r * phi.cos();
            let z = ring_r * phi.sin();

            let uv = Vec2::new(s as f32 / segments as f32, 1.0 - (r as f32 / rings as f32));
            let pos = Vec3::new(x, y, z);
            let vid = mesh.add_vertex_full(pos, pos.normalize_or_zero(), uv);
            current_ring.push(vid);
        }
        ring_vertices.push(current_ring);
    }

    // Alt kutup
    let south_pole = mesh.add_vertex_full(Vec3::new(0.0, -radius, 0.0), -Vec3::Y, Vec2::new(0.5, 0.0));

    // Üst kutup üçgenleri
    for s in 0..segments {
        let next_s = (s + 1) % segments;
        let _ = mesh.add_face(&[north_pole, ring_vertices[0][s as usize], ring_vertices[0][next_s as usize]]);
    }

    // Ara dörtgenler (Quads)
    for r in 0..(rings - 2) {
        for s in 0..segments {
            let next_s = (s + 1) % segments;
            let v0 = ring_vertices[r as usize][s as usize];
            let v1 = ring_vertices[(r + 1) as usize][s as usize];
            let v2 = ring_vertices[(r + 1) as usize][next_s as usize];
            let v3 = ring_vertices[r as usize][next_s as usize];

            let _ = mesh.add_face(&[v0, v1, v2, v3]);
        }
    }

    // Alt kutup üçgenleri
    let last_ring = rings - 2;
    for s in 0..segments {
        let next_s = (s + 1) % segments;
        let _ = mesh.add_face(&[
            ring_vertices[last_ring as usize][next_s as usize],
            ring_vertices[last_ring as usize][s as usize],
            south_pole,
        ]);
    }

    mesh.link_twins();
    mesh.compute_vertex_normals();
    mesh
}

/// Parametrik bir Silindir (Cylinder) üretir.
pub fn create_cylinder(radius: f32, height: f32, segments: u32) -> HalfEdgeMesh {
    let mut mesh = HalfEdgeMesh::new();
    let segments = segments.max(3);
    let half_h = height * 0.5;

    let mut bottom_ring = Vec::with_capacity(segments as usize);
    let mut top_ring = Vec::with_capacity(segments as usize);

    for s in 0..segments {
        let phi = (s as f32 / segments as f32) * 2.0 * PI;
        let x = radius * phi.cos();
        let z = radius * phi.sin();

        let v_bottom = mesh.add_vertex_full(
            Vec3::new(x, -half_h, z),
            Vec3::new(x, 0.0, z).normalize_or_zero(),
            Vec2::new(s as f32 / segments as f32, 0.0),
        );
        let v_top = mesh.add_vertex_full(
            Vec3::new(x, half_h, z),
            Vec3::new(x, 0.0, z).normalize_or_zero(),
            Vec2::new(s as f32 / segments as f32, 1.0),
        );

        bottom_ring.push(v_bottom);
        top_ring.push(v_top);
    }

    // Yan yüzeyler (Quads)
    for s in 0..segments {
        let next_s = (s + 1) % segments;
        let b0 = bottom_ring[s as usize];
        let b1 = bottom_ring[next_s as usize];
        let t1 = top_ring[next_s as usize];
        let t0 = top_ring[s as usize];

        let _ = mesh.add_face(&[b0, b1, t1, t0]);
    }

    // Üst ve alt kapaklar (N-gon)
    let _ = mesh.add_face(&top_ring);
    let mut reversed_bottom = bottom_ring;
    reversed_bottom.reverse();
    let _ = mesh.add_face(&reversed_bottom);

    mesh.link_twins();
    mesh.compute_vertex_normals();
    mesh
}
