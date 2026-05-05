use std::sync::Arc;

use eframe::egui::{
    self, Color32, Context as EguiContext, FontData, FontDefinitions, FontFamily, Stroke, vec2,
};

pub(crate) const THEME_APP_BG: Color32 = Color32::from_rgb(17, 17, 16);
pub(crate) const THEME_PANEL_BG: Color32 = Color32::from_rgb(25, 25, 23);
pub(crate) const THEME_PANEL_BG_SOFT: Color32 = Color32::from_rgb(34, 34, 31);
pub(crate) const THEME_CANVAS_BG: Color32 = Color32::from_rgb(13, 13, 12);
pub(crate) const THEME_MAP_BG: Color32 = Color32::from_rgb(24, 24, 22);
pub(crate) const THEME_BORDER: Color32 = Color32::from_rgb(75, 73, 67);
pub(crate) const THEME_TEXT: Color32 = Color32::from_rgb(228, 225, 215);
pub(crate) const THEME_MUTED_TEXT: Color32 = Color32::from_rgb(165, 160, 148);
pub(crate) const THEME_ACCENT: Color32 = Color32::from_rgb(176, 153, 112);
pub(crate) const THEME_ACCENT_STRONG: Color32 = Color32::from_rgb(214, 181, 118);
pub(crate) const THEME_ACCENT_DIM: Color32 = Color32::from_rgb(62, 53, 38);
pub(crate) const THEME_WARNING: Color32 = Color32::from_rgb(226, 166, 78);
pub(crate) const THEME_WARNING_BG: Color32 = Color32::from_rgb(82, 58, 28);
pub(crate) const THEME_ERROR: Color32 = Color32::from_rgb(222, 100, 82);
pub(crate) const THEME_COLLISION: Color32 = Color32::from_rgb(205, 92, 66);
pub(crate) const THEME_SELECTION: Color32 = Color32::from_rgb(213, 176, 92);
pub(crate) const THEME_MULTI_SELECTION: Color32 = Color32::from_rgb(169, 163, 131);

pub(crate) fn configure_editor_fonts(ctx: &EguiContext) {
    let mut fonts = FontDefinitions::default();
    let font_name = "alien_archive_ui".to_owned();
    fonts.font_data.insert(
        font_name.clone(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/ui.ttf"
        ))),
    );

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, font_name.clone());
    }

    ctx.set_fonts(fonts);
}

pub(crate) fn configure_editor_theme(ctx: &EguiContext) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(THEME_TEXT);
    visuals.panel_fill = THEME_PANEL_BG;
    visuals.window_fill = THEME_PANEL_BG;
    visuals.extreme_bg_color = THEME_APP_BG;
    visuals.faint_bg_color = THEME_PANEL_BG_SOFT;
    visuals.code_bg_color = Color32::from_rgb(20, 22, 20);
    visuals.warn_fg_color = THEME_WARNING;
    visuals.error_fg_color = THEME_ERROR;
    visuals.hyperlink_color = THEME_ACCENT_STRONG;
    visuals.selection.bg_fill = THEME_ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, THEME_ACCENT_STRONG);
    visuals.window_stroke = Stroke::new(1.0, THEME_BORDER);
    visuals.widgets.noninteractive.bg_fill = THEME_PANEL_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, THEME_BORDER);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(34, 37, 34);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(72, 79, 69));
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(50, 48, 42);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, THEME_ACCENT);
    visuals.widgets.active.bg_fill = THEME_ACCENT_DIM;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, THEME_ACCENT_STRONG);
    visuals.widgets.open.bg_fill = Color32::from_rgb(42, 40, 35);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, THEME_BORDER);

    ctx.set_visuals(visuals.clone());
    let mut style = (*ctx.global_style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(8.0, 4.0);
    style.spacing.menu_margin = egui::Margin::same(8);
    ctx.set_global_style(style);
}
