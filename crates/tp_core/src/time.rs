//! 3Point Zaman ve Kare Ölçüm Modülü (Frame Timing).

use std::time::Instant;

/// Karesel zaman farkı ($\Delta t$), geçen toplam süre ve FPS hesaplamalarını
/// yürüten yüksek hassasiyetli zaman yöneticisi.
#[derive(Debug)]
pub struct Time {
    start_time: Instant,
    last_frame_time: Instant,
    delta_time: f32,
    elapsed_seconds: f64,
    frame_count: u64,
    fps: f32,
    fps_accumulator: f32,
    fps_frame_counter: u32,
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

impl Time {
    /// Yeni bir zamanlayıcı örneği oluşturur.
    #[must_use]
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_frame_time: now,
            delta_time: 1.0 / 60.0,
            elapsed_seconds: 0.0,
            frame_count: 0,
            fps: 60.0,
            fps_accumulator: 0.0,
            fps_frame_counter: 0,
        }
    }

    /// Her render döngüsü başında çağrılarak zaman değerlerini günceller.
    pub fn update(&mut self) {
        let now = Instant::now();
        let frame_duration = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        // Ani takılmalarda (örneğin pencere taşıma/debug break) fizik ve simülasyonun
        // patlamasını önlemek için maksimum 0.1 saniye (100ms) ile sınırla (clamping).
        self.delta_time = (frame_duration.as_secs_f32()).min(0.1);
        self.elapsed_seconds = now.duration_since(self.start_time).as_secs_f64();
        self.frame_count += 1;

        // FPS hesabı (Her 0.25 saniyede bir yumuşatılmış değer güncellenir)
        self.fps_accumulator += self.delta_time;
        self.fps_frame_counter += 1;
        if self.fps_accumulator >= 0.25 {
            self.fps = (self.fps_frame_counter as f32) / self.fps_accumulator;
            self.fps_accumulator = 0.0;
            self.fps_frame_counter = 0;
        }
    }

    /// Son iki kare arasındaki süreyi (saniye cinsinden) döndürür.
    #[inline]
    #[must_use]
    pub fn delta_seconds(&self) -> f32 {
        self.delta_time
    }

    /// Motorun başlatılmasından bu yana geçen toplam süreyi (saniye cinsinden) döndürür.
    #[inline]
    #[must_use]
    pub fn elapsed_seconds(&self) -> f64 {
        self.elapsed_seconds
    }

    /// Başlangıçtan bu yana işlenen toplam kare sayısını döndürür.
    #[inline]
    #[must_use]
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Anlık yumuşatılmış kare hızını (Saniyedeki Kare Sayısı - FPS) döndürür.
    #[inline]
    #[must_use]
    pub fn fps(&self) -> f32 {
        self.fps
    }
}

