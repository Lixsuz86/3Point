//! # `tp_gfx`
//!
//! 3Point motoru için modern, sıfır maliyetli ve güvenli Vulkan grafik donanım katmanı (HAL).
//! Donanım hızlandırmalı Infinite Grid, Debug Wireframe ve MatCap/PBR Mesh çizim boru hatlarını içerir.

pub mod buffer;
pub mod command;
pub mod context;
pub mod debug_draw;
pub mod depth;
pub mod device;
pub mod error;
pub mod grid;
pub mod instance;
pub mod mesh_pipeline;
pub mod pipeline;
pub mod render_graph;
pub mod render_pass;
pub mod shader;
pub mod surface;
pub mod swapchain;
pub mod sync;

pub use ash::{self, vk};
pub use buffer::GpuBuffer;
pub use context::{FrameContext, GfxContext};
pub use debug_draw::{LineVertex, VulkanDebugDraw};
pub use depth::VulkanDepth;
pub use error::GfxError;
pub use grid::{GridPushConstants, VulkanGrid};
pub use mesh_pipeline::{MeshPushConstants, MeshVertex, VulkanMeshPipeline};
pub use pipeline::GraphicsPipelineBuilder;
pub use render_graph::{PassType, RenderGraph, RenderPassNode};
