//! 3Point Saf Rust WGSL -> SPIR-V Shader Derleyici ve Modül Yöneticisi (`naga`).

use crate::error::GfxError;
use ash::{vk, Device};

/// WGSL kaynak kodunu doğrular ve Vulkan uyumlu SPIR-V ikili formatına (`Vec<u32>`) derler.
pub fn compile_wgsl_to_spirv(source: &str, stage_name: &str) -> Result<Vec<u32>, GfxError> {
    let module = naga::front::wgsl::parse_str(source).map_err(|e| {
        let err_msg = format!("WGSL ayrıştırma hatası ({stage_name}):\n{}", e.emit_to_string(source));
        tracing::error!("{err_msg}");
        GfxError::Generic(err_msg)
    })?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );

    let module_info = validator.validate(&module).map_err(|e| {
        let err_msg = format!("Shader doğrulama hatası ({stage_name}):\n{}", e.emit_to_string(source));
        tracing::error!("{err_msg}");
        GfxError::Generic(err_msg)
    })?;

    let options = naga::back::spv::Options {
        lang_version: (1, 3), // Vulkan 1.2 için SPIR-V 1.3
        flags: naga::back::spv::WriterFlags::empty(),
        binding_map: naga::back::spv::BindingMap::default(),
        ..Default::default()
    };

    let pipeline_options = naga::back::spv::PipelineOptions {
        shader_stage: match stage_name {
            "vertex" => naga::ShaderStage::Vertex,
            "fragment" => naga::ShaderStage::Fragment,
            "compute" => naga::ShaderStage::Compute,
            _ => naga::ShaderStage::Vertex,
        },
        entry_point: "main".to_string(),
    };

    let spirv = naga::back::spv::write_vec(&module, &module_info, &options, Some(&pipeline_options))
        .map_err(|e| GfxError::Generic(format!("SPIR-V üretim hatası: {e:?}")))?;

    tracing::debug!("Shader derlendi ({stage_name}): {} SPIR-V kelimesi.", spirv.len());
    Ok(spirv)
}

/// SPIR-V kodundan Vulkan `vk::ShaderModule` nesnesi üretir.
pub fn create_shader_module(device: &Device, spirv: &[u32]) -> Result<vk::ShaderModule, GfxError> {
    let create_info = vk::ShaderModuleCreateInfo::default().code(spirv);

    // SAFETY: vkCreateShaderModule geçerli bayt dizisi ile çağrılır.
    let module = unsafe { device.create_shader_module(&create_info, None)? };
    Ok(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wgsl_to_spirv_compilation() {
        let wgsl = r#"
        @vertex
        fn main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
        "#;
        let spv = compile_wgsl_to_spirv(wgsl, "vertex");
        assert!(spv.is_ok());
        assert!(!spv.unwrap().is_empty());
    }
}

