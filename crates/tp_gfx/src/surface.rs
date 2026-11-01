//! 3Point Vulkan Pencere Yüzeyi (Surface) Katmanı.

use crate::error::GfxError;
use ash::{
    khr::surface::Instance as SurfaceLoader,
    vk, Entry, Instance,
};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

/// Vulkan pencere yüzeyi (`vk::SurfaceKHR`) ve uzantı yükleyicisi.
pub struct VulkanSurface {
    pub loader: SurfaceLoader,
    pub surface: vk::SurfaceKHR,
}

impl VulkanSurface {
    /// İşletim sistemi penceresi üzerinden yeni bir Vulkan yüzeyi oluşturur.
    pub fn new(
        entry: &Entry,
        instance: &Instance,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self, GfxError> {
        // SAFETY: ash_window::create_surface platform API'leri üzerinden güvenli şekilde yüzey oluşturur.
        let surface = unsafe {
            ash_window::create_surface(entry, instance, display_handle, window_handle, None)
                .map_err(|e| GfxError::SurfaceCreation(format!("{e:?}")))?
        };

        let loader = SurfaceLoader::new(entry, instance);
        tracing::info!("Vulkan Surface başarıyla oluşturuldu: {:?}", surface);

        Ok(Self { loader, surface })
    }
}

impl Drop for VulkanSurface {
    fn drop(&mut self) {
        // SAFETY: Yüzey yok edilmeden önce cihazın boşa çıktığından emin olunmalıdır.
        unsafe {
            self.loader.destroy_surface(self.surface, None);
        }
        tracing::debug!("Vulkan Surface serbest bırakıldı.");
    }
}
