//! 3Point Vulkan Komut Havuzu ve Komut Tamponları (Command Buffers).

use crate::{error::GfxError, sync::MAX_FRAMES_IN_FLIGHT};
use ash::{vk, Device};

/// GPU komut tamponlarını ve bellek havuzunu yöneten yapı.
pub struct VulkanCommands {
    pub pool: vk::CommandPool,
    pub buffers: [vk::CommandBuffer; MAX_FRAMES_IN_FLIGHT],
}

impl VulkanCommands {
    /// Yeni bir komut havuzu ve her kare için bir komut tamponu tahsis eder.
    pub fn new(device: &Device, graphics_family_index: u32) -> Result<Self, GfxError> {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(graphics_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        // SAFETY: vkCreateCommandPool çağrısı.
        let pool = unsafe { device.create_command_pool(&pool_info, None)? };

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);

        // SAFETY: vkAllocateCommandBuffers çağrısı.
        let raw_buffers = unsafe { device.allocate_command_buffers(&alloc_info)? };

        let mut buffers = [vk::CommandBuffer::null(); MAX_FRAMES_IN_FLIGHT];
        buffers.copy_from_slice(&raw_buffers[..MAX_FRAMES_IN_FLIGHT]);

        tracing::info!("Vulkan komut tamponları ({MAX_FRAMES_IN_FLIGHT} adet) tahsis edildi.");

        Ok(Self { pool, buffers })
    }

    /// Komut havuzunu ve tamponları serbest bırakır.
    pub fn destroy(&mut self, device: &Device) {
        // SAFETY: vkDestroyCommandPool çağrısı havuzu ve bağlı tüm tamponları temizler.
        unsafe {
            device.destroy_command_pool(self.pool, None);
        }
        tracing::debug!("Vulkan komut havuzu serbest bırakıldı.");
    }
}
