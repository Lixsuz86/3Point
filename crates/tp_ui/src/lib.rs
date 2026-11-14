//! # `tp_ui`
//!
//! 3Point 3D DCC Editörü için kullanıcı arayüzü (`egui`), panel yönetimi,
//! 3D transform gizmo'ları ve sahne outliner/inspector kütüphanesi.

pub mod editor;
pub mod panels;
pub mod theme;
pub mod vulkan_render;

pub use editor::{Editor, EditorEvent, EditorSceneContext};
pub use panels::{
    render_inspector, render_outliner, render_timeline, render_toolbox, render_top_menu,
    InspectorAction, InspectorData, MenuAction, SceneObject, ShadingMode, StudioMode,
    TimelineAction, TimelineData, ToolAction, TransformTool,
};
pub use theme::apply_studio_theme;
pub use vulkan_render::EguiVulkanRenderer;
