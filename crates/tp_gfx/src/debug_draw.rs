//! 3Point 3D Tel Kafes (Wireframe), Koordinat Eksenleri ve Hata Ayıklama Çizim Hattı (`DebugDraw`).

use crate::{
    buffer::GpuBuffer,
    error::GfxError,
    pipeline::GraphicsPipelineBuilder,
    shader::{compile_wgsl_to_spirv, create_shader_module},
};
use ash::{vk, Device};
use glam::{Mat4, Vec3, Vec4};
use gpu_allocator::vulkan::Allocator;
use tp_math::{Aabb, Ray, Transform};

const MAX_DEBUG_LINES: usize = 16_384;
const MAX_DEBUG_VERTICES: usize = MAX_DEBUG_LINES * 2;

/// Çizgi köşesi yapısı (Konum + RGBA Renk).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl LineVertex {
    /// Vertex layout tanımlayıcılarını döner.
    pub fn layout() -> (
        Vec<vk::VertexInputBindingDescription>,
        Vec<vk::VertexInputAttributeDescription>,
    ) {
        let binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX);

        let attr_pos = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0);

        let attr_color = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(12);

        (vec![binding], vec![attr_pos, attr_color])
    }
}

/// Çizgi shader'ı (WGSL).
const LINE_WGSL: &str = r#"
struct PushConstants {
    view_proj: mat4x4<f32>,
};

var<push_constant> pc: PushConstants;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = pc.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn main_fs(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// 3D Çizgi ve Tel Kafes yöneticisi.
pub struct VulkanDebugDraw {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub vertex_buffer: GpuBuffer,
    pub vertices: Vec<LineVertex>,
}

impl VulkanDebugDraw {
    /// Yeni bir hata ayıklama çizim boru hattı başlatır.
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        render_pass: vk::RenderPass,
    ) -> Result<Self, GfxError> {
        let vs_spv = compile_wgsl_to_spirv(LINE_WGSL, "vertex")?;
        let fs_spv = compile_wgsl_to_spirv(LINE_WGSL, "fragment")?;

        let vs_module = create_shader_module(device, &vs_spv)?;
        let fs_module = create_shader_module(device, &fs_spv)?;

        let (bindings, attrs) = LineVertex::layout();

        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<Mat4>() as u32);

        let (pipeline, layout) = GraphicsPipelineBuilder::new(device, vs_module, fs_module, render_pass)
            .with_vertex_layout(bindings, attrs)
            .with_topology(vk::PrimitiveTopology::LINE_LIST)
            .with_cull_mode(vk::CullModeFlags::NONE)
            .with_depth(true, false)
            .with_blending(true)
            .with_push_constant(push_range)
            .build()?;

        // SAFETY: Modüller pipeline'a aktarıldıktan sonra temizlenir.
        unsafe {
            device.destroy_shader_module(vs_module, None);
            device.destroy_shader_module(fs_module, None);
        }

        let buffer_size = (MAX_DEBUG_VERTICES * std::mem::size_of::<LineVertex>()) as u64;
        let vertex_buffer = GpuBuffer::new_vertex_buffer(device, allocator, buffer_size)?;

        tracing::info!("3Point DebugDraw Çizgi Boru Hattı oluşturuldu.");

        Ok(Self {
            pipeline,
            layout,
            vertex_buffer,
            vertices: Vec::with_capacity(2048),
        })
    }

    /// İki nokta arasına 3B renkli bir çizgi ekler.
    pub fn draw_line(&mut self, p0: Vec3, p1: Vec3, color: Vec4) {
        if self.vertices.len() + 2 > MAX_DEBUG_VERTICES {
            return;
        }
        let col = color.to_array();
        self.vertices.push(LineVertex {
            position: p0.to_array(),
            color: col,
        });
        self.vertices.push(LineVertex {
            position: p1.to_array(),
            color: col,
        });
    }

    /// Bir eksen hizalı sınırlayıcı kutunun (AABB) 12 kenarını tel kafes olarak çizer.
    pub fn draw_aabb(&mut self, aabb: &Aabb, color: Vec4) {
        let (min, max) = (aabb.min, aabb.max);

        // Alt yüzey
        self.draw_line(Vec3::new(min.x, min.y, min.z), Vec3::new(max.x, min.y, min.z), color);
        self.draw_line(Vec3::new(max.x, min.y, min.z), Vec3::new(max.x, min.y, max.z), color);
        self.draw_line(Vec3::new(max.x, min.y, max.z), Vec3::new(min.x, min.y, max.z), color);
        self.draw_line(Vec3::new(min.x, min.y, max.z), Vec3::new(min.x, min.y, min.z), color);

        // Üst yüzey
        self.draw_line(Vec3::new(min.x, max.y, min.z), Vec3::new(max.x, max.y, min.z), color);
        self.draw_line(Vec3::new(max.x, max.y, min.z), Vec3::new(max.x, max.y, max.z), color);
        self.draw_line(Vec3::new(max.x, max.y, max.z), Vec3::new(min.x, max.y, max.z), color);
        self.draw_line(Vec3::new(min.x, max.y, max.z), Vec3::new(min.x, max.y, min.z), color);

        // Düşey sütunlar
        self.draw_line(Vec3::new(min.x, min.y, min.z), Vec3::new(min.x, max.y, min.z), color);
        self.draw_line(Vec3::new(max.x, min.y, min.z), Vec3::new(max.x, max.y, min.z), color);
        self.draw_line(Vec3::new(max.x, min.y, max.z), Vec3::new(max.x, max.y, max.z), color);
        self.draw_line(Vec3::new(min.x, min.y, max.z), Vec3::new(min.x, max.y, max.z), color);
    }

    /// 3B Koordinat eksenlerini (Kırmızı: X, Yeşil: Y, Mavi: Z) çizer.
    pub fn draw_axes(&mut self, transform: &Transform, size: f32) {
        let origin = transform.translation;
        let x_axis = origin + transform.transform_vector(Vec3::X * size);
        let y_axis = origin + transform.transform_vector(Vec3::Y * size);
        let z_axis = origin + transform.transform_vector(Vec3::Z * size);

        self.draw_line(origin, x_axis, Vec4::new(1.0, 0.1, 0.1, 1.0)); // X: Kırmızı
        self.draw_line(origin, y_axis, Vec4::new(0.1, 1.0, 0.1, 1.0)); // Y: Yeşil
        self.draw_line(origin, z_axis, Vec4::new(0.1, 0.4, 1.0, 1.0)); // Z: Mavi
    }

    /// 3B Çember çizer.
    pub fn draw_circle(&mut self, center: Vec3, normal: Vec3, radius: f32, segments: usize, color: Vec4) {
        let n = normal.normalize_or_zero();
        let up = if n.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
        let u = n.cross(up).normalize();
        let v = n.cross(u).normalize();

        let step = std::f32::consts::TAU / (segments as f32);
        for i in 0..segments {
            let theta0 = (i as f32) * step;
            let theta1 = ((i + 1) as f32) * step;

            let p0 = center + (u * theta0.cos() + v * theta0.sin()) * radius;
            let p1 = center + (u * theta1.cos() + v * theta1.sin()) * radius;
            self.draw_line(p0, p1, color);
        }
    }

    /// Kamera görselleştirme tel kafesi (Frustum ve gövde).
    pub fn draw_camera_gizmo(&mut self, position: Vec3, target: Vec3, size: f32, color: Vec4) {
        let forward = (target - position).normalize_or_zero();
        let fwd = if forward.length_squared() < 0.001 { Vec3::NEG_Z } else { forward };
        let right = fwd.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(fwd).normalize_or_zero();

        // Kamera arkası gövde kutusu
        let half_w = right * (size * 0.35);
        let half_h = up * (size * 0.25);
        let back_center = position - fwd * (size * 0.4);

        let c0 = back_center - half_w - half_h;
        let c1 = back_center + half_w - half_h;
        let c2 = back_center + half_w + half_h;
        let c3 = back_center - half_w + half_h;

        self.draw_line(c0, c1, color);
        self.draw_line(c1, c2, color);
        self.draw_line(c2, c3, color);
        self.draw_line(c3, c0, color);

        // Vizör piramidi (Ön düzlem)
        let front_center = position + fwd * (size * 0.8);
        let fw_half = right * (size * 0.6);
        let fh_half = up * (size * 0.4);

        let f0 = front_center - fw_half - fh_half;
        let f1 = front_center + fw_half - fh_half;
        let f2 = front_center + fw_half + fh_half;
        let f3 = front_center - fw_half + fh_half;

        self.draw_line(f0, f1, color);
        self.draw_line(f1, f2, color);
        self.draw_line(f2, f3, color);
        self.draw_line(f3, f0, color);

        // Gövdeden vizöre çizgiler
        self.draw_line(position, f0, color);
        self.draw_line(position, f1, color);
        self.draw_line(position, f2, color);
        self.draw_line(position, f3, color);

        // Üst "Yukarı" gösterge üçgeni
        let top_marker = back_center + half_h + up * (size * 0.2);
        self.draw_line(c2, top_marker, color);
        self.draw_line(c3, top_marker, color);
    }

    /// Genel aydınlatma / Güneş simgesi (Sun Disc & Işınlar).
    pub fn draw_light_gizmo(&mut self, position: Vec3, direction: Vec3, size: f32, color: Vec4) {
        // Güneş çekirdeği çemberleri (3 Eksen)
        self.draw_circle(position, Vec3::Y, size * 0.4, 16, color);
        self.draw_circle(position, Vec3::X, size * 0.4, 16, color);
        self.draw_circle(position, Vec3::Z, size * 0.4, 16, color);

        // Çapraz parlama yıldızı
        self.draw_line(position - Vec3::X * (size * 0.6), position + Vec3::X * (size * 0.6), color);
        self.draw_line(position - Vec3::Y * (size * 0.6), position + Vec3::Y * (size * 0.6), color);
        self.draw_line(position - Vec3::Z * (size * 0.6), position + Vec3::Z * (size * 0.6), color);

        // Güneş yönlü ışınları (Directional Rays)
        let dir = direction.normalize_or_zero();
        let dir = if dir.length_squared() < 0.001 { Vec3::new(-0.4, -0.8, -0.4).normalize() } else { dir };
        let ray_len = size * 1.5;

        let offsets = [
            Vec3::ZERO,
            Vec3::new(size * 0.5, 0.0, 0.0),
            Vec3::new(-size * 0.5, 0.0, 0.0),
            Vec3::new(0.0, 0.0, size * 0.5),
            Vec3::new(0.0, 0.0, -size * 0.5),
        ];

        let ray_color = Vec4::new(color.x, color.y, color.z, 0.7);
        for off in offsets {
            let start = position + off;
            let end = start + dir * ray_len;
            self.draw_line(start, end, ray_color);
            // Işın ucu ok göstergesi
            self.draw_line(end, end - dir * (size * 0.2) + Vec3::Y * (size * 0.1), ray_color);
        }
    }

    /// 3B Taşıma Gizmosu (Translate Handles).
    pub fn draw_translate_gizmo(&mut self, position: Vec3, size: f32) {
        let x_end = position + Vec3::X * size;
        let y_end = position + Vec3::Y * size;
        let z_end = position + Vec3::Z * size;

        let red = Vec4::new(0.95, 0.22, 0.25, 1.0);
        let green = Vec4::new(0.25, 0.90, 0.30, 1.0);
        let blue = Vec4::new(0.25, 0.45, 0.98, 1.0);

        // Ana kollar
        self.draw_line(position, x_end, red);
        self.draw_line(position, y_end, green);
        self.draw_line(position, z_end, blue);

        // Eksen ok uçları
        let arrow_size = size * 0.12;
        self.draw_line(x_end, x_end - Vec3::X * arrow_size + Vec3::Y * arrow_size, red);
        self.draw_line(x_end, x_end - Vec3::X * arrow_size - Vec3::Y * arrow_size, red);
        self.draw_line(y_end, y_end - Vec3::Y * arrow_size + Vec3::X * arrow_size, green);
        self.draw_line(y_end, y_end - Vec3::Y * arrow_size - Vec3::X * arrow_size, green);
        self.draw_line(z_end, z_end - Vec3::Z * arrow_size + Vec3::Y * arrow_size, blue);
        self.draw_line(z_end, z_end - Vec3::Z * arrow_size - Vec3::Y * arrow_size, blue);

        // Düzlemsel tutamaçlar (Küçük kareler: XY, XZ, YZ)
        let plane_size = size * 0.3;
        let xy_corner = position + (Vec3::X + Vec3::Y) * plane_size;
        let xz_corner = position + (Vec3::X + Vec3::Z) * plane_size;
        let yz_corner = position + (Vec3::Y + Vec3::Z) * plane_size;

        self.draw_line(position + Vec3::X * plane_size, xy_corner, red);
        self.draw_line(position + Vec3::Y * plane_size, xy_corner, green);

        self.draw_line(position + Vec3::X * plane_size, xz_corner, red);
        self.draw_line(position + Vec3::Z * plane_size, xz_corner, blue);

        self.draw_line(position + Vec3::Y * plane_size, yz_corner, green);
        self.draw_line(position + Vec3::Z * plane_size, yz_corner, blue);
    }

    /// 3B Döndürme Gizmosu (Rotation Rings).
    pub fn draw_rotate_gizmo(&mut self, position: Vec3, size: f32) {
        let red = Vec4::new(0.95, 0.22, 0.25, 0.9);
        let green = Vec4::new(0.25, 0.90, 0.30, 0.9);
        let blue = Vec4::new(0.25, 0.45, 0.98, 0.9);

        // X, Y, Z eksenleri etrafındaki döndürme halkaları
        self.draw_circle(position, Vec3::X, size, 32, red);
        self.draw_circle(position, Vec3::Y, size, 32, green);
        self.draw_circle(position, Vec3::Z, size, 32, blue);
    }

    /// 3B Ölçekleme Gizmosu (Scale Handles).
    pub fn draw_scale_gizmo(&mut self, position: Vec3, size: f32) {
        let x_end = position + Vec3::X * size;
        let y_end = position + Vec3::Y * size;
        let z_end = position + Vec3::Z * size;

        let red = Vec4::new(0.95, 0.22, 0.25, 1.0);
        let green = Vec4::new(0.25, 0.90, 0.30, 1.0);
        let blue = Vec4::new(0.25, 0.45, 0.98, 1.0);

        self.draw_line(position, x_end, red);
        self.draw_line(position, y_end, green);
        self.draw_line(position, z_end, blue);

        // Uçlardaki ölçekleme kutucukları
        let box_size = size * 0.08;
        let aabb_x = Aabb::new(x_end - Vec3::splat(box_size), x_end + Vec3::splat(box_size));
        let aabb_y = Aabb::new(y_end - Vec3::splat(box_size), y_end + Vec3::splat(box_size));
        let aabb_z = Aabb::new(z_end - Vec3::splat(box_size), z_end + Vec3::splat(box_size));

        self.draw_aabb(&aabb_x, red);
        self.draw_aabb(&aabb_y, green);
        self.draw_aabb(&aabb_z, blue);

        // Eşit/Tüm yönlü ölçekleme dış çemberi
        self.draw_circle(position, Vec3::Y, size * 1.15, 32, Vec4::new(0.9, 0.9, 0.9, 0.5));
    }

    /// Bir ışını (Ray) yönü boyunca görselleştirir.
    pub fn draw_ray(&mut self, ray: &Ray, length: f32, color: Vec4) {
        self.draw_line(ray.origin, ray.at(length), color);
    }

    /// Biriken tüm çizgileri GPU tamponuna yazar ve render komutunu gönderir.
    pub fn flush(&mut self, device: &Device, cmd: vk::CommandBuffer, view_proj: Mat4) {
        if self.vertices.is_empty() {
            return;
        }

        let count = self.vertices.len() as u32;
        if let Err(e) = self.vertex_buffer.write_data(&self.vertices) {
            tracing::error!("DebugDraw köşe aktarım hatası: {e}");
            self.vertices.clear();
            return;
        }

        // SAFETY: Pipeline ve Vertex buffer bind edilerek çizim tetiklenir.
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            let vp_cols = view_proj.to_cols_array_2d();
            let vp_bytes = bytemuck::bytes_of(&vp_cols);
            device.cmd_push_constants(
                cmd,
                self.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                vp_bytes,
            );

            device.cmd_bind_vertex_buffers(cmd, 0, &[self.vertex_buffer.buffer], &[0]);
            device.cmd_draw(cmd, count, 1, 0, 0);
        }

        self.vertices.clear();
    }

    /// Kaynakları serbest bırakır.
    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        self.vertex_buffer.destroy(device, allocator);
        // SAFETY: vkDestroyPipeline ve vkDestroyPipelineLayout.
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}
