//! 3Point Vulkan Derinlik Tamponu (Depth Buffer) ve Format Yönetimi.

use crate::error::GfxError;
use ash::{vk, Device, Instance};
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, Allocator},
    MemoryLocation,
};

/// 3D sahnelerde piksel derinlik sıralamasını sağlayan derinlik dokusu.
pub struct VulkanDepth {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub allocation: Option<Allocation>,
    pub format: vk::Format,
}

impl VulkanDepth {
    /// Cihazın desteklediği en yüksek hassasiyetli derinlik formatını bulur.
    pub fn find_depth_format(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::Format, GfxError> {
        let candidates = [
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ];

        for &format in &candidates {
            // SAFETY: Format özelliklerinin donanımdan sorgulanması.
            let props = unsafe {
                instance.get_physical_device_format_properties(physical_device, format)
            };

            if props
                .optimal_tiling_features
                .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
            {
                return Ok(format);
            }
        }

        Err(GfxError::Generic("Uygun derinlik formatı (Depth Format) bulunamadı.".into()))
    }

    /// Yeni bir derinlik görüntüsü ve bellek tahsisi oluşturur.
    pub fn new(
        instance: &Instance,
        device: &Device,
        allocator: &mut Allocator,
        physical_device: vk::PhysicalDevice,
        extent: vk::Extent2D,
    ) -> Result<Self, GfxError> {
        let format = Self::find_depth_format(instance, physical_device)?;

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
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        // SAFETY: vkCreateImage çağrısı.
        let image = unsafe { device.create_image(&image_info, None)? };

        // SAFETY: Görüntü bellek gereksinimlerinin sorgulanması.
        let reqs = unsafe { device.get_image_memory_requirements(image) };

        let allocation = allocator.allocate(&AllocationCreateDesc {
            name: "Vulkan Depth Image",
            requirements: reqs,
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
        }).map_err(|e| GfxError::Generic(format!("Derinlik belleği tahsis hatası: {e}")))?;

        // SAFETY: Belleğin görüntüye bağlanması (Bind).
        unsafe {
            device.bind_image_memory(image, allocation.memory(), allocation.offset())?;
        }

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

        // SAFETY: ImageView oluşturulması.
        let view = unsafe { device.create_image_view(&view_info, None)? };

        Ok(Self {
            image,
            view,
            allocation: Some(allocation),
            format,
        })
    }

    /// Derinlik tamponu kaynaklarını serbest bırakır.
    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        // SAFETY: View ve Image yok edilmesi.
        unsafe {
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
        }

        if let Some(alloc) = self.allocation.take() {
            let _ = allocator.free(alloc);
        }
        tracing::debug!("Vulkan Derinlik tamponu serbest bırakıldı.");
    }
}
