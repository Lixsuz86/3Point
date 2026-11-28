//! 3Point PBR (Physically-Based Rendering) Materyal Sistemi.

use glam::{Vec3, Vec4};

/// Fiziksel tabanlı Cook-Torrance metalik-pürüzlülük materyali.
#[derive(Debug, Clone, PartialEq)]
pub struct PbrMaterial {
    pub name: String,
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: Vec3,
    pub ao: f32,
    pub ior: f32,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            name: "Default Material".to_string(),
            base_color: Vec4::new(0.82, 0.84, 0.88, 1.0),
            metallic: 0.1,
            roughness: 0.5,
            emissive: Vec3::ZERO,
            ao: 1.0,
            ior: 1.5,
        }
    }
}

impl PbrMaterial {
    /// Yeni özel bir PBR materyali oluşturur.
    pub fn new(name: impl Into<String>, base_color: Vec4, metallic: f32, roughness: f32) -> Self {
        Self {
            name: name.into(),
            base_color,
            metallic: metallic.clamp(0.0, 1.0),
            roughness: roughness.clamp(0.04, 1.0),
            emissive: Vec3::ZERO,
            ao: 1.0,
            ior: 1.5,
        }
    }

    /// Saf Altın (Gold) PBR materyal şablonu.
    pub fn gold() -> Self {
        Self {
            name: "Altın (Gold)".to_string(),
            base_color: Vec4::new(1.0, 0.85, 0.57, 1.0),
            metallic: 1.0,
            roughness: 0.25,
            emissive: Vec3::ZERO,
            ao: 1.0,
            ior: 0.47,
        }
    }

    /// Bakır (Copper) PBR materyal şablonu.
    pub fn copper() -> Self {
        Self {
            name: "Bakır (Copper)".to_string(),
            base_color: Vec4::new(0.95, 0.64, 0.54, 1.0),
            metallic: 0.95,
            roughness: 0.3,
            emissive: Vec3::ZERO,
            ao: 1.0,
            ior: 1.1,
        }
    }

    /// Krom / Ayna (Chrome) PBR materyal şablonu.
    pub fn chrome() -> Self {
        Self {
            name: "Krom (Chrome)".to_string(),
            base_color: Vec4::new(0.95, 0.95, 0.95, 1.0),
            metallic: 1.0,
            roughness: 0.05,
            emissive: Vec3::ZERO,
            ao: 1.0,
            ior: 2.9,
        }
    }

    /// Renkli Parlak Plastik materyal şablonu.
    pub fn plastic(color: Vec4) -> Self {
        Self {
            name: "Plastik (Plastic)".to_string(),
            base_color: color,
            metallic: 0.0,
            roughness: 0.2,
            emissive: Vec3::ZERO,
            ao: 1.0,
            ior: 1.46,
        }
    }

    /// Mat Kauçuk / Lastik (Rubber) materyal şablonu.
    pub fn rubber() -> Self {
        Self {
            name: "Kauçuk (Rubber)".to_string(),
            base_color: Vec4::new(0.15, 0.15, 0.15, 1.0),
            metallic: 0.0,
            roughness: 0.9,
            emissive: Vec3::ZERO,
            ao: 1.0,
            ior: 1.52,
        }
    }

    /// Modelleme Kili (Clay / Studio Gray) materyal şablonu.
    pub fn clay() -> Self {
        Self {
            name: "Kil (Clay)".to_string(),
            base_color: Vec4::new(0.72, 0.70, 0.68, 1.0),
            metallic: 0.0,
            roughness: 0.85,
            emissive: Vec3::ZERO,
            ao: 1.0,
            ior: 1.5,
        }
    }

    /// Işıma yayan (Emissive) neon materyali.
    pub fn emissive(color: Vec3, intensity: f32) -> Self {
        Self {
            name: "Neon / Işık".to_string(),
            base_color: Vec4::new(color.x, color.y, color.z, 1.0),
            metallic: 0.0,
            roughness: 0.5,
            emissive: color * intensity,
            ao: 1.0,
            ior: 1.0,
        }
    }

    /// Basit metin formatında serileştirir (`.3pmaterial`).
    pub fn serialize(&self) -> String {
        format!(
            "name={}\ncolor={},{},{},{}\nmetallic={}\nroughness={}\nemissive={},{},{}\nao={}\nior={}\n",
            self.name,
            self.base_color.x, self.base_color.y, self.base_color.z, self.base_color.w,
            self.metallic,
            self.roughness,
            self.emissive.x, self.emissive.y, self.emissive.z,
            self.ao,
            self.ior
        )
    }

    /// Basit metin formatından ayrıştırır (`.3pmaterial`).
    pub fn deserialize(data: &str) -> Option<Self> {
        let mut mat = Self::default();
        for line in data.lines() {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0].trim();
            let val = parts[1].trim();
            match key {
                "name" => mat.name = val.to_string(),
                "color" => {
                    let nums: Vec<f32> = val.split(',').filter_map(|s| s.parse().ok()).collect();
                    if nums.len() == 4 {
                        mat.base_color = Vec4::new(nums[0], nums[1], nums[2], nums[3]);
                    }
                }
                "metallic" => {
                    if let Ok(v) = val.parse() {
                        mat.metallic = v;
                    }
                }
                "roughness" => {
                    if let Ok(v) = val.parse() {
                        mat.roughness = v;
                    }
                }
                "emissive" => {
                    let nums: Vec<f32> = val.split(',').filter_map(|s| s.parse().ok()).collect();
                    if nums.len() == 3 {
                        mat.emissive = Vec3::new(nums[0], nums[1], nums[2]);
                    }
                }
                "ao" => {
                    if let Ok(v) = val.parse() {
                        mat.ao = v;
                    }
                }
                "ior" => {
                    if let Ok(v) = val.parse() {
                        mat.ior = v;
                    }
                }
                _ => {}
            }
        }
        Some(mat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_presets() {
        let gold = PbrMaterial::gold();
        assert_eq!(gold.metallic, 1.0);
        assert!(gold.roughness < 0.5);

        let plastic = PbrMaterial::plastic(Vec4::ONE);
        assert_eq!(plastic.metallic, 0.0);
    }

    #[test]
    fn test_material_serialization_roundtrip() {
        let original = PbrMaterial::copper();
        let serialized = original.serialize();
        let loaded = PbrMaterial::deserialize(&serialized).expect("Parse başarısız");
        assert_eq!(original.name, loaded.name);
        assert_eq!(original.metallic, loaded.metallic);
        assert_eq!(original.roughness, loaded.roughness);
    }
}
