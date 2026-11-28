//! 3Point Vulkan `egui` Render Entegrasyonu (`EguiVulkanRenderer`).

use ash::{vk, Device};
use egui::{epaint::Primitive, ClippedPrimitive};
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, Allocator},
    MemoryLocation,
};
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

@group(0) @binding(0)
var font_texture: texture_2d<f32>;
@group(0) @binding(1)
var font_sampler: sampler;

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
    let tex = textureSample(font_texture, font_sampler, in.uv);
    return in.color * tex;
}
"#;

/// GPU üzerinde saklanan yazı tipi dokusu (Font Texture Atlas).
pub struct VulkanTexture {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub allocation: Option<Allocation>,
    pub descriptor_set: vk::DescriptorSet,
    pub width: u32,
    pub height: u32,
}

impl VulkanTexture {
    /// Doku kaynaklarını GPU'dan serbest bırakır.
    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        // SAFETY: Vulkan sampler, image view ve image geçerlidir ve kullanımları sonlanmıştır.
        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
        }
        if let Some(alloc) = self.allocation.take() {
            let _ = allocator.free(alloc);
        }
    }
}

/// Vulkan üzerinde egui çizimlerini yürüten donanım hızlandırmalı UI motoru.
pub struct EguiVulkanRenderer {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub font_texture: Option<VulkanTexture>,
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

        // Descriptor Set Layout: Binding 0 -> SAMPLED_IMAGE, Binding 1 -> SAMPLER
        let dsl_bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];

        let dsl_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&dsl_bindings);
        // SAFETY: dsl_info geçerli bağlayıcı tanımlarını içerir.
        let descriptor_set_layout = unsafe { device.create_descriptor_set_layout(&dsl_info, None)? };

        // Descriptor Pool
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(16),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLER)
                .descriptor_count(16),
        ];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(16);
        // SAFETY: pool_info geçerli havuz boyutları ile tanımlanmıştır.
        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };

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
            .with_blending(true) // Şeffaf UI pencereleri ve anti-aliased fontlar için Alpha Blending
            .with_push_constant(push_range)
            .with_descriptor_layout(descriptor_set_layout)
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
            descriptor_set_layout,
            descriptor_pool,
            font_texture: None,
            vertex_buffer,
            index_buffer,
        })
    }

    /// Egui font ve arayüz dokularını GPU'ya yükler veya günceller.
    pub fn update_texture(
        &mut self,
        device: &Device,
        allocator: &mut Allocator,
        queue: vk::Queue,
        pool: vk::CommandPool,
        delta: &egui::epaint::ImageDelta,
    ) -> Result<(), GfxError> {
        let (delta_w, delta_h, pixels) = match &delta.image {
            egui::ImageData::Color(img) => {
                let px: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                (img.width() as u32, img.height() as u32, px)
            }
            egui::ImageData::Font(img) => {
                let px: Vec<u8> = img.srgba_pixels(None).flat_map(|c| c.to_array()).collect();
                (img.width() as u32, img.height() as u32, px)
            }
        };

        if delta_w == 0 || delta_h == 0 || pixels.is_empty() {
            return Ok(());
        }

        // Staging buffer oluştur ve veriyi yaz
        let buffer_size = (delta_w * delta_h * 4) as u64;
        let mut staging = GpuBuffer::new(
            device,
            allocator,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
            "Font Texture Staging",
        )?;
        staging.write_data(&pixels)?;

        if let (Some(tex), Some(pos)) = (&mut self.font_texture, delta.pos) {
            // Mevcut dokunun bir alt bölgesini (sub-region) güncelle
            let offset_x = pos[0] as i32;
            let offset_y = pos[1] as i32;

            let alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            // SAFETY: Havuz geçerlidir ve tek seferlik komut tamponu tahsis edilir.
            let cmd = unsafe { device.allocate_command_buffers(&alloc_info)?[0] };
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            // SAFETY: Geçerli komut tamponu başlatılır ve kopyalama bariyerleri işlenir.
            unsafe {
                device.begin_command_buffer(cmd, &begin_info)?;

                let barrier1 = vk::ImageMemoryBarrier::default()
                    .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .src_access_mask(vk::AccessFlags::SHADER_READ)
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .image(tex.image)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                device.cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier1],
                );

                let region = vk::BufferImageCopy::default()
                    .buffer_offset(0)
                    .buffer_row_length(0)
                    .buffer_image_height(0)
                    .image_subresource(vk::ImageSubresourceLayers {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image_offset(vk::Offset3D { x: offset_x, y: offset_y, z: 0 })
                    .image_extent(vk::Extent3D { width: delta_w, height: delta_h, depth: 1 });

                device.cmd_copy_buffer_to_image(
                    cmd,
                    staging.buffer,
                    tex.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[region],
                );

                let barrier2 = vk::ImageMemoryBarrier::default()
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ)
                    .image(tex.image)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                device.cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier2],
                );

                device.end_command_buffer(cmd)?;
                let cmds = [cmd];
                let submit = vk::SubmitInfo::default().command_buffers(&cmds);
                let submits = [submit];
                device.queue_submit(queue, &submits, vk::Fence::null())?;
                device.queue_wait_idle(queue)?;
                let free_cmds = [cmd];
                device.free_command_buffers(pool, &free_cmds);
            }
        } else {
            // Yeni tam doku oluştur
            if let Some(mut old_tex) = self.font_texture.take() {
                old_tex.destroy(device, allocator);
            }

            let extent = vk::Extent3D {
                width: delta_w,
                height: delta_h,
                depth: 1,
            };

            let image_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .extent(extent)
                .mip_levels(1)
                .array_layers(1)
                .format(vk::Format::R8G8B8A8_UNORM)
                .tiling(vk::ImageTiling::OPTIMAL)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                .samples(vk::SampleCountFlags::TYPE_1)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            // SAFETY: image_info geçerli bir 2B doku tanımıdır.
            let image = unsafe { device.create_image(&image_info, None)? };
            // SAFETY: image nesnesi geçerlidir.
            let reqs = unsafe { device.get_image_memory_requirements(image) };

            let allocation = allocator.allocate(&AllocationCreateDesc {
                name: "Egui Font Texture",
                requirements: reqs,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            }).map_err(|e| GfxError::Generic(format!("Font dokusu bellek hatası: {e}")))?;

            // SAFETY: Bellek tahsisi başarılıdır ve görüntüye bağlanır.
            unsafe {
                device.bind_image_memory(image, allocation.memory(), allocation.offset())?;
            }

            let alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            // SAFETY: Havuz geçerlidir ve tek seferlik komut tamponu tahsis edilir.
            let cmd = unsafe { device.allocate_command_buffers(&alloc_info)?[0] };
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            // SAFETY: Geçerli komut tamponu başlatılır ve kopyalama bariyerleri işlenir.
            unsafe {
                device.begin_command_buffer(cmd, &begin_info)?;

                let barrier1 = vk::ImageMemoryBarrier::default()
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .image(image)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                device.cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier1],
                );

                let region = vk::BufferImageCopy::default()
                    .buffer_offset(0)
                    .buffer_row_length(0)
                    .buffer_image_height(0)
                    .image_subresource(vk::ImageSubresourceLayers {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                    .image_extent(extent);

                device.cmd_copy_buffer_to_image(
                    cmd,
                    staging.buffer,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[region],
                );

                let barrier2 = vk::ImageMemoryBarrier::default()
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ)
                    .image(image)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                device.cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier2],
                );

                device.end_command_buffer(cmd)?;
                let cmds = [cmd];
                let submit = vk::SubmitInfo::default().command_buffers(&cmds);
                let submits = [submit];
                device.queue_submit(queue, &submits, vk::Fence::null())?;
                device.queue_wait_idle(queue)?;
                let free_cmds = [cmd];
                device.free_command_buffers(pool, &free_cmds);
            }

            let view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            // SAFETY: view_info geçerli doku görüntüsü tanımlarını içerir.
            let view = unsafe { device.create_image_view(&view_info, None)? };

            let sampler_info = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .anisotropy_enable(false)
                .max_anisotropy(1.0)
                .border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE)
                .unnormalized_coordinates(false)
                .compare_enable(false)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR);
            // SAFETY: sampler_info geçerli filtreleme ve kenar modu tanımlarını içerir.
            let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

            let set_layouts = [self.descriptor_set_layout];
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(self.descriptor_pool)
                .set_layouts(&set_layouts);
            // SAFETY: Tanımlayıcı havuzu geçerlidir ve yeterli kapasiteye sahiptir.
            let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info)?[0] };

            let image_info = [vk::DescriptorImageInfo::default()
                .image_view(view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
            let sampler_info = [vk::DescriptorImageInfo::default()
                .sampler(sampler)];

            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&sampler_info),
            ];
            // SAFETY: writes tanımlayıcı kümesi için geçerli hedef ve kaynak bağlantılarını içerir.
            unsafe {
                device.update_descriptor_sets(&writes, &[]);
            }

            self.font_texture = Some(VulkanTexture {
                image,
                view,
                sampler,
                allocation: Some(allocation),
                descriptor_set,
                width: delta_w,
                height: delta_h,
            });
        }

        staging.destroy(device, allocator);
        Ok(())
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

        // SAFETY: Pipeline, Descriptor Set, Push Constants ve Scissor ile çizim tetiklenir.
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            if let Some(tex) = &self.font_texture {
                device.cmd_bind_descriptor_sets(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.layout,
                    0,
                    &[tex.descriptor_set],
                    &[],
                );
            }

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
        if let Some(mut tex) = self.font_texture.take() {
            tex.destroy(device, allocator);
        }
        self.vertex_buffer.destroy(device, allocator);
        self.index_buffer.destroy(device, allocator);
        // SAFETY: Pipeline, layout ve descriptor havuzları yok edilir.
        unsafe {
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}
