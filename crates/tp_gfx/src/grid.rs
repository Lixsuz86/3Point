//! 3Point Sonsuz Viewport Zemin Izgarası (Infinite Grid) ve Koordinat Eksenleri.

use crate::{
    error::GfxError,
    pipeline::GraphicsPipelineBuilder,
    shader::{compile_wgsl_to_spirv, create_shader_module},
};
use ash::{vk, Device};
use glam::{Mat4, Vec3};

/// Izgara renderı için GPU'ya aktarılan Push Constant yapısı (std140/std430 hizalamalı).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridPushConstants {
    pub inv_view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub near: f32,
    pub far: f32,
    pub _padding: [f32; 3],
}

/// Sonsuz zemin ızgarası shader'ı (WGSL formatında).
const GRID_WGSL: &str = r#"
struct PushConstants {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    near: f32,
    far: f32,
};

var<push_constant> pc: PushConstants;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) near_point: vec3<f32>,
    @location(1) far_point: vec3<f32>,
};

// Tam ekran 6 köşe (0 Vertex Buffer gerektirir)
@vertex
fn main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );

    let p = pos[vertex_index];

    // Kamera ışınlarını Near (Z=0.0) ve Far (Z=1.0) düzleminden ters projeksiyon ile aç
    let p_near_h = pc.inv_view_proj * vec4<f32>(p.x, p.y, 0.0, 1.0);
    let p_far_h  = pc.inv_view_proj * vec4<f32>(p.x, p.y, 1.0, 1.0);

    var out: VertexOutput;
    out.clip_position = vec4<f32>(p.x, p.y, 0.0, 1.0);
    out.near_point = p_near_h.xyz / p_near_h.w;
    out.far_point  = p_far_h.xyz  / p_far_h.w;
    return out;
}

@fragment
fn main_fs(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = -in.near_point.y / (in.far_point.y - in.near_point.y);
    if (t <= 0.0) {
        discard;
    }

    let pos = in.near_point + t * (in.far_point - in.near_point);
    let coord = pos.xz;

    // Analitik türev tabanlı pürüzsüz çizgi kalınlığı
    let derivative = fwidth(coord);
    let grid_1m = abs(fract(coord - 0.5) - 0.5) / derivative;
    let line_1m = min(grid_1m.x, grid_1m.y);

    let grid_10cm = abs(fract(coord * 10.0 - 0.5) - 0.5) / (derivative * 10.0);
    let line_10cm = min(grid_10cm.x, grid_10cm.y);

    let min_dist = min(line_1m, line_10cm);
    var color = vec4<f32>(0.22, 0.23, 0.26, 1.0 - min(min_dist, 1.0));

    // Ana 1 metre çizgileri daha belirgin yap
    if (line_1m < 1.0) {
        color = vec4<f32>(0.35, 0.36, 0.40, 1.0 - line_1m);
    }

    // Koordinat Eksenleri: X Ekseni (Kırmızı: Z=0 civarı), Z Ekseni (Mavi: X=0 civarı)
    let axis_width = derivative * 1.5;
    if (abs(pos.z) < axis_width.y) {
        color = vec4<f32>(0.85, 0.22, 0.25, 0.9); // Kırmızı (X Ekseni)
    }
    if (abs(pos.x) < axis_width.x) {
        color = vec4<f32>(0.22, 0.45, 0.85, 0.9); // Mavi (Z Ekseni)
    }

    // Mesafeye bağlı yumuşak sönümleme (Falloff)
    let dist = length(pos.xz - pc.camera_pos.xz);
    let fade = 1.0 - smoothstep(10.0, 80.0, dist);
    color.a = color.a * fade;

    if (color.a < 0.01) {
        discard;
    }

    return color;
}
"#;

/// Infinite Viewport Grid yöneticisi.
pub struct VulkanGrid {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
}

impl VulkanGrid {
    /// Izgara render boru hattını oluşturur.
    pub fn new(device: &Device, render_pass: vk::RenderPass) -> Result<Self, GfxError> {
        let vs_spv = compile_wgsl_to_spirv(GRID_WGSL, "vertex")?;
        let fs_spv = compile_wgsl_to_spirv(GRID_WGSL, "fragment")?;

        let vs_module = create_shader_module(device, &vs_spv)?;
        let fs_module = create_shader_module(device, &fs_spv)?;

        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<GridPushConstants>() as u32);

        let (pipeline, layout) = GraphicsPipelineBuilder::new(device, vs_module, fs_module, render_pass)
            .with_topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .with_cull_mode(vk::CullModeFlags::NONE)
            .with_depth(false, false) // Grid diğer objeleri örtmemesi için derinlik yazmaz
            .with_blending(true) // Şeffaf zemin çizgileri için Alpha Blending
            .with_push_constant(push_range)
            .build()?;

        // SAFETY: ShaderModule'lar pipeline içine fırınlandıktan sonra yok edilebilir.
        unsafe {
            device.destroy_shader_module(vs_module, None);
            device.destroy_shader_module(fs_module, None);
        }

        tracing::info!("3Point Infinite Grid Pipeline başarıyla derlendi.");

        Ok(Self { pipeline, layout })
    }

    /// Izgarayı komut tamponuna çizer.
    pub fn draw(
        &self,
        device: &Device,
        cmd: vk::CommandBuffer,
        inv_view_proj: Mat4,
        camera_pos: Vec3,
        near: f32,
        far: f32,
    ) {
        let pc = GridPushConstants {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.to_array(),
            near,
            far,
            _padding: [0.0; 3],
        };

        // SAFETY:
        // 1. Pipeline bind edilir.
        // 2. Push constants verisi kopyalanır.
        // 3. 6 köşe (vertex) procedural olarak çizilir.
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            let pc_bytes = bytemuck::bytes_of(&pc);
            device.cmd_push_constants(
                cmd,
                self.layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                pc_bytes,
            );

            device.cmd_draw(cmd, 6, 1, 0, 0);
        }
    }

    /// Kaynakları serbest bırakır.
    pub fn destroy(&mut self, device: &Device) {
        // SAFETY: vkDestroyPipeline ve vkDestroyPipelineLayout.
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

