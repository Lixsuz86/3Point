//! 3Point Vulkan `egui` Render Entegrasyonu (`EguiVulkanRenderer`).

use ash::{vk, Device};
use egui::{epaint::Primitive, ClippedPrimitive};
use gpu_allocator::vulkan::Allocator;
use tp_gfx::{
    buffer::GpuBuffer,
    error::GfxError,
    pipeline::GraphicsPipelineBuilder,
    shader::{compile_wgsl_to_spirv, create_shader_module},
};

/// Egui köşe düzeni (egui::epaint::Vertex ile bayt düzeyinde %100 uyumlu).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EguiGpuVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

/// Egui Push Constant yapısı.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EguiPushConstants {
    pub screen_size: [f32; 2],
}

const EGUI_WGSL: &str = r#"
struct PushConstants {
    screen_size: vec2<f32>,
};

var<push_constant> pc: PushConstants;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let x = (in.pos.x / pc.screen_size.x) * 2.0 - 1.0;
    let y = (in.pos.y / pc.screen_size.y) * 2.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn main_fs(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// Vulkan üzerinde egui çizimlerini yürüten donanım hızlandırmalı UI motoru.
pub struct EguiVulkanRenderer {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub vertex_buffer: GpuBuffer,
    pub index_buffer: GpuBuffer,
}

impl EguiVulkanRenderer {
    /// Yeni bir EguiVulkanRenderer oluşturur.
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        render_pass: vk::RenderPass,
    ) -> Result<Self, GfxError> {
        let vs_spv = compile_wgsl_to_spirv(EGUI_WGSL, "vertex")?;
        let fs_spv = compile_wgsl_to_spirv(EGUI_WGSL, "fragment")?;

        let vs_module = create_shader_module(device, &vs_spv)?;
        let fs_module = create_shader_module(device, &fs_spv)?;

        // Köşe düzeni tanımları
        let binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<EguiGpuVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX);

        let attr_pos = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0);

        let attr_uv = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(8);

        let attr_color = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(2)
            .format(vk::Format::R8G8B8A8_UNORM)
            .offset(16);

        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<EguiPushConstants>() as u32);

        let (pipeline, layout) = GraphicsPipelineBuilder::new(device, vs_module, fs_module, render_pass)
            .with_vertex_layout(vec![binding], vec![attr_pos, attr_uv, attr_color])
            .with_topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .with_cull_mode(vk::CullModeFlags::NONE)
            .with_depth(false, false) // UI daima en üst katmandadır
            .with_blending(true) // Şeffaf UI pencereleri için Alpha Blending
            .with_push_constant(push_range)
            .build()?;

        // SAFETY: Modüller temizlenir.
        unsafe {
            device.destroy_shader_module(vs_module, None);
            device.destroy_shader_module(fs_module, None);
        }

        // Dinamik UI köşe ve indeks tamponları (Her kare güncellenebilir)
        let max_vertices = 65_536;
        let max_indices = 131_072;
        let vb_size = (max_vertices * std::mem::size_of::<EguiGpuVertex>()) as u64;
        let ib_size = (max_indices * std::mem::size_of::<u32>()) as u64;

        let vertex_buffer = GpuBuffer::new_vertex_buffer(device, allocator, vb_size)?;
        let index_buffer = GpuBuffer::new_index_buffer(device, allocator, ib_size)?;

        tracing::info!("Egui Vulkan Renderer başarıyla başlatıldı.");

        Ok(Self {
            pipeline,
            layout,
            vertex_buffer,
            index_buffer,
        })
    }

    /// Egui tarafından üretilen çizim ilkellerini (ClippedPrimitives) Vulkan komut tamponuna çizer.
    pub fn render(
        &mut self,
        device: &Device,
        cmd: vk::CommandBuffer,
        screen_width: f32,
        screen_height: f32,
        pixels_per_point: f32,
        primitives: &[ClippedPrimitive],
    ) -> Result<(), GfxError> {
        if primitives.is_empty() || screen_width <= 0.0 || screen_height <= 0.0 {
            return Ok(());
        }

        let mut all_vertices: Vec<EguiGpuVertex> = Vec::with_capacity(4096);
        let mut all_indices: Vec<u32> = Vec::with_capacity(8192);

        for clipped in primitives {
            if let Primitive::Mesh(mesh) = &clipped.primitive {
                let base_vertex = all_vertices.len() as u32;

                for v in &mesh.vertices {
                    all_vertices.push(EguiGpuVertex {
                        pos: [v.pos.x, v.pos.y],
                        uv: [v.uv.x, v.uv.y],
                        color: [v.color.r(), v.color.g(), v.color.b(), v.color.a()],
                    });
                }

                for &idx in &mesh.indices {
                    all_indices.push(base_vertex + idx);
                }
            }
        }

        if all_vertices.is_empty() || all_indices.is_empty() {
            return Ok(());
        }

        self.vertex_buffer.write_data(&all_vertices)?;
        self.index_buffer.write_data(&all_indices)?;

        let pc = EguiPushConstants {
            screen_size: [screen_width, screen_height],
        };

        // SAFETY: Pipeline, Push Constants ve Scissor ile çizim tetiklenir.
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            let pc_bytes = bytemuck::bytes_of(&pc);
            device.cmd_push_constants(
                cmd,
                self.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                pc_bytes,
            );

            device.cmd_bind_vertex_buffers(cmd, 0, &[self.vertex_buffer.buffer], &[0]);
            device.cmd_bind_index_buffer(cmd, self.index_buffer.buffer, 0, vk::IndexType::UINT32);

            let mut index_offset = 0;
            for clipped in primitives {
                if let Primitive::Mesh(mesh) = &clipped.primitive {
                    if mesh.indices.is_empty() {
                        continue;
                    }

                    // Kırpma kutusunu (Scissor Rect) piksel uzayına dönüştür
                    let clip_min_x = (clipped.clip_rect.min.x * pixels_per_point).max(0.0) as i32;
                    let clip_min_y = (clipped.clip_rect.min.y * pixels_per_point).max(0.0) as i32;
                    let clip_max_x = (clipped.clip_rect.max.x * pixels_per_point).min(screen_width * pixels_per_point) as i32;
                    let clip_max_y = (clipped.clip_rect.max.y * pixels_per_point).min(screen_height * pixels_per_point) as i32;

                    let scissor_w = (clip_max_x - clip_min_x).max(1) as u32;
                    let scissor_h = (clip_max_y - clip_min_y).max(1) as u32;

                    let scissor = vk::Rect2D {
                        offset: vk::Offset2D { x: clip_min_x, y: clip_min_y },
                        extent: vk::Extent2D { width: scissor_w, height: scissor_h },
                    };

                    device.cmd_set_scissor(cmd, 0, &[scissor]);
                    device.cmd_draw_indexed(cmd, mesh.indices.len() as u32, 1, index_offset, 0, 0);

                    index_offset += mesh.indices.len() as u32;
                }
            }
        }

        Ok(())
    }

    /// Kaynakları serbest bırakır.
    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        self.vertex_buffer.destroy(device, allocator);
        self.index_buffer.destroy(device, allocator);
        // SAFETY: Pipeline ve layout yok edilir.
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}
