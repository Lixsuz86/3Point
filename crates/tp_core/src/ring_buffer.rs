//! 3Point GPU / CPU Senkronizasyonu İçin Çift/Üçlü Tamponlama (Ring Buffer).

/// GPU senkronizasyonunda "Frames-in-Flight" (CPU-GPU ardışık işleme) modellerinde
/// sıfır tahsisatla $N$ adet kaynağı döngüsel olarak yöneten halka tampon (Ring Buffer).
#[derive(Debug, Clone)]
pub struct FrameRingBuffer<T, const N: usize> {
    slots: [T; N],
    current_index: usize,
    total_cycles: u64,
}

impl<T, const N: usize> FrameRingBuffer<T, N> {
    /// Önceden üretilmiş $N$ adet elemanla tamponu başlatır.
    pub fn new(slots: [T; N]) -> Self {
        assert!(N > 0, "Halka tampon kapasitesi sıfır olamaz!");
        Self {
            slots,
            current_index: 0,
            total_cycles: 0,
        }
    }

    /// Bir sonraki kareye ilerler ve yeni aktif indeks döngüsünü ayarlar.
    #[inline]
    pub fn advance(&mut self) -> usize {
        self.current_index = (self.current_index + 1) % N;
        self.total_cycles += 1;
        self.current_index
    }

    /// Aktif karenin yuva indeksini döndürür ($0 \dots N-1$).
    #[inline]
    #[must_use]
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Aktif karenin kaynağına değişmez referans döndürür.
    #[inline]
    #[must_use]
    pub fn current(&self) -> &T {
        &self.slots[self.current_index]
    }

    /// Aktif karenin kaynağına değiştirilebilir referans döndürür.
    #[inline]
    pub fn current_mut(&mut self) -> &mut T {
        &mut self.slots[self.current_index]
    }

    /// Belirtilen indeksli yuvaya erişir.
    #[inline]
    #[must_use]
    pub fn get(&self, index: usize) -> &T {
        &self.slots[index % N]
    }

    /// Belirtilen indeksli yuvaya değiştirilebilir erişir.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> &mut T {
        &mut self.slots[index % N]
    }

    /// Toplam kaç kere ilerleme yapıldığını döndürür.
    #[inline]
    #[must_use]
    pub fn total_cycles(&self) -> u64 {
        self.total_cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_cycling() {
        let mut rb = FrameRingBuffer::new([10, 20, 30]);
        assert_eq!(*rb.current(), 10);
        assert_eq!(rb.current_index(), 0);

        rb.advance();
        assert_eq!(*rb.current(), 20);
        assert_eq!(rb.current_index(), 1);

        rb.advance();
        assert_eq!(*rb.current(), 30);
        assert_eq!(rb.current_index(), 2);

        rb.advance();
        assert_eq!(*rb.current(), 10);
        assert_eq!(rb.current_index(), 0);
    }
}

