//! 3Point Ana Vulkan Grafik Bağlamı (Graphics Context).

use crate::{
    command::VulkanCommands,
    debug_draw::VulkanDebugDraw,
    depth::VulkanDepth,
    device::VulkanDevice,
    error::GfxError,
    grid::VulkanGrid,
    instance::VulkanInstance,
    mesh_pipeline::VulkanMeshPipeline,
    render_pass::VulkanRenderPass,
    surface::VulkanSurface,
    swapchain::VulkanSwapchain,
    sync::{VulkanSync, MAX_FRAMES_IN_FLIGHT},
};
use ash::vk;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

/// Aktif bir karenin çizim durumunu temsil eden bağlam.
pub struct FrameContext {
    pub image_index: u32,
    pub command_buffer: vk::CommandBuffer,
    pub extent: vk::Extent2D,
}

/// Tüm Vulkan bileşenlerini, boru hatlarını ve GPU bellek yöneticisini bir araya getiren ana grafik motoru arayüzü.
pub struct GfxContext {
    pub instance: VulkanInstance,
    pub surface: VulkanSurface,
    pub device: VulkanDevice,
    pub allocator: Allocator,
    pub depth: VulkanDepth,
    pub swapchain: VulkanSwapchain,
    pub render_pass: VulkanRenderPass,
    pub grid: VulkanGrid,
    pub debug_draw: VulkanDebugDraw,
    pub mesh_pipeline: VulkanMeshPipeline,
    pub sync: VulkanSync,
    pub commands: VulkanCommands,
    pub current_frame: usize,
    pub framebuffer_resized: bool,
}

impl GfxContext {
    /// Pencere ve ekran tutucuları üzerinden tam teşekküllü Vulkan grafik motorunu başlatır.
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

        let mut allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.instance.clone(),
            device: device.device.clone(),
            physical_device: device.physical_device,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        }).map_err(|e| GfxError::Generic(format!("GPU Allocator başlatılamadı: {e}")))?;

        let swapchain = VulkanSwapchain::new(
            &instance.instance,
            &surface,
            &device,
            width,
            height,
            None,
        )?;

        let depth = VulkanDepth::new(
            &instance.instance,
            &device.device,
            &mut allocator,
            device.physical_device,
            swapchain.extent,
        )?;

        let render_pass = VulkanRenderPass::new(
            &device.device,
            swapchain.format.format,
            depth.format,
            &swapchain.image_views,
            depth.view,
            swapchain.extent,
        )?;

        let grid = VulkanGrid::new(&device.device, render_pass.render_pass)?;
        let debug_draw = VulkanDebugDraw::new(&device.device, &mut allocator, render_pass.render_pass)?;
        let mesh_pipeline = VulkanMeshPipeline::new(&device.device, render_pass.render_pass)?;

        let sync = VulkanSync::new(&device.device)?;
        let commands = VulkanCommands::new(&device.device, device.queue_indices.graphics_family)?;

        tracing::info!("3Point Vulkan Grafik Bağlamı (GfxContext - Grid, Mesh, Debug) başarıyla kuruldu.");

        Ok(Self {
            instance,
            surface,
            device,
            allocator,
            depth,
            swapchain,
            render_pass,
            grid,
            debug_draw,
            mesh_pipeline,
            sync,
            commands,
            current_frame: 0,
            framebuffer_resized: false,
        })
    }

    /// Yeni bir karenin çizimini başlatır.
    pub fn begin_frame(&mut self, width: u32, height: u32) -> Result<Option<FrameContext>, GfxError> {
        if width == 0 || height == 0 {
            return Ok(None);
        }

        let fence = self.sync.in_flight_fences[self.current_frame];

        // SAFETY: Önceki karenin tamamlanmasını bekle.
        unsafe {
            self.device
                .device
                .wait_for_fences(&[fence], true, 1_000_000_000)?;
        }

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

        // SAFETY: Komut tamponunu kayda hazırla.
        unsafe {
            self.device
                .device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.device.begin_command_buffer(cmd, &begin_info)?;

            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.12, 0.13, 0.15, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let render_pass_begin = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass.render_pass)
                .framebuffer(self.render_pass.framebuffers[image_index as usize])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain.extent,
                })
                .clear_values(&clear_values);

            self.device.device.cmd_begin_render_pass(
                cmd,
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            );

            // Dinamik Viewport ve Scissor ayarları
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.swapchain.extent.width as f32,
                height: self.swapchain.extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.extent,
            };
            self.device.device.cmd_set_viewport(cmd, 0, &[viewport]);
            self.device.device.cmd_set_scissor(cmd, 0, &[scissor]);
        }

        Ok(Some(FrameContext {
            image_index,
            command_buffer: cmd,
            extent: self.swapchain.extent,
        }))
    }

    /// Kare çizimini sonlandırır, kuyruğa gönderir ve ekrana sunar.
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

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    /// Ekran boyutu değiştiğinde Swapchain, Derinlik ve Framebuffer nesnelerini yeniler.
    pub fn recreate_swapchain(&mut self, width: u32, height: u32) -> Result<(), GfxError> {
        if width == 0 || height == 0 {
            return Ok(());
        }

        // SAFETY: Çizimlerin tamamlanmasını bekle.
        unsafe {
            self.device.device.device_wait_idle()?;
        }

        self.render_pass.destroy_framebuffers(&self.device.device);

        let old_swapchain = self.swapchain.swapchain;
        self.swapchain.destroy_image_views(&self.device.device);
        self.depth.destroy(&self.device.device, &mut self.allocator);

        let new_swapchain = VulkanSwapchain::new(
            &self.instance.instance,
            &self.surface,
            &self.device,
            width,
            height,
            Some(old_swapchain),
        )?;

        // SAFETY: Eski swapchain yok edilir.
        unsafe {
            self.swapchain.loader.destroy_swapchain(old_swapchain, None);
        }

        self.swapchain = new_swapchain;

        self.depth = VulkanDepth::new(
            &self.instance.instance,
            &self.device.device,
            &mut self.allocator,
            self.device.physical_device,
            self.swapchain.extent,
        )?;

        self.render_pass.framebuffers = VulkanRenderPass::create_framebuffers(
            &self.device.device,
            self.render_pass.render_pass,
            &self.swapchain.image_views,
            self.depth.view,
            self.swapchain.extent,
        )?;

        self.framebuffer_resized = false;
        tracing::info!("Swapchain & Derinlik {}x{} boyutuyla yenilendi.", width, height);

        Ok(())
    }

    /// Pencere boyutu değiştiğinde çağrılır.
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

        self.mesh_pipeline.destroy(&self.device.device);
        self.debug_draw.destroy(&self.device.device, &mut self.allocator);
        self.grid.destroy(&self.device.device);
        self.depth.destroy(&self.device.device, &mut self.allocator);
        self.commands.destroy(&self.device.device);
        self.sync.destroy(&self.device.device);
        self.render_pass.destroy(&self.device.device);
        self.swapchain.destroy(&self.device.device);
        tracing::info!("3Point GfxContext tüm kaynakları güvenle temizlendi.");
    }
}
