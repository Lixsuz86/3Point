//! 3Point Vulkan Render Geçişi (RenderPass) ve Çerçeve Tamponları (Framebuffers).

use crate::error::GfxError;
use ash::{vk, Device};

/// Temel sahne çizim geçişi ve her swapchain görüntüsü için framebuffer yönetimi.
pub struct VulkanRenderPass {
    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl VulkanRenderPass {
    /// Swapchain formatına uygun standart bir RenderPass oluşturur.
    pub fn new(
        device: &Device,
        color_format: vk::Format,
        image_views: &[vk::ImageView],
        extent: vk::Extent2D,
    ) -> Result<Self, GfxError> {
        let color_attachment = vk::AttachmentDescription::default()
            .format(color_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_attachment_ref));

        let dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(std::slice::from_ref(&color_attachment))
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(std::slice::from_ref(&dependency));

        // SAFETY: vkCreateRenderPass geçerli parametrelerle çağrılır.
        let render_pass = unsafe { device.create_render_pass(&render_pass_info, None)? };

        let framebuffers = Self::create_framebuffers(device, render_pass, image_views, extent)?;

        tracing::info!(
            "Vulkan RenderPass ve {} adet Framebuffer başarıyla oluşturuldu.",
            framebuffers.len()
        );

        Ok(Self {
            render_pass,
            framebuffers,
        })
    }

    /// Swapchain image view'ları için framebuffer'ları oluşturur.
    pub fn create_framebuffers(
        device: &Device,
        render_pass: vk::RenderPass,
        image_views: &[vk::ImageView],
        extent: vk::Extent2D,
    ) -> Result<Vec<vk::Framebuffer>, GfxError> {
        let mut framebuffers = Vec::with_capacity(image_views.len());

        for &view in image_views {
            let attachments = [view];
            let fb_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);

            // SAFETY: vkCreateFramebuffer çağrısı.
            let fb = unsafe { device.create_framebuffer(&fb_info, None)? };
            framebuffers.push(fb);
        }

        Ok(framebuffers)
    }

    /// Framebuffer'ları temizler (Örn: Ekran boyutu değiştiğinde).
    pub fn destroy_framebuffers(&mut self, device: &Device) {
        for fb in self.framebuffers.drain(..) {
            // SAFETY: Framebuffer yok edilmesi.
            unsafe {
                device.destroy_framebuffer(fb, None);
            }
        }
    }

    /// RenderPass nesnesini ve framebuffer'ları tamamen yok eder.
    pub fn destroy(&mut self, device: &Device) {
        self.destroy_framebuffers(device);
        // SAFETY: RenderPass yok edilmesi.
        unsafe {
            device.destroy_render_pass(self.render_pass, None);
        }
        tracing::debug!("Vulkan RenderPass serbest bırakıldı.");
    }
}
