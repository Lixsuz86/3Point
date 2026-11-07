//! 3Point Vulkan Grafik Boru Hattı (Graphics Pipeline) İnşa Edicisi.

use crate::error::GfxError;
use ash::{vk, Device};

/// Esnek, modüler Vulkan grafik pipeline oluşturucu.
pub struct GraphicsPipelineBuilder<'a> {
    device: &'a Device,
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    vertex_bindings: Vec<vk::VertexInputBindingDescription>,
    vertex_attributes: Vec<vk::VertexInputAttributeDescription>,
    topology: vk::PrimitiveTopology,
    polygon_mode: vk::PolygonMode,
    cull_mode: vk::CullModeFlags,
    depth_test_enable: bool,
    depth_write_enable: bool,
    blend_enable: bool,
    push_constants: Vec<vk::PushConstantRange>,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    render_pass: vk::RenderPass,
}

impl<'a> GraphicsPipelineBuilder<'a> {
    /// Yeni bir pipeline builder başlatır.
    pub fn new(
        device: &'a Device,
        vertex_shader: vk::ShaderModule,
        fragment_shader: vk::ShaderModule,
        render_pass: vk::RenderPass,
    ) -> Self {
        Self {
            device,
            vertex_shader,
            fragment_shader,
            vertex_bindings: Vec::new(),
            vertex_attributes: Vec::new(),
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::BACK,
            depth_test_enable: true,
            depth_write_enable: true,
            blend_enable: false,
            push_constants: Vec::new(),
            descriptor_set_layouts: Vec::new(),
            render_pass,
        }
    }

    /// Köşe düzeni (Vertex Layout) tanımlar.
    #[must_use]
    pub fn with_vertex_layout(
        mut self,
        bindings: Vec<vk::VertexInputBindingDescription>,
        attributes: Vec<vk::VertexInputAttributeDescription>,
    ) -> Self {
        self.vertex_bindings = bindings;
        self.vertex_attributes = attributes;
        self
    }

    /// Geometri topolojisini ayarlar (Örn: Çizgiler veya Üçgenler).
    #[must_use]
    pub fn with_topology(mut self, topology: vk::PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    /// Arka yüz kırpma (Cull Mode) modunu ayarlar.
    #[must_use]
    pub fn with_cull_mode(mut self, cull_mode: vk::CullModeFlags) -> Self {
        self.cull_mode = cull_mode;
        self
    }

    /// Derinlik test ve yazma durumunu ayarlar.
    #[must_use]
    pub fn with_depth(mut self, test: bool, write: bool) -> Self {
        self.depth_test_enable = test;
        self.depth_write_enable = write;
        self
    }

    /// Renk harmanlama (Alpha Blending) durumunu ayarlar.
    #[must_use]
    pub fn with_blending(mut self, blend: bool) -> Self {
        self.blend_enable = blend;
        self
    }

    /// Push Constant aralığı ekler.
    #[must_use]
    pub fn with_push_constant(mut self, range: vk::PushConstantRange) -> Self {
        self.push_constants.push(range);
        self
    }

    /// Pipeline ve Pipeline Layout nesnelerini derleyip oluşturur.
    pub fn build(self) -> Result<(vk::Pipeline, vk::PipelineLayout), GfxError> {
        let entry_point_name = c"main";

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(self.vertex_shader)
                .name(entry_point_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(self.fragment_shader)
                .name(entry_point_name),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&self.vertex_bindings)
            .vertex_attribute_descriptions(&self.vertex_attributes);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(self.topology)
            .primitive_restart_enable(false);

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(self.polygon_mode)
            .line_width(1.0)
            .cull_mode(self.cull_mode)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(self.depth_test_enable)
            .depth_write_enable(self.depth_write_enable)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let color_blend_attachment = if self.blend_enable {
            vk::PipelineColorBlendAttachmentState::default()
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
        } else {
            vk::PipelineColorBlendAttachmentState::default()
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .blend_enable(false)
        };

        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(std::slice::from_ref(&color_blend_attachment));

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&self.descriptor_set_layouts)
            .push_constant_ranges(&self.push_constants);

        // SAFETY: vkCreatePipelineLayout çağrısı.
        let pipeline_layout = unsafe { self.device.create_pipeline_layout(&layout_info, None)? };

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(self.render_pass)
            .subpass(0);

        // SAFETY: vkCreateGraphicsPipelines çağrısı.
        let pipelines = unsafe {
            self.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, err)| GfxError::VkResult(err))?
        };

        Ok((pipelines[0], pipeline_layout))
    }
}

