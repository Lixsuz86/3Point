//! 3Point DCC Stüdyo Arayüz Teması ve Görsel Stiller.

use egui::{
    epaint::Shadow,
    style::{Interaction, Spacing, Visuals, WidgetVisuals, Widgets},
    Color32, Margin, Rounding, Stroke,
};

/// 3Point için modern, karanlık ve profesyonel stüdyo temasını yapılandırır.
pub fn apply_studio_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // Renk Paleti (Blender 4.x & Unreal 5 estetiği)
    let bg_dark = Color32::from_rgb(28, 29, 32);       // #1c1d20
    let panel_bg = Color32::from_rgb(38, 40, 44);      // #26282c
    let widget_bg = Color32::from_rgb(48, 51, 56);     // #303338
    let widget_hover = Color32::from_rgb(60, 64, 72);  // #3c4048
    let widget_active = Color32::from_rgb(56, 139, 253);// #388bfd (Accent Blue)
    let border_color = Color32::from_rgb(52, 55, 60);  // #34373c
    let text_normal = Color32::from_rgb(220, 222, 228);// #dcdee4
    let text_dim = Color32::from_rgb(140, 144, 154);   // #8c909a

    visuals.override_text_color = Some(text_normal);
    visuals.panel_fill = panel_bg;
    visuals.window_fill = bg_dark;
    visuals.faint_bg_color = widget_bg;
    visuals.extreme_bg_color = Color32::from_rgb(20, 21, 23);

    // Pencere ve Panel gölgeleri
    visuals.window_shadow = Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(120),
    };

    let rounding = Rounding::same(4.0);

    visuals.widgets = Widgets {
        noninteractive: WidgetVisuals {
            weak_bg_fill: panel_bg,
            bg_fill: panel_bg,
            bg_stroke: Stroke::new(1.0_f32, border_color),
            rounding,
            fg_stroke: Stroke::new(1.0_f32, text_dim),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            weak_bg_fill: widget_bg,
            bg_fill: widget_bg,
            bg_stroke: Stroke::new(1.0_f32, border_color),
            rounding,
            fg_stroke: Stroke::new(1.0_f32, text_normal),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            weak_bg_fill: widget_hover,
            bg_fill: widget_hover,
            bg_stroke: Stroke::new(1.0_f32, widget_active),
            rounding,
            fg_stroke: Stroke::new(1.0_f32, Color32::WHITE),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            weak_bg_fill: widget_active,
            bg_fill: widget_active,
            bg_stroke: Stroke::new(1.0_f32, Color32::WHITE),
            rounding,
            fg_stroke: Stroke::new(1.0_f32, Color32::WHITE),
            expansion: 1.0,
        },
        open: WidgetVisuals {
            weak_bg_fill: widget_hover,
            bg_fill: widget_hover,
            bg_stroke: Stroke::new(1.0_f32, widget_active),
            rounding,
            fg_stroke: Stroke::new(1.0_f32, text_normal),
            expansion: 0.0,
        },
    };

    visuals.selection.bg_fill = widget_active;
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing = Spacing {
        item_spacing: egui::vec2(6.0, 6.0),
        window_margin: Margin::same(8.0),
        button_padding: egui::vec2(8.0, 4.0),
        menu_margin: Margin::same(6.0),
        indent: 14.0,
        interact_size: egui::vec2(32.0, 22.0),
        slider_width: 120.0,
        ..Default::default()
    };
    style.interaction = Interaction {
        resize_grab_radius_side: 5.0,
        resize_grab_radius_corner: 8.0,
        show_tooltips_only_when_still: true,
        tooltip_delay: 0.2,
        ..Default::default()
    };

    ctx.set_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_studio_theme() {
        let ctx = egui::Context::default();
        apply_studio_theme(&ctx);

        let visuals = ctx.style().visuals.clone();
        assert_eq!(visuals.panel_fill, Color32::from_rgb(38, 40, 44));
        assert_eq!(visuals.window_fill, Color32::from_rgb(28, 29, 32));
        assert!(visuals.dark_mode);
    }
}
