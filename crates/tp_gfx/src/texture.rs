//! 3Point Vulkan 2D Doku (Texture) ve Örnekleyici (Sampler) Yönetimi.

use crate::{error::GfxError, GpuBuffer};
use ash::{vk, Device};
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, Allocator},
    MemoryLocation,
};

/// Donanım hızlandırmalı Vulkan 2D Doku nesnesi.
pub struct VulkanTexture {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub allocation: Option<Allocation>,
    pub width: u32,
    pub height: u32,
}

impl VulkanTexture {
    /// Ham RGBA8 piksel verisinden yeni bir 2D GPU dokusu oluşturur.
    pub fn from_rgba8(
        device: &Device,
        allocator: &mut Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<Self, GfxError> {
        let expected_size = (width * height * 4) as usize;
        if pixels.len() < expected_size {
            return Err(GfxError::Generic(format!(
                "Geçersiz piksel boyutu: beklenen {}, alınan {}",
                expected_size,
                pixels.len()
            )));
        }

        // 1. CPU Staging Tamponu oluştur ve pikselleri kopyala
        let staging_size = expected_size as u64;
        let mut staging = GpuBuffer::new(
            device,
            allocator,
            staging_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
            "Texture Staging Buffer",
        )?;
        staging.write_data(&pixels[..expected_size])?;

        // 2. Hedef GPU Doku Görüntüsünü (vk::Image) oluştur
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(vk::Format::R8G8B8A8_UNORM)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        // SAFETY: image_info is valid and device is initialized
        let image = unsafe { device.create_image(&image_info, None)? };
        // SAFETY: image handle is valid
        let reqs = unsafe { device.get_image_memory_requirements(image) };

        let allocation = allocator
            .allocate(&AllocationCreateDesc {
                name: "Vulkan Texture Image",
                requirements: reqs,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| GfxError::Generic(format!("Doku belleği tahsis hatası: {e}")))?;

        // SAFETY: image and memory allocation are valid, offset is within bounds
        unsafe {
            device.bind_image_memory(image, allocation.memory(), allocation.offset())?;
        }

        // 3. Tek kullanımlık komut tamponu ile veriyi GPU dokusuna kopyala
        let cmd_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        // SAFETY: cmd_alloc_info is valid and command_pool is valid
        let cmd = unsafe { device.allocate_command_buffers(&cmd_alloc_info)?[0] };

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        // SAFETY: cmd, buffers and pipeline barrier states are valid for submission
        unsafe {
            device.begin_command_buffer(cmd, &begin_info)?;

            // Düzen geçişi: UNDEFINED -> TRANSFER_DST_OPTIMAL
            let barrier_to_dst = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);

            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier_to_dst],
            );

            // Staging tamponundan GPU dokusuna kopyala
            let copy_region = vk::BufferImageCopy::default()
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
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                });

            device.cmd_copy_buffer_to_image(
                cmd,
                staging.buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy_region],
            );

            // Düzen geçişi: TRANSFER_DST_OPTIMAL -> SHADER_READ_ONLY_OPTIMAL
            let barrier_to_shader = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);

            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier_to_shader],
            );

            device.end_command_buffer(cmd)?;

            // Komutları gönder ve tamamlanmasını bekle
            let cmd_buffers = [cmd];
            let submits = [vk::SubmitInfo::default().command_buffers(&cmd_buffers)];
            device.queue_submit(queue, &submits, vk::Fence::null())?;
            device.queue_wait_idle(queue)?;

            device.free_command_buffers(command_pool, &[cmd]);
        }

        // Staging tamponunu temizle
        staging.destroy(device, allocator);

        // 4. ImageView oluştur
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

        // SAFETY: view_info references valid image
        let view = unsafe { device.create_image_view(&view_info, None)? };

        // 5. Sampler oluştur (Doğrusal filtreleme ve tekrar döngüsü)
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(false)
            .max_anisotropy(1.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR);

        // SAFETY: sampler_info is valid
        let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

        Ok(Self {
            image,
            view,
            sampler,
            allocation: Some(allocation),
            width,
            height,
        })
    }

    /// 1x1 düz beyaz temel doku oluşturur (Varsayılan dokusuz yüzeyler için).
    pub fn create_white(
        device: &Device,
        allocator: &mut Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
    ) -> Result<Self, GfxError> {
        let white_pixel = [255u8, 255u8, 255u8, 255u8];
        Self::from_rgba8(device, allocator, queue, command_pool, 1, 1, &white_pixel)
    }

    /// 1x1 varsayılan teğet uzayı normal haritası dokusu [128, 128, 255, 255] (+Z dik normal).
    pub fn create_default_normal(
        device: &Device,
        allocator: &mut Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
    ) -> Result<Self, GfxError> {
        let normal_pixel = [128u8, 128u8, 255u8, 255u8];
        Self::from_rgba8(device, allocator, queue, command_pool, 1, 1, &normal_pixel)
    }

    /// Damalı (Checkerboard) stüdyo UV test dokusu oluşturur.
    pub fn create_checkerboard(
        device: &Device,
        allocator: &mut Allocator,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        size: u32,
        tile_size: u32,
    ) -> Result<Self, GfxError> {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        let c1 = [220u8, 220u8, 225u8, 255u8];
        let c2 = [70u8, 70u8, 75u8, 255u8];

        for y in 0..size {
            for x in 0..size {
                let is_even = ((x / tile_size) + (y / tile_size)).is_multiple_of(2);
                let color = if is_even { c1 } else { c2 };
                pixels.extend_from_slice(&color);
            }
        }

        Self::from_rgba8(device, allocator, queue, command_pool, size, size, &pixels)
    }

    /// Doku kaynaklarını temizler.
    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        // SAFETY: All Vulkan handles are valid and will not be accessed again after destruction
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
