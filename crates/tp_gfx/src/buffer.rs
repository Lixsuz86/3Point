//! 3Point Vulkan GPU Bellek ve Tampon (Buffer) Yönetimi.

use crate::error::GfxError;
use ash::{vk, Device};
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, Allocator},
    MemoryLocation,
};

/// Donanım hızlandırmalı GPU tamponu (Vertex, Index, Uniform veya Staging).
pub struct GpuBuffer {
    pub buffer: vk::Buffer,
    pub allocation: Option<Allocation>,
    pub size: u64,
    pub usage: vk::BufferUsageFlags,
}

impl GpuBuffer {
    /// Genel amaçlı yeni bir GPU tamponu oluşturur.
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        size: u64,
        usage: vk::BufferUsageFlags,
        location: MemoryLocation,
        name: &'static str,
    ) -> Result<Self, GfxError> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        // SAFETY: vkCreateBuffer çağrısı.
        let buffer = unsafe { device.create_buffer(&buffer_info, None)? };

        // SAFETY: Bellek gereksinimlerinin sorgulanması.
        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let allocation = allocator.allocate(&AllocationCreateDesc {
            name,
            requirements,
            location,
            linear: true,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
        }).map_err(|e| GfxError::Generic(format!("GPU bellek tahsis hatası ({name}): {e}")))?;

        // SAFETY: Belleğin tampona bağlanması.
        unsafe {
            device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;
        }

        Ok(Self {
            buffer,
            allocation: Some(allocation),
            size,
            usage,
        })
    }

    /// CPU tarafından doğrudan yazılabilir bir Köşe (Vertex) tamponu oluşturur.
    pub fn new_vertex_buffer(
        device: &Device,
        allocator: &mut Allocator,
        size: u64,
    ) -> Result<Self, GfxError> {
        Self::new(
            device,
            allocator,
            size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
            "Vertex Buffer",
        )
    }

    /// CPU tarafından doğrudan yazılabilir bir İndeks (Index) tamponu oluşturur.
    pub fn new_index_buffer(
        device: &Device,
        allocator: &mut Allocator,
        size: u64,
    ) -> Result<Self, GfxError> {
        Self::new(
            device,
            allocator,
            size,
            vk::BufferUsageFlags::INDEX_BUFFER,
            MemoryLocation::CpuToGpu,
            "Index Buffer",
        )
    }

    /// CPU'dan GPU'ya hızlı veri aktarımı için eşlenen (mapped) tampona doğrudan veri yazar.
    pub fn write_data<T: bytemuck::NoUninit>(&mut self, data: &[T]) -> Result<(), GfxError> {
        let byte_slice = bytemuck::cast_slice(data);
        if byte_slice.len() as u64 > self.size {
            return Err(GfxError::Generic(format!(
                "Tampon taşması: Yazılacak veri boyutu ({} bayt), tampon kapasitesini ({} bayt) aşıyor!",
                byte_slice.len(),
                self.size
            )));
        }

        if let Some(alloc) = &mut self.allocation {
            if let Some(mapped_ptr) = alloc.mapped_ptr() {
                // SAFETY:
                // 1. mapped_ptr geçerli bir CPU-mapped bellek adresini işaret etmektedir.
                // 2. byte_slice.len() tampon sınırları içinde kontrol edilmiştir.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        byte_slice.as_ptr(),
                        mapped_ptr.as_ptr().cast::<u8>(),
                        byte_slice.len(),
                    );
                }
                return Ok(());
            }
        }

        Err(GfxError::Generic("Tampon CPU belleğine eşlenemedi (Not Mapped)!".into()))
    }

    /// Tamponu ve bağlı bellek tahsisini serbest bırakır.
    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        // SAFETY: vkDestroyBuffer çağrısı.
        unsafe {
            device.destroy_buffer(self.buffer, None);
        }

        if let Some(alloc) = self.allocation.take() {
            let _ = allocator.free(alloc);
        }
    }
}
