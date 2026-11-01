//! 3Point Vulkan Fiziksel ve Mantıksal Cihaz (GPU) Yönetimi.

use crate::{error::GfxError, surface::VulkanSurface};
use ash::{
    khr::swapchain,
    vk, Device, Instance,
};
use std::ffi::CStr;

/// Grafik ve sunum işlemlerinin gönderileceği kuyruk aile indeksleri.
#[derive(Debug, Clone, Copy)]
pub struct QueueFamilyIndices {
    pub graphics_family: u32,
    pub present_family: u32,
}

impl QueueFamilyIndices {
    /// Kuyruk indekslerinin geçerli olup olmadığını sınar.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        true
    }
}

/// Vulkan fiziksel GPU ve mantıksal aygıt yöneticisi.
pub struct VulkanDevice {
    pub physical_device: vk::PhysicalDevice,
    pub device: Device,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub queue_indices: QueueFamilyIndices,
}

impl VulkanDevice {
    /// Sistemdeki GPU'ları analiz ederek en uygun olanını seçer ve mantıksal aygıtı oluşturur.
    pub fn new(instance: &Instance, surface: &VulkanSurface) -> Result<Self, GfxError> {
        // SAFETY: enumerate_physical_devices sistemdeki tüm Vulkan destekli kartları listeler.
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        if physical_devices.is_empty() {
            return Err(GfxError::NoSuitableGpu);
        }

        tracing::info!("Sistemde {} adet Vulkan grafik aygıtı bulundu.", physical_devices.len());

        let mut selected_gpu = None;
        let mut selected_indices = None;
        let mut best_score = -1;

        for &p_device in &physical_devices {
            // SAFETY: get_physical_device_properties GPU özelliklerini sorgular.
            let props = unsafe { instance.get_physical_device_properties(p_device) };
            // SAFETY: device_name geçerli bir C dizisidir.
            let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }
                .to_str()
                .unwrap_or("<bilinmeyen>");

            let score = rate_physical_device(instance, p_device, surface);
            tracing::info!("GPU: '{}' | Skor: {} | Türü: {:?}", name, score, props.device_type);

            if let Some(indices) = find_queue_families(instance, p_device, surface)? {
                if score > best_score {
                    best_score = score;
                    selected_gpu = Some(p_device);
                    selected_indices = Some(indices);
                }
            }
        }

        let physical_device = selected_gpu.ok_or(GfxError::NoSuitableGpu)?;
        let queue_indices = selected_indices.ok_or(GfxError::QueueFamilyNotFound)?;

        // SAFETY: Seçilen GPU'nun adını loglamak için özellikleri tekrar alıyoruz.
        let props = unsafe { instance.get_physical_device_properties(physical_device) };
        let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }
            .to_str()
            .unwrap_or("<bilinmeyen>");
        tracing::info!("Seçilen GPU: '{}'", name);

        // Kuyruk oluşturma bilgileri
        let mut unique_indices = vec![queue_indices.graphics_family];
        if queue_indices.present_family != queue_indices.graphics_family {
            unique_indices.push(queue_indices.present_family);
        }

        let queue_priorities = [1.0f32];
        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = unique_indices
            .iter()
            .map(|&family_idx| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(family_idx)
                    .queue_priorities(&queue_priorities)
            })
            .collect();

        // Gerekli cihaz uzantıları (Örn: Swapchain)
        let device_extension_names = [swapchain::NAME.as_ptr()];

        // Düşük donanımlı eski GPU'larda da çalışması için yalnızca temel cihaz özellikleri
        let device_features = vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(true);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extension_names)
            .enabled_features(&device_features);

        // SAFETY: Mantıksal cihaz oluşturma geçerli parametrelerle gerçekleştirilir.
        let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };

        // SAFETY: Cihaz oluşturulduktan sonra kuyruk tutucularını alıyoruz.
        let graphics_queue = unsafe { device.get_device_queue(queue_indices.graphics_family, 0) };
        let present_queue = unsafe { device.get_device_queue(queue_indices.present_family, 0) };

        Ok(Self {
            physical_device,
            device,
            graphics_queue,
            present_queue,
            queue_indices,
        })
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        // SAFETY: Mantıksal cihaz yok edilmeden önce boşa düşmesi beklenir.
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_device(None);
        }
        tracing::debug!("Vulkan Mantıksal Cihaz serbest bırakıldı.");
    }
}

/// GPU'yu puanlar: Ayrık (Discrete) GPU'lara öncelik verir, Entegre (Integrated) GPU'ları da destekler.
fn rate_physical_device(
    instance: &Instance,
    device: vk::PhysicalDevice,
    surface: &VulkanSurface,
) -> i32 {
    // SAFETY: Donanım sorgulaması.
    let props = unsafe { instance.get_physical_device_properties(device) };
    let mut score = 0;

    match props.device_type {
        vk::PhysicalDeviceType::DISCRETE_GPU => score += 1000,
        vk::PhysicalDeviceType::INTEGRATED_GPU => score += 500, // Entegre laptop GPU'ları için destek
        vk::PhysicalDeviceType::VIRTUAL_GPU => score += 200,
        vk::PhysicalDeviceType::CPU => score += 50,
        _ => {}
    }

    // Swapchain desteği kontrolü
    if !check_device_extension_support(instance, device) {
        return -1;
    }

    // Yüzey format ve sunum modu kontrolü
    // SAFETY: Yüzey yeteneklerinin sorgulanması.
    let formats = unsafe {
        surface
            .loader
            .get_physical_device_surface_formats(device, surface.surface)
            .unwrap_or_default()
    };
    let present_modes = unsafe {
        surface
            .loader
            .get_physical_device_surface_present_modes(device, surface.surface)
            .unwrap_or_default()
    };

    if formats.is_empty() || present_modes.is_empty() {
        return -1;
    }

    score
}

/// Cihazın Swapchain uzantısını destekleyip desteklemediğini kontrol eder.
fn check_device_extension_support(instance: &Instance, device: vk::PhysicalDevice) -> bool {
    // SAFETY: enumerate_device_extension_properties uzantıları sorgular.
    let extensions = unsafe {
        instance
            .enumerate_device_extension_properties(device)
            .unwrap_or_default()
    };

    let target_name = swapchain::NAME;
    extensions.iter().any(|ext| {
        // SAFETY: extension_name null ile sonlandırılmış bir dizidir.
        let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
        name == target_name
    })
}

/// Grafik ve ekran sunumu (presentation) yapabilen kuyruk ailelerini bulur.
fn find_queue_families(
    instance: &Instance,
    device: vk::PhysicalDevice,
    surface: &VulkanSurface,
) -> Result<Option<QueueFamilyIndices>, GfxError> {
    // SAFETY: get_physical_device_queue_family_properties kuyruk yapılarını döner.
    let queue_families = unsafe { instance.get_physical_device_queue_family_properties(device) };

    let mut graphics_family = None;
    let mut present_family = None;

    for (idx, family) in queue_families.iter().enumerate() {
        let index = idx as u32;

        if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            graphics_family = Some(index);
        }

        // SAFETY: get_physical_device_surface_support yüzey desteğini sorgular.
        let present_support = unsafe {
            surface
                .loader
                .get_physical_device_surface_support(device, index, surface.surface)?
        };

        if present_support {
            present_family = Some(index);
        }

        if graphics_family.is_some() && present_family.is_some() {
            break;
        }
    }

    if let (Some(g), Some(p)) = (graphics_family, present_family) {
        Ok(Some(QueueFamilyIndices {
            graphics_family: g,
            present_family: p,
        }))
    } else {
        Ok(None)
    }
}
