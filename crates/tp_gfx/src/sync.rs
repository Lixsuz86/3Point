//! 3Point Vulkan CPU-GPU Senkronizasyonu (Semaphores & Fences).

use crate::error::GfxError;
use ash::{vk, Device};

/// Ardışık işlenen kare sayısı (Frames-in-Flight). Çift tamponlama için 2 seçilmiştir.
pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

/// Her kare için gerekli Vulkan senkronizasyon nesneleri.
pub struct VulkanSync {
    pub image_available_semaphores: [vk::Semaphore; MAX_FRAMES_IN_FLIGHT],
    pub render_finished_semaphores: [vk::Semaphore; MAX_FRAMES_IN_FLIGHT],
    pub in_flight_fences: [vk::Fence; MAX_FRAMES_IN_FLIGHT],
}

impl VulkanSync {
    /// Senkronizasyon ilkellerini oluşturur.
    pub fn new(device: &Device) -> Result<Self, GfxError> {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        // İlk karenin beklemeden başlaması için fence'ler SIGNALED durumunda oluşturulur.
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let mut image_available = [vk::Semaphore::null(); MAX_FRAMES_IN_FLIGHT];
        let mut render_finished = [vk::Semaphore::null(); MAX_FRAMES_IN_FLIGHT];
        let mut in_flight = [vk::Fence::null(); MAX_FRAMES_IN_FLIGHT];

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            // SAFETY: vkCreateSemaphore ve vkCreateFence çağrıları.
            unsafe {
                image_available[i] = device.create_semaphore(&semaphore_info, None)?;
                render_finished[i] = device.create_semaphore(&semaphore_info, None)?;
                in_flight[i] = device.create_fence(&fence_info, None)?;
            }
        }

        tracing::info!("Vulkan senkronizasyon nesneleri (2 Frames-in-Flight) oluşturuldu.");

        Ok(Self {
            image_available_semaphores: image_available,
            render_finished_semaphores: render_finished,
            in_flight_fences: in_flight,
        })
    }

    /// Senkronizasyon nesnelerini serbest bırakır.
    pub fn destroy(&mut self, device: &Device) {
        // SAFETY: Nesnelerin yok edilmesi.
        unsafe {
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                device.destroy_semaphore(self.image_available_semaphores[i], None);
                device.destroy_semaphore(self.render_finished_semaphores[i], None);
                device.destroy_fence(self.in_flight_fences[i], None);
            }
        }
        tracing::debug!("Vulkan senkronizasyon nesneleri serbest bırakıldı.");
    }
}

