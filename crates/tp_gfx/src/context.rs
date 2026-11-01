//! 3Point Ana Vulkan Grafik Bağlamı (Graphics Context).

use crate::{
    command::VulkanCommands,
    device::VulkanDevice,
    error::GfxError,
    instance::VulkanInstance,
    render_pass::VulkanRenderPass,
    surface::VulkanSurface,
    swapchain::VulkanSwapchain,
    sync::{VulkanSync, MAX_FRAMES_IN_FLIGHT},
};
use ash::vk;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

/// Aktif bir karenin çizim durumunu temsil eden bağlam.
pub struct FrameContext {
    pub image_index: u32,
    pub command_buffer: vk::CommandBuffer,
    pub extent: vk::Extent2D,
}

/// Tüm Vulkan bileşenlerini bir araya getiren ana grafik motoru arayüzü.
pub struct GfxContext {
    pub instance: VulkanInstance,
    pub surface: VulkanSurface,
    pub device: VulkanDevice,
    pub swapchain: VulkanSwapchain,
    pub render_pass: VulkanRenderPass,
    pub sync: VulkanSync,
    pub commands: VulkanCommands,
    pub current_frame: usize,
    pub framebuffer_resized: bool,
}

impl GfxContext {
    /// Pencere ve ekran tutucuları üzerinden Vulkan grafik motorunu başlatır.
    pub fn new(
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        width: u32,
        height: u32,
    ) -> Result<Self, GfxError> {
        let enable_validation = cfg!(debug_assertions);
        let instance = VulkanInstance::new(display_handle, enable_validation)?;
        let surface = VulkanSurface::new(
            &instance.entry,
            &instance.instance,
            display_handle,
            window_handle,
        )?;
        let device = VulkanDevice::new(&instance.instance, &surface)?;
        let swapchain = VulkanSwapchain::new(
            &instance.instance,
            &surface,
            &device,
            width,
            height,
            None,
        )?;
        let render_pass = VulkanRenderPass::new(
            &device.device,
            swapchain.format.format,
            &swapchain.image_views,
            swapchain.extent,
        )?;
        let sync = VulkanSync::new(&device.device)?;
        let commands = VulkanCommands::new(&device.device, device.queue_indices.graphics_family)?;

        tracing::info!("3Point Vulkan Grafik Bağlamı (GfxContext) başarıyla kuruldu.");

        Ok(Self {
            instance,
            surface,
            device,
            swapchain,
            render_pass,
            sync,
            commands,
            current_frame: 0,
            framebuffer_resized: false,
        })
    }

    /// Yeni bir karenin çizimini başlatır.
    ///
    /// Swapchain ekran boyutuyla uyumsuz hale gelmişse `None` döner ve swapchain'i yeniler.
    pub fn begin_frame(&mut self, width: u32, height: u32) -> Result<Option<FrameContext>, GfxError> {
        if width == 0 || height == 0 {
            return Ok(None); // Pencere simge durumuna küçültülmüş
        }

        let fence = self.sync.in_flight_fences[self.current_frame];

        // SAFETY: Önceki karenin GPU tarafında tamamlanmasını bekle (1 saniye zaman aşımı).
        unsafe {
            self.device
                .device
                .wait_for_fences(&[fence], true, 1_000_000_000)?;
        }

        // Sıradaki swapchain görüntüsünü iste
        let image_available_semaphore = self.sync.image_available_semaphores[self.current_frame];
        let acquire_result = unsafe {
            self.swapchain.loader.acquire_next_image(
                self.swapchain.swapchain,
                1_000_000_000,
                image_available_semaphore,
                vk::Fence::null(),
            )
        };

        let image_index = match acquire_result {
            Ok((idx, suboptimal)) => {
                if suboptimal || self.framebuffer_resized {
                    self.recreate_swapchain(width, height)?;
                    return Ok(None);
                }
                idx
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain(width, height)?;
                return Ok(None);
            }
            Err(e) => return Err(GfxError::VkResult(e)),
        };

        // SAFETY: Fence'i sıfırlıyoruz.
        unsafe {
            self.device.device.reset_fences(&[fence])?;
        }

        let cmd = self.commands.buffers[self.current_frame];

        // SAFETY: Komut tamponunu sıfırla ve kayda başla.
        unsafe {
            self.device
                .device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.device.begin_command_buffer(cmd, &begin_info)?;

            // 3Point Stüdyo Koyu Arka Plan Rengi: [0.12, 0.13, 0.15, 1.0]
            let clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.12, 0.13, 0.15, 1.0],
                },
            };

            let render_pass_begin = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass.render_pass)
                .framebuffer(self.render_pass.framebuffers[image_index as usize])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain.extent,
                })
                .clear_values(std::slice::from_ref(&clear_value));

            self.device.device.cmd_begin_render_pass(
                cmd,
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            );
        }

        Ok(Some(FrameContext {
            image_index,
            command_buffer: cmd,
            extent: self.swapchain.extent,
        }))
    }

    /// Kare çizimini sonlandırır, komutları kuyruğa gönderir ve ekrana sunar.
    pub fn end_frame(&mut self, frame: FrameContext, width: u32, height: u32) -> Result<(), GfxError> {
        let cmd = frame.command_buffer;

        // SAFETY: RenderPass ve komut tamponu kaydını sonlandır.
        unsafe {
            self.device.device.cmd_end_render_pass(cmd);
            self.device.device.end_command_buffer(cmd)?;
        }

        let wait_semaphores = [self.sync.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.sync.render_finished_semaphores[self.current_frame]];
        let command_buffers = [cmd];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);

        let fence = self.sync.in_flight_fences[self.current_frame];

        // SAFETY: Komutları grafik kuyruğuna gönder.
        unsafe {
            self.device
                .device
                .queue_submit(self.device.graphics_queue, &[submit_info], fence)?;
        }

        let swapchains = [self.swapchain.swapchain];
        let image_indices = [frame.image_index];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        // SAFETY: Görüntüyü ekrana sun.
        let present_result = unsafe {
            self.swapchain
                .loader
                .queue_present(self.device.present_queue, &present_info)
        };

        match present_result {
            Ok(suboptimal) => {
                if suboptimal || self.framebuffer_resized {
                    self.recreate_swapchain(width, height)?;
                }
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain(width, height)?;
            }
            Err(e) => return Err(GfxError::VkResult(e)),
        }

        // Bir sonraki kare döngüsüne ilerle
        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    /// Ekran boyutu değiştiğinde Swapchain ve Framebuffer nesnelerini sıfırlar.
    pub fn recreate_swapchain(&mut self, width: u32, height: u32) -> Result<(), GfxError> {
        if width == 0 || height == 0 {
            return Ok(());
        }

        // SAFETY: Mevcut çizimlerin tamamlanmasını bekle.
        unsafe {
            self.device.device.device_wait_idle()?;
        }

        self.render_pass.destroy_framebuffers(&self.device.device);

        let old_swapchain = self.swapchain.swapchain;
        self.swapchain.destroy_image_views(&self.device.device);

        let new_swapchain = VulkanSwapchain::new(
            &self.instance.instance,
            &self.surface,
            &self.device,
            width,
            height,
            Some(old_swapchain),
        )?;

        // SAFETY: Eski swapchain'i yok et.
        unsafe {
            self.swapchain.loader.destroy_swapchain(old_swapchain, None);
        }

        self.swapchain = new_swapchain;

        self.render_pass.framebuffers = VulkanRenderPass::create_framebuffers(
            &self.device.device,
            self.render_pass.render_pass,
            &self.swapchain.image_views,
            self.swapchain.extent,
        )?;

        self.framebuffer_resized = false;
        tracing::info!("Swapchain {}x{} boyutuyla yeniden oluşturuldu.", width, height);

        Ok(())
    }

    /// Pencere yeniden boyutlandırıldığında çağrılır.
    #[inline]
    pub fn notify_resize(&mut self) {
        self.framebuffer_resized = true;
    }
}

impl Drop for GfxContext {
    fn drop(&mut self) {
        // SAFETY: Cihazın boşa çıkmasını bekle ve kaynakları doğru sırayla temizle.
        unsafe {
            let _ = self.device.device.device_wait_idle();
        }

        self.commands.destroy(&self.device.device);
        self.sync.destroy(&self.device.device);
        self.render_pass.destroy(&self.device.device);
        self.swapchain.destroy(&self.device.device);
        tracing::info!("3Point GfxContext tüm kaynakları güvenle temizlendi.");
    }
}
