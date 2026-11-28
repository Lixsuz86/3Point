//! 3Point Dinamik Yönlü Işık Gölge Haritalama Hattı (Directional Shadow Mapping & PCF).

use crate::error::GfxError;
use ash::{vk, Device};
use glam::{Mat4, Vec3};
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, Allocator},
    MemoryLocation,
};

/// 2048x2048 çözünürlüklü donanım hızlandırmalı gölge haritası nesnesi.
pub struct ShadowMap {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub allocation: Option<Allocation>,
    pub extent: vk::Extent2D,
    pub format: vk::Format,
}

impl ShadowMap {
    pub const RESOLUTION: u32 = 2048;

    /// Yeni bir Gölge Haritası derinlik tamponu ve RenderPass'i oluşturur.
    pub fn new(device: &Device, allocator: &mut Allocator) -> Result<Self, GfxError> {
        let format = vk::Format::D32_SFLOAT;
        let extent = vk::Extent2D {
            width: Self::RESOLUTION,
            height: Self::RESOLUTION,
        };

        // 1. Derinlik Görüntüsü (Image) oluştur
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        // SAFETY: image_info is valid and device is initialized
        let image = unsafe { device.create_image(&image_info, None)? };
        // SAFETY: image handle is valid
        let reqs = unsafe { device.get_image_memory_requirements(image) };

        let allocation = allocator
            .allocate(&AllocationCreateDesc {
                name: "Shadow Map Depth Image",
                requirements: reqs,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| GfxError::Generic(format!("Gölge haritası belleği tahsis hatası: {e}")))?;

        // SAFETY: image and memory allocation are valid, offset is within bounds
        unsafe {
            device.bind_image_memory(image, allocation.memory(), allocation.offset())?;
        }

        // 2. ImageView oluştur
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        // SAFETY: view_info is valid and references the newly created valid image
        let view = unsafe { device.create_image_view(&view_info, None)? };

        // 3. Karşılaştırmalı Donanım PCF Sampler oluştur
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_BORDER)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_BORDER)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_BORDER)
            .border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE)
            .compare_enable(true)
            .compare_op(vk::CompareOp::LESS_OR_EQUAL);

        // SAFETY: sampler_info is valid and device is initialized
        let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

        // 4. Yalnızca Derinlik içeren Gölge RenderPass'i oluştur
        let depth_attachment = vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let depth_attachment_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .depth_stencil_attachment(&depth_attachment_ref);

        let dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .dst_stage_mask(vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
            .src_access_mask(vk::AccessFlags::SHADER_READ)
            .dst_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE);

        let attachments = [depth_attachment];
        let subpasses = [subpass];
        let dependencies = [dependency];

        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        // SAFETY: render_pass_info references valid attachment descriptions
        let render_pass = unsafe { device.create_render_pass(&render_pass_info, None)? };

        // 5. Framebuffer oluştur
        let fb_attachments = [view];
        let fb_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .attachments(&fb_attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        // SAFETY: fb_info references valid render_pass and image view attachments
        let framebuffer = unsafe { device.create_framebuffer(&fb_info, None)? };

        Ok(Self {
            image,
            view,
            sampler,
            render_pass,
            framebuffer,
            allocation: Some(allocation),
            extent,
            format,
        })
    }

    /// Işık yönü ve sahne sınırlarına göre Ortografik Işık View-Projection matrisini hesaplar.
    pub fn compute_light_view_proj(light_dir: Vec3, scene_center: Vec3, scene_radius: f32) -> Mat4 {
        let dir = light_dir.normalize();
        let eye = scene_center - dir * scene_radius;
        let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };

        let view = Mat4::look_at_rh(eye, scene_center, up);
        let r = scene_radius.max(1.0);
        let mut proj = Mat4::orthographic_rh(-r, r, -r, r, 0.1, scene_radius * 2.5);
        proj.y_axis.y *= -1.0; // Vulkan Y-ekseni ters çevirme

        proj * view
    }

    /// Gölge haritası kaynaklarını temizler.
    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        // SAFETY: All Vulkan handles are valid and will not be accessed again after destruction
        unsafe {
            device.destroy_framebuffer(self.framebuffer, None);
            device.destroy_render_pass(self.render_pass, None);
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
        }

        if let Some(alloc) = self.allocation.take() {
            let _ = allocator.free(alloc);
        }
    }
}
