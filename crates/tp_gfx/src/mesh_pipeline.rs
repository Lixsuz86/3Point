//! 3Point 3D Mesh Render Boru Hattı (MatCap Modelleme ve PBR Desteği).

use crate::{
    error::GfxError,
    pipeline::GraphicsPipelineBuilder,
    shader::{compile_wgsl_to_spirv, create_shader_module},
};
use ash::{vk, Device};
use glam::{Mat4, Vec3, Vec4};

/// 3D Mesh köşe veri yapısı.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl MeshVertex {
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

        let attr_normal = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(12);

        let attr_uv = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(24);

        let attr_color = vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(3)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(32);

        (
            vec![binding],
            vec![attr_pos, attr_normal, attr_uv, attr_color],
        )
    }
}

/// Mesh renderı için Push Constant parametreleri.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshPushConstants {
    pub model: [[f32; 4]; 4],
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub render_mode: u32, // 0: Unlit, 1: MatCap/Studio, 2: PBR
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub _padding: [f32; 2],
}

/// Ölçeklenebilir 3D Mesh Shader'ı (MatCap ve Cook-Torrance PBR).
const MESH_WGSL: &str = r#"
struct PushConstants {
    model: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    render_mode: u32,
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
};

var<push_constant> pc: PushConstants;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

@vertex
fn main(in: VertexInput) -> VertexOutput {
    let world_pos = (pc.model * vec4<f32>(in.position, 1.0)).xyz;
    let normal_matrix = mat3x3<f32>(pc.model[0].xyz, pc.model[1].xyz, pc.model[2].xyz);
    let world_normal = normalize(normal_matrix * in.normal);

    var out: VertexOutput;
    out.clip_position = pc.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    out.normal = world_normal;
    out.uv = in.uv;
    out.color = in.color * pc.base_color;
    return out;
}

const PI: f32 = 3.14159265359;

@fragment
fn main_fs(in: VertexOutput) -> @location(0) vec4<f32> {
    let N = normalize(in.normal);
    let V = normalize(pc.camera_pos - in.world_pos);

    // 0: Unlit
    if (pc.render_mode == 0u) {
        return in.color;
    }

    // 1: Modelleme / Studio MatCap Modu (Düşük donanımlar için yüksek performanslı ve estetik)
    if (pc.render_mode == 1u) {
        let light_dir1 = normalize(vec3<f32>(0.5, 0.8, 0.6));
        let light_dir2 = normalize(vec3<f32>(-0.4, -0.3, -0.5));
        
        let diff1 = max(dot(N, light_dir1), 0.0);
        let diff2 = max(dot(N, light_dir2), 0.0) * 0.3;
        let rim = 1.0 - max(dot(N, V), 0.0);
        let rim_light = pow(rim, 3.0) * 0.4;

        let final_color = in.color.rgb * (diff1 + diff2 + 0.25) + vec3<f32>(rim_light);
        return vec4<f32>(final_color, in.color.a);
    }

    // 2: Cook-Torrance Mikro Yüzey PBR Modu
    let L = normalize(vec3<f32>(0.6, 1.0, 0.5));
    let H = normalize(V + L);
    let NdotL = max(dot(N, L), 0.0);
    let NdotV = max(dot(N, V), 0.001);
    let NdotH = max(dot(N, H), 0.0);
    let VdotH = max(dot(V, H), 0.0);

    // Fresnel Schlick
    let F0 = mix(vec3<f32>(0.04), in.color.rgb, pc.metallic);
    let F = F0 + (1.0 - F0) * pow(1.0 - VdotH, 5.0);

    // Trowbridge-Reitz GGX
    let a = pc.roughness * pc.roughness;
    let a2 = a * a;
    let denom = (NdotH * NdotH * (a2 - 1.0) + 1.0);
    let D = a2 / (PI * denom * denom);

    // Smith Geometry
    let k = (pc.roughness + 1.0) * (pc.roughness + 1.0) / 8.0;
    let g1 = NdotV / (NdotV * (1.0 - k) + k);
    let g2 = NdotL / (NdotL * (1.0 - k) + k);
    let G = g1 * g2;

    let specular = (D * G * F) / (4.0 * NdotV * NdotL + 0.001);
    let kD = (vec3<f32>(1.0) - F) * (1.0 - pc.metallic);
    let diffuse = kD * in.color.rgb / PI;

    let ambient = vec3<f32>(0.05) * in.color.rgb;
    let color = ambient + (diffuse + specular) * 2.5 * NdotL;

    return vec4<f32>(color, in.color.a);
}
"#;

/// 3D Mesh Render boru hattı yöneticisi.
pub struct VulkanMeshPipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
}

impl VulkanMeshPipeline {
    /// 3D Mesh pipeline'ını derler ve oluşturur.
    pub fn new(device: &Device, render_pass: vk::RenderPass) -> Result<Self, GfxError> {
        let vs_spv = compile_wgsl_to_spirv(MESH_WGSL, "vertex")?;
        let fs_spv = compile_wgsl_to_spirv(MESH_WGSL, "fragment")?;

        let vs_module = create_shader_module(device, &vs_spv)?;
        let fs_module = create_shader_module(device, &fs_spv)?;

        let (bindings, attrs) = MeshVertex::layout();

        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<MeshPushConstants>() as u32);

        let (pipeline, layout) = GraphicsPipelineBuilder::new(device, vs_module, fs_module, render_pass)
            .with_vertex_layout(bindings, attrs)
            .with_topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .with_cull_mode(vk::CullModeFlags::BACK)
            .with_depth(true, true) // Derinlik testi ve derinlik yazma aktif
            .with_push_constant(push_range)
            .build()?;

        // SAFETY: Modüller temizlenir.
        unsafe {
            device.destroy_shader_module(vs_module, None);
            device.destroy_shader_module(fs_module, None);
        }

        tracing::info!("3Point 3D Mesh (MatCap & PBR) Pipeline başarıyla oluşturuldu.");

        Ok(Self { pipeline, layout })
    }

    /// Bir mesh çizim çağrısı yürütür.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        device: &Device,
        cmd: vk::CommandBuffer,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        index_count: u32,
        model: Mat4,
        view_proj: Mat4,
        camera_pos: Vec3,
        render_mode: u32,
        base_color: Vec4,
        metallic: f32,
        roughness: f32,
    ) {
        let pc = MeshPushConstants {
            model: model.to_cols_array_2d(),
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.to_array(),
            render_mode,
            base_color: base_color.to_array(),
            metallic,
            roughness,
            _padding: [0.0; 2],
        };

        // SAFETY: Pipeline ve buffer'lar bağlanır.
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

            device.cmd_bind_vertex_buffers(cmd, 0, &[vertex_buffer], &[0]);
            device.cmd_bind_index_buffer(cmd, index_buffer, 0, vk::IndexType::UINT32);
            device.cmd_draw_indexed(cmd, index_count, 1, 0, 0, 0);
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

