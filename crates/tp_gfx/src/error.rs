//! 3Point Vulkan Grafik Hata Türleri.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GfxError {
    #[error("Vulkan yükleme hatası (Entry): {0}")]
    LoadingError(#[from] ash::LoadingError),

    #[error("Vulkan API çağrı hatası: {0}")]
    VkResult(#[from] ash::vk::Result),

    #[error("Uygun fiziksel grafik kartı (GPU) bulunamadı.")]
    NoSuitableGpu,

    #[error("Grafik ve sunum kuyruk aileleri (Queue Families) bulunamadı.")]
    QueueFamilyNotFound,

    #[error("Pencere yüzeyi (Surface) oluşturma hatası: {0}")]
    SurfaceCreation(String),

    #[error("Swapchain formatı veya sunum modu desteklenmiyor.")]
    UnsupportedSwapchainFormat,

    #[error("Genel grafik hatası: {0}")]
    Generic(String),
}
