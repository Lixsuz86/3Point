//! 3Point Animasyon Zaman Çizelgesi Paneli (Timeline & Dopesheet).

use egui::{Color32, DragValue, RichText, Slider, Ui};

/// Zaman çizelgesi panelinden tetiklenen eylemler.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimelineAction {
    Play,
    Pause,
    Stop,
    Scrub(f32),
    AddKeyframe,
}

/// Zaman çizelgesi durum verileri.
pub struct TimelineData<'a> {
    pub current_time: &'a mut f32,
    pub duration: f32,
    pub speed: &'a mut f32,
    pub is_playing: bool,
    pub looping: &'a mut bool,
    pub fps: u32,
}

/// Alt kısımdaki animasyon zaman çizelgesi (Timeline) panelini render eder.
pub fn render_timeline(ui: &mut Ui, data: TimelineData<'_>) -> Option<TimelineAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        // 1. Oynatma Kontrol Butonları
        if data.is_playing {
            if ui.button(RichText::new("⏸").size(14.0)).clicked() {
                action = Some(TimelineAction::Pause);
            }
        } else if ui.button(RichText::new("▶").size(14.0)).clicked() {
            action = Some(TimelineAction::Play);
        }

        if ui.button(RichText::new("⏮").size(14.0)).on_hover_text("Başa Sar").clicked() {
            action = Some(TimelineAction::Stop);
        }

        ui.checkbox(data.looping, "🔁 Döngü");

        ui.separator();

        // 2. Zaman & Kare Göstergesi
        let current_frame = (*data.current_time * data.fps as f32).round() as i32;
        let total_frames = (data.duration * data.fps as f32).round() as i32;

        ui.label(
            RichText::new(format!(
                "Kare: {:03} / {:03}  ({:.2}s / {:.2}s)",
                current_frame, total_frames, *data.current_time, data.duration
            ))
            .monospace()
            .strong()
            .color(Color32::from_rgb(56, 189, 248)),
        );

        ui.separator();

        // 3. Oynatma Hızı
        ui.label("Hız:");
        ui.add(
            DragValue::new(data.speed)
                .speed(0.05)
                .range(0.1..=4.0)
                .suffix("x"),
        );

        ui.separator();

        // 4. Anahtar Kare Ekleme
        if ui
            .button(RichText::new("➕ Key").color(Color32::from_rgb(250, 204, 21)))
            .on_hover_text("Seçili nesneye/kemiğe anahtar kare ekle")
            .clicked()
        {
            action = Some(TimelineAction::AddKeyframe);
        }

        ui.separator();

        // 5. Scrubber (Zaman Çubuğu)
        let old_time = *data.current_time;
        let slider = Slider::new(data.current_time, 0.0..=data.duration.max(0.01))
            .show_value(false);
        let slider_response = ui.add(slider);

        if slider_response.changed() || (*data.current_time - old_time).abs() > 1e-5 {
            action = Some(TimelineAction::Scrub(*data.current_time));
        }
    });

    action
}

