//! 3Point Vulkan Başlatma ve Instance Yönetimi.

use crate::error::GfxError;
use ash::{
    ext::debug_utils::Instance as DebugUtils,
    vk::{self, make_api_version},
    Entry, Instance,
};
use raw_window_handle::RawDisplayHandle;
use std::ffi::{c_char, c_void, CStr, CString};

const VALIDATION_LAYER_NAME: &CStr = c"VK_LAYER_KHRONOS_validation";

/// Vulkan instance ve hata ayıklama (validation layers) yöneticisi.
pub struct VulkanInstance {
    pub entry: Entry,
    pub instance: Instance,
    pub debug_utils: Option<DebugUtils>,
    pub debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
}

impl VulkanInstance {
    /// Yeni bir Vulkan Instance'ı oluşturur.
    ///
    /// # Parametreler
    /// - `display_handle`: Pencere sisteminin ekran tutucusu (`RawDisplayHandle`).
    /// - `enable_validation`: Debug modunda doğrulama katmanlarını etkinleştirme bayrağı.
    pub fn new(
        display_handle: RawDisplayHandle,
        enable_validation: bool,
    ) -> Result<Self, GfxError> {
        // SAFETY: Entry::load() sistemdeki Vulkan dinamik kütüphanesini (vulkan-1.dll vb.) güvenle yükler.
        let entry = unsafe { Entry::load()? };

        let app_name = CString::new("3Point Studio").map_err(|e| GfxError::Generic(e.to_string()))?;
        let engine_name = CString::new("3Point Engine").map_err(|e| GfxError::Generic(e.to_string()))?;

        // Düşük donanımlı eski GPU'lar (Intel HD 500+, eski dizüstü kartları) için
        // geniş uyumluluk sağlayan Vulkan 1.2 taban sürümü seçilmiştir.
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(make_api_version(0, 1, 2, 0));

        // Pencere yüzeyi (Surface) oluşturmak için gerekli pencere uzantıları
        let mut extension_names: Vec<*const c_char> =
            ash_window::enumerate_required_extensions(display_handle)
                .map_err(|e| GfxError::Generic(e.to_string()))?
                .to_vec();

        let mut layer_names: Vec<*const c_char> = Vec::new();
        let mut debug_utils_enabled = false;

        if enable_validation {
            // Sistemde doğrulama katmanlarının yüklü olup olmadığını sorgula
            // SAFETY: enumerate_instance_layer_properties geçerli Vulkan API çağrısıdır.
            let available_layers = unsafe { entry.enumerate_instance_layer_properties()? };
            let has_validation_layer = available_layers.iter().any(|layer| {
                // SAFETY: layer_name null ile sonlandırılmış bir C dizisidir.
                let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
                name == VALIDATION_LAYER_NAME
            });

            if has_validation_layer {
                tracing::info!("Vulkan Doğrulama Katmanı (Validation Layer) etkinleştirildi.");
                layer_names.push(VALIDATION_LAYER_NAME.as_ptr());
                extension_names.push(ash::ext::debug_utils::NAME.as_ptr());
                debug_utils_enabled = true;
            } else {
                tracing::warn!("Vulkan SDK Validation Layers bulunamadı, doğrulama katmanı atlanıyor.");
            }
        }

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .enabled_layer_names(&layer_names);

        // SAFETY: vkCreateInstance çağrısı doğrulanmış parametrelerle yürütülür.
        let instance = unsafe { entry.create_instance(&create_info, None)? };
        tracing::info!("Vulkan Instance başarıyla oluşturuldu.");

        let mut debug_utils = None;
        let mut debug_messenger = None;

        if debug_utils_enabled {
            let du = DebugUtils::new(&entry, &instance);
            let messenger_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));

            // SAFETY: create_debug_utils_messenger çağrısı geçerli struct işaretçisi ile yapılır.
            let msgr = unsafe { du.create_debug_utils_messenger(&messenger_info, None)? };
            debug_utils = Some(du);
            debug_messenger = Some(msgr);
        }

        Ok(Self {
            entry,
            instance,
            debug_utils,
            debug_messenger,
        })
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        // SAFETY: Nesneler oluşturulma sırasının tersine göre temizlenir.
        unsafe {
            if let (Some(du), Some(msgr)) = (&self.debug_utils, self.debug_messenger) {
                du.destroy_debug_utils_messenger(msgr, None);
            }
            self.instance.destroy_instance(None);
        }
        tracing::debug!("Vulkan Instance kaynakları serbest bırakıldı.");
    }
}

/// Vulkan doğrulama katmanı hata ayıklama geri çağırım fonksiyonu.
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let message = if p_callback_data.is_null() {
        "<boş veri>"
    } else {
        // SAFETY: p_callback_data geçerli işaretçi ise p_message null ile sonlandırılmış bir C dizesidir.
        let msg_ptr = (*p_callback_data).p_message;
        if msg_ptr.is_null() {
            "<boş mesaj>"
        } else {
            CStr::from_ptr(msg_ptr).to_str().unwrap_or("<geçersiz utf-8>")
        }
    };

    if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        tracing::error!("[Vulkan {:?}] {}", message_types, message);
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
        tracing::warn!("[Vulkan {:?}] {}", message_types, message);
    } else {
        tracing::debug!("[Vulkan {:?}] {}", message_types, message);
    }

    vk::FALSE
}
