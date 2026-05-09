use std::path::Path;

use anyhow::Result;
use runtime::{Camera2d, Color, Rect, Renderer, Vec2};
use rusttype::Font;

use crate::ui::{
    menu_widgets::{draw_border, draw_corner_brackets, draw_screen_rect, draw_texture_rect},
    text::{TextSprite, draw_text, draw_text_centered, load_ui_font, upload_text},
};

use super::{GameContext, Language, SceneId, inventory_scene};

const QUICKBAR_SLOTS: usize = 6;

const HUD_PLAYER_AVATAR: &str = "hud.player_avatar";

const HUD_TEXTURES: &[(&str, &str)] = &[(
    HUD_PLAYER_AVATAR,
    "assets/images/ui/hud/hud_player_avatar.png",
)];

const PLAYER_PANEL_BASE_SIZE: Vec2 = Vec2::new(312.0, 96.0);
const QUICKBAR_SLOT_SIZE: f32 = 46.0;
const QUICKBAR_SLOT_GAP: f32 = 8.0;
const QUICKBAR_BASE_SIZE: Vec2 = Vec2::new(
    QUICKBAR_SLOT_SIZE * QUICKBAR_SLOTS as f32 + QUICKBAR_SLOT_GAP * (QUICKBAR_SLOTS as f32 - 1.0),
    58.0,
);
const COMPACT_METER_IDS: [&str; 4] = ["health", "suit", "stamina", "load"];

#[derive(Default)]
pub(super) struct FieldHud {
    font: Option<Font<'static>>,
    text_key: String,
    status_lines: Vec<TextSprite>,
    meter_labels: Vec<TextSprite>,
    quick_counts: Vec<Option<TextSprite>>,
    tried_loading_hud_textures: bool,
    tried_loading_item_icons: bool,
}

#[derive(Clone, Copy)]
struct HudLayout {
    scale: f32,
    player_panel: Rect,
    quickbar: Rect,
}

impl HudLayout {
    fn new(viewport: Vec2) -> Self {
        let scale = (viewport.y / 720.0)
            .min(viewport.x / 1280.0)
            .clamp(0.74, 1.0);

        let max_player_width = (viewport.x - 28.0 * scale).max(196.0 * scale);
        let player_width = (PLAYER_PANEL_BASE_SIZE.x * scale).min(max_player_width);
        let player_panel = Rect::new(
            Vec2::new(14.0 * scale, 14.0 * scale),
            Vec2::new(
                player_width,
                player_width * PLAYER_PANEL_BASE_SIZE.y / PLAYER_PANEL_BASE_SIZE.x,
            ),
        );

        let quickbar_width = (QUICKBAR_BASE_SIZE.x * scale).min(viewport.x - 42.0 * scale);
        let quickbar_height = quickbar_width * QUICKBAR_BASE_SIZE.y / QUICKBAR_BASE_SIZE.x;
        let quickbar = Rect::new(
            Vec2::new(
                (viewport.x - quickbar_width) * 0.5,
                viewport.y - quickbar_height - 18.0 * scale,
            ),
            Vec2::new(quickbar_width, quickbar_height),
        );

        Self {
            scale,
            player_panel,
            quickbar,
        }
    }
}

impl FieldHud {
    pub(super) fn draw(
        &mut self,
        renderer: &mut dyn Renderer,
        ctx: &GameContext,
        scene_id: SceneId,
    ) -> Result<()> {
        if !self.tried_loading_hud_textures {
            self.tried_loading_hud_textures = true;
            load_hud_textures(renderer);
        }
        if !self.tried_loading_item_icons {
            self.tried_loading_item_icons = true;
            let _ = inventory_scene::load_inventory_item_icons(renderer);
        }

        self.upload_textures_if_needed(renderer, ctx, scene_id)?;
        let viewport = renderer.screen_size();
        let layout = HudLayout::new(viewport);
        renderer.set_camera(Camera2d::default());
        self.draw_status_panel(renderer, viewport, ctx, &layout);
        self.draw_quickbar(renderer, viewport, ctx, &layout);
        Ok(())
    }

    fn draw_status_panel(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        ctx: &GameContext,
        layout: &HudLayout,
    ) {
        draw_compact_status_panel_frame(renderer, viewport, layout.player_panel, layout.scale);
        self.draw_profile_portrait(renderer, viewport, layout);
        self.draw_status_text(renderer, viewport, layout);
        self.draw_time_weather_text(renderer, viewport, layout);
        self.draw_meter_fills(renderer, viewport, ctx, layout);
    }

    fn draw_profile_portrait(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        layout: &HudLayout,
    ) {
        let scale = layout.scale;
        let portrait_frame = Rect::new(
            Vec2::new(
                layout.player_panel.origin.x + 12.0 * scale,
                layout.player_panel.origin.y + 18.0 * scale,
            ),
            Vec2::new(44.0 * scale, 44.0 * scale),
        );
        draw_screen_rect(
            renderer,
            viewport,
            portrait_frame,
            Color::rgba(0.006, 0.024, 0.030, 0.88),
        );
        draw_border(
            renderer,
            viewport,
            portrait_frame,
            1.0 * scale,
            Color::rgba(0.22, 0.82, 0.92, 0.72),
        );

        let portrait = inset_rect(portrait_frame, 5.0 * scale);

        if let Some(image_size) = renderer.texture_size(HUD_PLAYER_AVATAR) {
            draw_texture_rect(
                renderer,
                viewport,
                HUD_PLAYER_AVATAR,
                contain_rect(portrait, image_size),
                Color::rgba(1.0, 1.0, 1.0, 0.95),
            );
        } else {
            draw_screen_rect(
                renderer,
                viewport,
                portrait,
                Color::rgba(0.015, 0.052, 0.062, 0.68),
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(
                        portrait.origin.x + portrait.size.x * 0.44,
                        portrait.origin.y,
                    ),
                    Vec2::new(2.0 * scale, portrait.size.y),
                ),
                Color::rgba(0.30, 0.88, 0.96, 0.24),
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(
                        portrait.origin.x,
                        portrait.origin.y + portrait.size.y * 0.52,
                    ),
                    Vec2::new(portrait.size.x, 2.0 * scale),
                ),
                Color::rgba(0.30, 0.88, 0.96, 0.18),
            );
        }

        let environment_slot = Rect::new(
            Vec2::new(
                layout.player_panel.origin.x + 8.0 * scale,
                layout.player_panel.origin.y + 66.0 * scale,
            ),
            Vec2::new(60.0 * scale, 22.0 * scale),
        );
        draw_screen_rect(
            renderer,
            viewport,
            environment_slot,
            Color::rgba(0.006, 0.026, 0.032, 0.70),
        );
        draw_border(
            renderer,
            viewport,
            environment_slot,
            1.0 * scale,
            Color::rgba(0.20, 0.78, 0.88, 0.30),
        );
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(
                    environment_slot.origin.x + 6.0 * scale,
                    environment_slot.origin.y + 11.0 * scale,
                ),
                Vec2::new(environment_slot.size.x - 12.0 * scale, 1.0 * scale),
            ),
            Color::rgba(0.28, 0.86, 0.94, 0.18),
        );
    }

    fn draw_status_text(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &HudLayout) {
        if let Some(scene) = self.status_lines.first() {
            draw_text(
                renderer,
                scene,
                viewport,
                layout.player_panel.origin.x + 80.0 * layout.scale,
                layout.player_panel.origin.y + 9.0 * layout.scale,
                Color::rgba(0.80, 1.0, 0.96, 0.96),
            );
        }
    }

    fn draw_time_weather_text(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        layout: &HudLayout,
    ) {
        let environment_center_x = layout.player_panel.origin.x + 38.0 * layout.scale;
        if let Some(time) = self.status_lines.get(1) {
            draw_text_centered(
                renderer,
                time,
                viewport,
                environment_center_x,
                layout.player_panel.origin.y + 62.0 * layout.scale,
                Color::rgba(0.82, 1.0, 0.96, 0.96),
            );
        }
        if let Some(weather) = self.status_lines.get(2) {
            draw_text_centered(
                renderer,
                weather,
                viewport,
                environment_center_x,
                layout.player_panel.origin.y + 75.0 * layout.scale,
                Color::rgba(0.52, 0.84, 0.90, 0.92),
            );
        }
    }

    fn draw_meter_fills(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        ctx: &GameContext,
        layout: &HudLayout,
    ) {
        let label_x = layout.player_panel.origin.x + 80.0 * layout.scale;
        let bar_x = layout.player_panel.origin.x + 122.0 * layout.scale;
        let bar_width = layout.player_panel.right() - bar_x - 12.0 * layout.scale;

        for (index, meter_id) in COMPACT_METER_IDS.iter().enumerate() {
            let y = layout.player_panel.origin.y + (34.0 + index as f32 * 12.0) * layout.scale;
            if let Some(label) = self.meter_labels.get(index) {
                draw_text(
                    renderer,
                    label,
                    viewport,
                    label_x,
                    y - 5.0 * layout.scale,
                    Color::rgba(0.62, 0.84, 0.86, 0.88),
                );
            }

            let rect = Rect::new(
                Vec2::new(bar_x, y),
                Vec2::new(bar_width.max(80.0 * layout.scale), 5.0 * layout.scale),
            );
            draw_segmented_fill(
                renderer,
                viewport,
                rect,
                meter_ratio(ctx, meter_id),
                meter_color(meter_id),
                if *meter_id == "load" { 6 } else { 10 },
                layout.scale,
            );
        }
    }

    fn draw_quickbar(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        ctx: &GameContext,
        layout: &HudLayout,
    ) {
        let slots = inventory_scene::inventory_slots(&ctx.save_data.inventory);
        for quick_index in 0..QUICKBAR_SLOTS {
            let slot_index = quickbar_slot_index(ctx, quick_index);
            let slot = quickbar_slot_rect(layout.quickbar, quick_index);
            let selected = ctx.save_data.inventory.selected_slot == slot_index;

            draw_quick_slot_frame(renderer, viewport, slot, selected, layout.scale);

            if let Some(Some(item)) = slots.get(slot_index) {
                let icon_rect = inset_rect(slot, 8.0 * layout.scale);
                if !draw_texture_rect(
                    renderer,
                    viewport,
                    item.texture_id,
                    icon_rect,
                    Color::rgba(1.0, 1.0, 1.0, if item.locked { 0.64 } else { 1.0 }),
                ) {
                    draw_item_fallback(renderer, viewport, icon_rect, item.rarity_color);
                }
            }

            if let Some(Some(count)) = self.quick_counts.get(quick_index) {
                draw_text(
                    renderer,
                    count,
                    viewport,
                    slot.right() - count.size.x - 3.0 * layout.scale,
                    slot.bottom() - count.size.y - 1.0 * layout.scale,
                    Color::rgba(0.88, 1.0, 0.96, 1.0),
                );
            }
        }
    }

    fn upload_textures_if_needed(
        &mut self,
        renderer: &mut dyn Renderer,
        ctx: &GameContext,
        scene_id: SceneId,
    ) -> Result<()> {
        let status_lines = status_lines(ctx, scene_id);
        let quick_counts = quickbar_counts(ctx);
        let key = format!(
            "{}|{}",
            status_lines.join("\n"),
            quick_counts
                .iter()
                .map(|count| count.clone().unwrap_or_default())
                .collect::<Vec<_>>()
                .join(",")
        );
        if self.text_key == key {
            return Ok(());
        }
        if self.font.is_none() {
            self.font = Some(load_ui_font()?);
        }
        let font = self.font.as_ref().expect("field HUD font should be loaded");

        self.status_lines = status_lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                upload_text(
                    renderer,
                    font,
                    &format!("field_hud_status_line_{index}"),
                    line,
                    match index {
                        0 => 14.0,
                        1 | 2 => 9.0,
                        _ => 12.0,
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.meter_labels = COMPACT_METER_IDS
            .iter()
            .enumerate()
            .map(|(index, meter_id)| {
                upload_text(
                    renderer,
                    font,
                    &format!("field_hud_meter_label_{index}"),
                    meter_label(meter_id, ctx.language),
                    10.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.quick_counts = quick_counts
            .iter()
            .enumerate()
            .map(|(index, count)| {
                count
                    .as_ref()
                    .map(|count| {
                        upload_text(
                            renderer,
                            font,
                            &format!("field_hud_quick_count_{index}"),
                            count,
                            11.0,
                        )
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>>>()?;
        self.text_key = key;
        Ok(())
    }
}

fn load_hud_textures(renderer: &mut dyn Renderer) {
    for (texture_id, path) in HUD_TEXTURES {
        if renderer.texture_size(texture_id).is_none() {
            let _ = renderer.load_texture(texture_id, Path::new(path));
        }
    }
}

fn status_lines(ctx: &GameContext, scene_id: SceneId) -> Vec<String> {
    let language = ctx.language;
    vec![
        scene_label(scene_id, language).to_owned(),
        field_time_label(ctx.save_data.world.field_time_minutes),
        weather_label(&ctx.save_data.world.weather, language).to_owned(),
    ]
}

fn quickbar_counts(ctx: &GameContext) -> Vec<Option<String>> {
    let slots = inventory_scene::inventory_slots(&ctx.save_data.inventory);
    (0..QUICKBAR_SLOTS)
        .map(|quick_index| {
            let slot_index = quickbar_slot_index(ctx, quick_index);
            slots
                .get(slot_index)
                .and_then(|slot| slot.as_ref())
                .and_then(|item| (item.quantity > 1).then(|| item.quantity.to_string()))
        })
        .collect()
}

fn quickbar_slot_index(ctx: &GameContext, quick_index: usize) -> usize {
    ctx.save_data
        .inventory
        .quickbar
        .get(quick_index)
        .and_then(|slot| *slot)
        .unwrap_or(quick_index)
}

pub(super) fn quickbar_slot_at_position(viewport: Vec2, position: Vec2) -> Option<usize> {
    let layout = HudLayout::new(viewport);
    (0..QUICKBAR_SLOTS)
        .find(|index| screen_point_in_rect(position, quickbar_slot_rect(layout.quickbar, *index)))
}

fn field_time_label(minutes: u32) -> String {
    let minutes = minutes % (24 * 60);
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

fn scene_label(scene_id: SceneId, language: Language) -> &'static str {
    match (scene_id, language) {
        (SceneId::Facility, Language::Chinese) => "设施内部",
        (SceneId::Facility, Language::English) => "Facility",
        (_, Language::Chinese) => "地表探索",
        (_, Language::English) => "Overworld",
    }
}

fn weather_label(weather: &str, language: Language) -> &'static str {
    match (weather, language) {
        ("cold_mist", Language::Chinese) => "冷雾",
        ("ion_wind", Language::Chinese) => "离子风",
        ("spore_drift", Language::Chinese) => "孢子漂流",
        ("clear", Language::Chinese) => "晴朗",
        ("cold_mist", Language::English) => "Cold Mist",
        ("ion_wind", Language::English) => "Ion Wind",
        ("spore_drift", Language::English) => "Spore Drift",
        ("clear", Language::English) => "Clear",
        (_, Language::Chinese) => "未知天气",
        (_, Language::English) => "Unknown",
    }
}

fn meter_ratio(ctx: &GameContext, id: &str) -> f32 {
    ctx.save_data.profile.meter(id).map_or(0.0, |meter| {
        if meter.max == 0 {
            0.0
        } else {
            meter.value as f32 / meter.max as f32
        }
    })
}

fn meter_color(id: &str) -> Color {
    match id {
        "health" => Color::rgba(0.90, 0.30, 0.36, 0.86),
        "stamina" => Color::rgba(0.34, 0.86, 0.55, 0.84),
        "suit" => Color::rgba(0.38, 0.78, 0.98, 0.86),
        "load" => Color::rgba(0.92, 0.72, 0.40, 0.84),
        _ => Color::rgba(0.34, 0.88, 1.0, 0.86),
    }
}

fn meter_label(id: &str, language: Language) -> &'static str {
    match (id, language) {
        ("health", Language::Chinese) => "生命",
        ("suit", Language::Chinese) => "护甲",
        ("stamina", Language::Chinese) => "体力",
        ("load", Language::Chinese) => "负重",
        ("health", Language::English) => "HLT",
        ("suit", Language::English) => "SUIT",
        ("stamina", Language::English) => "STA",
        ("load", Language::English) => "LOAD",
        _ => "",
    }
}

fn draw_segmented_fill(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    rect: Rect,
    ratio: f32,
    color: Color,
    segments: usize,
    scale: f32,
) {
    let segment_count = segments.max(1);
    let fill_width = rect.size.x * ratio.clamp(0.0, 1.0);

    draw_screen_rect(
        renderer,
        viewport,
        rect,
        Color::rgba(0.012, 0.034, 0.040, 0.82),
    );
    if fill_width > 0.5 {
        let fill = Rect::new(rect.origin, Vec2::new(fill_width, rect.size.y));
        draw_screen_rect(renderer, viewport, fill, color);
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(fill.origin, Vec2::new(fill.size.x, (2.0 * scale).max(1.0))),
            Color::rgba(0.86, 1.0, 1.0, color.a * 0.30),
        );
    }

    for index in 1..segment_count {
        let x = rect.origin.x + rect.size.x * index as f32 / segment_count as f32;
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(Vec2::new(x, rect.origin.y), Vec2::new(1.0, rect.size.y)),
            Color::rgba(0.004, 0.018, 0.022, 0.62),
        );
    }
}

fn draw_compact_status_panel_frame(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    panel: Rect,
    scale: f32,
) {
    draw_screen_rect(
        renderer,
        viewport,
        panel,
        Color::rgba(0.003, 0.014, 0.018, 0.72),
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(panel.origin, Vec2::new(panel.size.x, 32.0 * scale)),
        Color::rgba(0.010, 0.044, 0.052, 0.38),
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(panel.origin.x, panel.origin.y + 10.0 * scale),
            Vec2::new(3.0 * scale, panel.size.y - 20.0 * scale),
        ),
        Color::rgba(0.20, 0.86, 0.94, 0.42),
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(panel.right() - 48.0 * scale, panel.origin.y),
            Vec2::new(28.0 * scale, 2.0 * scale),
        ),
        Color::rgba(0.95, 0.67, 0.28, 0.72),
    );
    draw_border(
        renderer,
        viewport,
        panel,
        1.0 * scale,
        Color::rgba(0.24, 0.78, 0.86, 0.32),
    );
    draw_corner_brackets(
        renderer,
        viewport,
        inset_rect(panel, 3.0 * scale),
        9.0 * scale,
        1.0 * scale,
        Color::rgba(0.48, 0.96, 1.0, 0.28),
    );
}

fn draw_quick_slot_frame(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    slot: Rect,
    selected: bool,
    scale: f32,
) {
    if selected {
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(slot.origin.x - 3.0 * scale, slot.origin.y - 3.0 * scale),
                Vec2::new(slot.size.x + 6.0 * scale, slot.size.y + 6.0 * scale),
            ),
            Color::rgba(0.16, 0.64, 0.72, 0.20),
        );
    }

    draw_screen_rect(
        renderer,
        viewport,
        slot,
        if selected {
            Color::rgba(0.012, 0.044, 0.052, 0.78)
        } else {
            Color::rgba(0.004, 0.018, 0.024, 0.58)
        },
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(slot.origin, Vec2::new(slot.size.x, 2.0 * scale)),
        if selected {
            Color::rgba(0.88, 0.68, 0.32, 0.82)
        } else {
            Color::rgba(0.28, 0.82, 0.88, 0.24)
        },
    );
    draw_border(
        renderer,
        viewport,
        slot,
        1.0 * scale,
        if selected {
            Color::rgba(0.62, 0.96, 1.0, 0.70)
        } else {
            Color::rgba(0.32, 0.72, 0.80, 0.30)
        },
    );
}

fn draw_item_fallback(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, color: Color) {
    let center = Vec2::new(
        rect.origin.x + rect.size.x * 0.5,
        rect.origin.y + rect.size.y * 0.5,
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(center.x - rect.size.x * 0.20, center.y - rect.size.y * 0.28),
            Vec2::new(rect.size.x * 0.40, rect.size.y * 0.56),
        ),
        color,
    );
    draw_border(
        renderer,
        viewport,
        rect,
        1.0,
        Color::rgba(color.r, color.g, color.b, 0.42),
    );
}

fn quickbar_slot_rect(quickbar: Rect, index: usize) -> Rect {
    let scale_x = quickbar.size.x / QUICKBAR_BASE_SIZE.x;
    Rect::new(
        Vec2::new(
            quickbar.origin.x + index as f32 * (QUICKBAR_SLOT_SIZE + QUICKBAR_SLOT_GAP) * scale_x,
            quickbar.origin.y + 6.0 * scale_x,
        ),
        Vec2::new(QUICKBAR_SLOT_SIZE * scale_x, QUICKBAR_SLOT_SIZE * scale_x),
    )
}

fn screen_point_in_rect(point: Vec2, rect: Rect) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.right()
        && point.y >= rect.origin.y
        && point.y <= rect.bottom()
}

fn contain_rect(frame: Rect, image_size: Vec2) -> Rect {
    if image_size.x <= 0.0 || image_size.y <= 0.0 {
        return frame;
    }

    let scale = (frame.size.x / image_size.x).min(frame.size.y / image_size.y);
    let size = image_size * scale;
    Rect::new(
        Vec2::new(
            frame.origin.x + (frame.size.x - size.x) * 0.5,
            frame.origin.y + (frame.size.y - size.y) * 0.5,
        ),
        size,
    )
}

fn inset_rect(rect: Rect, inset: f32) -> Rect {
    Rect::new(
        Vec2::new(rect.origin.x + inset, rect.origin.y + inset),
        Vec2::new(
            (rect.size.x - inset * 2.0).max(0.0),
            (rect.size.y - inset * 2.0).max(0.0),
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn field_time_label_formats_day_minutes() {
        assert_eq!(field_time_label(8 * 60 + 7), "08:07");
        assert_eq!(field_time_label(24 * 60 + 3), "00:03");
    }

    #[test]
    fn quickbar_uses_saved_slot_mapping() {
        let mut ctx = GameContext::default();
        ctx.save_data.inventory.quickbar = vec![Some(7), None];

        assert_eq!(quickbar_slot_index(&ctx, 0), 7);
        assert_eq!(quickbar_slot_index(&ctx, 1), 1);
    }

    #[test]
    fn quickbar_slot_hit_testing_uses_screen_position() {
        let viewport = Vec2::new(1280.0, 720.0);
        let layout = HudLayout::new(viewport);
        let slot = quickbar_slot_rect(layout.quickbar, 2);
        let center = Vec2::new(
            slot.origin.x + slot.size.x * 0.5,
            slot.origin.y + slot.size.y * 0.5,
        );

        assert_eq!(quickbar_slot_at_position(viewport, center), Some(2));
        assert_eq!(quickbar_slot_at_position(viewport, Vec2::ZERO), None);
    }

    #[test]
    fn optional_hud_texture_paths_stay_in_hud_folder() {
        let hud_root = Path::new("assets/images/ui/hud");
        for (_, path) in HUD_TEXTURES {
            assert!(
                Path::new(path).starts_with(hud_root),
                "{path} should stay in the field HUD asset folder"
            );
        }
    }
}
