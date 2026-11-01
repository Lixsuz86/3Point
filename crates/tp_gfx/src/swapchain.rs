//! 3Point Vulkan Swapchain (Kare Değişim Zinciri) ve Görüntü Görünümleri (Image Views).

use crate::{
    device::VulkanDevice,
    error::GfxError,
    surface::VulkanSurface,
};
use ash::{
    khr::swapchain::Device as SwapchainLoader,
    vk, Instance,
};

/// Swapchain ve ilişkili kare görüntülerini (`vk::ImageView`) yöneten yapı.
pub struct VulkanSwapchain {
    pub loader: SwapchainLoader,
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    pub format: vk::SurfaceFormatKHR,
    pub extent: vk::Extent2D,
}

impl VulkanSwapchain {
    /// Yeni bir swapchain ve görüntü görünümlerini oluşturur.
    pub fn new(
        instance: &Instance,
        surface: &VulkanSurface,
        device: &VulkanDevice,
        width: u32,
        height: u32,
        old_swapchain: Option<vk::SwapchainKHR>,
    ) -> Result<Self, GfxError> {
        let loader = SwapchainLoader::new(instance, &device.device);

        // SAFETY: Yüzey yeteneklerinin sorgulanması.
        let capabilities = unsafe {
            surface
                .loader
                .get_physical_device_surface_capabilities(device.physical_device, surface.surface)?
        };

        // SAFETY: Desteklenen formatların sorgulanması.
        let formats = unsafe {
            surface
                .loader
                .get_physical_device_surface_formats(device.physical_device, surface.surface)?
        };

        // SAFETY: Desteklenen sunum modlarının sorgulanması.
        let present_modes = unsafe {
            surface
                .loader
                .get_physical_device_surface_present_modes(device.physical_device, surface.surface)?
        };

        let surface_format = choose_swapchain_surface_format(&formats);
        let present_mode = choose_swapchain_present_mode(&present_modes);
        let extent = choose_swapchain_extent(&capabilities, width, height);

        // En az çift/üçlü tamponlama için görüntü sayısı
        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }

        let mut create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        if let Some(old) = old_swapchain {
            create_info = create_info.old_swapchain(old);
        }

        let queue_family_indices = [
            device.queue_indices.graphics_family,
            device.queue_indices.present_family,
        ];

        let create_info = if device.queue_indices.graphics_family != device.queue_indices.present_family {
            create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_family_indices)
        } else {
            create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        };

        // SAFETY: vkCreateSwapchainKHR geçerli struct bilgileri ile yürütülür.
        let swapchain = unsafe { loader.create_swapchain(&create_info, None)? };

        // SAFETY: Swapchain görüntülerini alıyoruz.
        let images = unsafe { loader.get_swapchain_images(swapchain)? };

        // Her swapchain görüntüsü için ImageView oluşturma
        let mut image_views = Vec::with_capacity(images.len());
        for &img in &images {
            let view_info = vk::ImageViewCreateInfo::default()
                .image(img)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            // SAFETY: ImageView oluşturulması.
            let view = unsafe { device.device.create_image_view(&view_info, None)? };
            image_views.push(view);
        }

        tracing::info!(
            "Swapchain başarıyla oluşturuldu: {}x{} | {} Görüntü | Mod: {:?}",
            extent.width,
            extent.height,
            images.len(),
            present_mode
        );

        Ok(Self {
            loader,
            swapchain,
            images,
            image_views,
            format: surface_format,
            extent,
        })
    }

    /// Swapchain'e ait Image View'ları yok eder.
    pub fn destroy_image_views(&mut self, device: &ash::Device) {
        for view in self.image_views.drain(..) {
            // SAFETY: ImageView yok edilmesi.
            unsafe {
                device.destroy_image_view(view, None);
            }
        }
    }

    /// Swapchain'i tamamen serbest bırakır.
    pub fn destroy(&mut self, device: &ash::Device) {
        self.destroy_image_views(device);
        // SAFETY: Swapchain nesnesinin yok edilmesi.
        unsafe {
            self.loader.destroy_swapchain(self.swapchain, None);
        }
        tracing::debug!("Vulkan Swapchain serbest bırakıldı.");
    }
}

/// En uygun yüzey formatını (Tercihen sRGB) seçer.
fn choose_swapchain_surface_format(available_formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    for &f in available_formats {
        if (f.format == vk::Format::B8G8R8A8_SRGB || f.format == vk::Format::R8G8B8A8_SRGB)
            && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return f;
        }
    }
    available_formats[0]
}

/// Sunum modunu seçer: V-Sync yırtılmasız Mailbox veya tüm kartlarda standart olan Fifo.
fn choose_swapchain_present_mode(available_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    for &mode in available_modes {
        if mode == vk::PresentModeKHR::MAILBOX {
            return mode;
        }
    }
    vk::PresentModeKHR::FIFO
}

/// Swapchain piksel boyutunu (Extent) pencere boyutuna ve GPU limitlerine göre sınırlar.
fn choose_swapchain_extent(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    width: u32,
    height: u32,
) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D {
            width: width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }
}
