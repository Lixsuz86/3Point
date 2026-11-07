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
