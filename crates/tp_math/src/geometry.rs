//! 3Point Geometri, Hacim Testleri ve Işın Kesişim Algoritmaları.

use crate::transform::Transform;
use glam::{Mat4, Vec3, Vec4};

/// 3D uzayda bir başlangıç noktası (origin) ve birim yön vektöründen (direction) oluşan ışın.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    /// Yeni bir ışın oluşturur. Yön vektörü otomatik olarak birim uzunluğa normalize edilir.
    #[inline]
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize_or_zero(),
        }
    }

    /// Işın boyunca $t$ parametresi mesafesindeki noktayı hesaplar: $P(t) = origin + t \cdot direction$
    #[inline]
    #[must_use]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + (self.direction * t)
    }

    /// Işının bir düzlemle kesişip kesişmediğini test eder.
    /// Kesişme varsa $t \ge 0$ parametresini döndürür.
    pub fn intersect_plane(&self, plane: &Plane) -> Option<f32> {
        let denom = plane.normal.dot(self.direction);
        if denom.abs() < 1e-6 {
            return None; // Işın düzleme paralel
        }
        let t = -(plane.normal.dot(self.origin) + plane.distance) / denom;
        if t >= 0.0 {
            Some(t)
        } else {
            None
        }
    }

    /// Işının bir Eksen Hizalı Sınırlayıcı Kutu (AABB) ile kesişimini test eder (Kayle-Slab algoritması).
    /// Kesişim varsa `(t_min, t_max)` mesafelerini döndürür.
    pub fn intersect_aabb(&self, aabb: &Aabb) -> Option<(f32, f32)> {
        let inv_dir = Vec3::select(
            self.direction.abs().cmplt(Vec3::splat(1e-8)),
            Vec3::splat(f32::INFINITY),
            Vec3::ONE / self.direction,
        );

        let t0s = (aabb.min - self.origin) * inv_dir;
        let t1s = (aabb.max - self.origin) * inv_dir;

        let tmin_v = t0s.min(t1s);
        let tmax_v = t0s.max(t1s);

        let t_enter = tmin_v.max_element();
        let t_exit = tmax_v.min_element();

        if t_exit >= t_enter && t_exit >= 0.0 {
            Some((t_enter.max(0.0), t_exit))
        } else {
            None
        }
    }

    /// Işının bir küre ile kesişim mesafesini hesaplar.
    pub fn intersect_sphere(&self, center: Vec3, radius: f32) -> Option<f32> {
        let oc = self.origin - center;
        let b = oc.dot(self.direction);
        let c = oc.length_squared() - (radius * radius);
        let discriminant = (b * b) - c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrt_disc = discriminant.sqrt();
        let t0 = -b - sqrt_disc;
        let t1 = -b + sqrt_disc;

        if t0 >= 0.0 {
            Some(t0)
        } else if t1 >= 0.0 {
            Some(t1)
        } else {
            None
        }
    }

    /// Möller–Trumbore algoritması ile ışının bir üçgen ($v_0, v_1, v_2$) ile kesişimini hesaplar.
    /// Kesişim varsa `(t, barycentric_uv)` sonucunu döndürür.
    pub fn intersect_triangle(&self, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<(f32, Vec3)> {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let h = self.direction.cross(edge2);
        let a = edge1.dot(h);

        if a.abs() < 1e-7 {
            return None; // Işın üçgen düzlemine paralel
        }

        let f = 1.0 / a;
        let s = self.origin - v0;
        let u = f * s.dot(h);

        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * self.direction.dot(q);

        if v < 0.0 || (u + v) > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);
        if t > 1e-7 {
            let w = 1.0 - u - v;
            Some((t, Vec3::new(w, u, v)))
        } else {
            None
        }
    }
}

/// Eksen Hizalı Sınırlayıcı Kutu (Axis-Aligned Bounding Box - AABB).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for Aabb {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Aabb {
    /// Boş, geçersiz başlangıç AABB'si.
    pub const EMPTY: Self = Self {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    /// Minimum ve maksimum noktalardan yeni AABB oluşturur.
    #[inline]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }

    /// Bir nokta kümesini kapsayan en küçük AABB'yi üretir.
    pub fn from_points(points: &[Vec3]) -> Self {
        let mut aabb = Self::EMPTY;
        for &pt in points {
            aabb.encapsulate_point(pt);
        }
        aabb
    }

    /// Bir noktayı kapsayacak şekilde AABB sınırlarını genişletir.
    #[inline]
    pub fn encapsulate_point(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    /// Başka bir AABB'yi kapsayacak şekilde sınırları genişletir.
    #[inline]
    pub fn encapsulate_aabb(&mut self, other: &Aabb) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    /// AABB'nin merkez noktasını döndürür.
    #[inline]
    #[must_use]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// AABB'nin boyutlarını (genişlik, yükseklik, derinlik) döndürür.
    #[inline]
    #[must_use]
    pub fn extents(&self) -> Vec3 {
        self.max - self.min
    }

    /// Yarı boyutlarını döndürür.
    #[inline]
    #[must_use]
    pub fn half_extents(&self) -> Vec3 {
        self.extents() * 0.5
    }

    /// AABB'nin geçerli olup olmadığını sınar (min <= max).
    #[inline]
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }

    /// Verilen noktanın AABB içinde olup olmadığını kontrol eder.
    #[inline]
    #[must_use]
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    /// Başka bir AABB ile çakışıp çakışmadığını test eder.
    #[inline]
    #[must_use]
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// AABB'yi verilen dönüşümle dönüştürür ve yeni eksen hizalı AABB'yi hesaplar.
    pub fn transformed(&self, transform: &Transform) -> Self {
        let center = self.center();
        let half_extents = self.half_extents();

        let mat = transform.to_matrix();
        let new_center = mat.transform_point3(center);

        // Yeni yarı boyutlar, matrisin mutlak değerlerinin çarpımı ile hesaplanır
        let abs_col0 = mat.x_axis.truncate().abs();
        let abs_col1 = mat.y_axis.truncate().abs();
        let abs_col2 = mat.z_axis.truncate().abs();

        let new_half = (abs_col0 * half_extents.x)
            + (abs_col1 * half_extents.y)
            + (abs_col2 * half_extents.z);

        Self {
            min: new_center - new_half,
            max: new_center + new_half,
        }
    }
}

/// Düzlem denklemi: $ax + by + cz + d = 0$. Normal birim vektör olmalıdır.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    /// Birim normal ve mesafeden düzlem oluşturur.
    #[inline]
    pub fn new(normal: Vec3, distance: f32) -> Self {
        let len = normal.length();
        let inv_len = if len > 1e-6 { 1.0 / len } else { 1.0 };
        Self {
            normal: normal * inv_len,
            distance: distance * inv_len,
        }
    }

    /// Düzlem üzerindeki bir nokta ve normal vektöründen oluşturur.
    #[inline]
    pub fn from_point_and_normal(point: Vec3, normal: Vec3) -> Self {
        let norm = normal.normalize_or_zero();
        Self {
            normal: norm,
            distance: -norm.dot(point),
        }
    }

    /// Bir noktanın düzleme olan işaretli mesafesini hesaplar.
    #[inline]
    #[must_use]
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

/// Kamera görüş hacmini (Frustum) temsil eden 6 düzlem kümesi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    /// Sol, Sağ, Alt, Üst, Yakın, Uzak düzlemler.
    pub planes: [Plane; 6],
}

impl Frustum {
    /// View-Projection matrisinden 6 kesme düzlemini çıkarır (Gribb-Hartmann metodu).
    pub fn from_view_projection(vp: Mat4) -> Self {
        let row0 = vp.row(0);
        let row1 = vp.row(1);
        let row2 = vp.row(2);
        let row3 = vp.row(3);

        let make_plane = |v: Vec4| Plane::new(v.truncate(), v.w);

        Self {
            planes: [
                make_plane(row3 + row0), // Sol
                make_plane(row3 - row0), // Sağ
                make_plane(row3 + row1), // Alt
                make_plane(row3 - row1), // Üst
                make_plane(row2),        // Yakın (Vulkan 0..1 Z derinliği)
                make_plane(row3 - row2), // Uzak
            ],
        }
    }

    /// Bir AABB'nin görüş hacmi içinde veya kesişiyor olup olmadığını test eder (Frustum Culling).
    pub fn intersects_aabb(&self, aabb: &Aabb) -> bool {
        for plane in &self.planes {
            // Düzlemin normal yönüne göre en pozitif köşeyi (p-vertex) seç
            let p = Vec3::new(
                if plane.normal.x > 0.0 { aabb.max.x } else { aabb.min.x },
                if plane.normal.y > 0.0 { aabb.max.y } else { aabb.min.y },
                if plane.normal.z > 0.0 { aabb.max.z } else { aabb.min.z },
            );

            if plane.distance_to_point(p) < 0.0 {
                return false; // Kutu düzlemin tamamen dışındadır
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_plane_intersection() {
        let ray = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y);
        let plane = Plane::from_point_and_normal(Vec3::ZERO, Vec3::Y);
        let hit = ray.intersect_plane(&plane);
        assert_eq!(hit, Some(10.0));
    }

    #[test]
    fn test_ray_aabb_intersection() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, -10.0), Vec3::Z);
        let aabb = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        let hit = ray.intersect_aabb(&aabb);
        assert!(hit.is_some());
        let (t_min, t_max) = hit.unwrap();
        assert!((t_min - 9.0).abs() < 1e-4);
        assert!((t_max - 11.0).abs() < 1e-4);
    }

    #[test]
    fn test_ray_triangle_moller_trumbore() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, -5.0), Vec3::Z);
        let v0 = Vec3::new(-1.0, -1.0, 0.0);
        let v1 = Vec3::new(1.0, -1.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        let hit = ray.intersect_triangle(v0, v1, v2);
        assert!(hit.is_some());
        let (t, _bary) = hit.unwrap();
        assert!((t - 5.0).abs() < 1e-4);
    }
}

