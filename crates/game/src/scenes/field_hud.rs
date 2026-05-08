use std::path::Path;

use anyhow::Result;
use runtime::{Camera2d, Color, Rect, Renderer, Vec2};
use rusttype::Font;

use crate::ui::{
    menu_widgets::{draw_border, draw_screen_rect, draw_texture_rect, screen_rect},
    text::{TextSprite, draw_text, draw_text_centered, load_ui_font, upload_text},
};

use super::{GameContext, Language, SceneId, inventory_scene};

const QUICKBAR_SLOTS: usize = 6;

const HUD_PLAYER_PANEL: &str = "hud.player_panel";
const HUD_PLAYER_AVATAR: &str = "hud.player_avatar";
const HUD_QUICKBAR_DOCK: &str = "hud.quickbar_dock";
const HUD_QUICK_SLOT_EMPTY: &str = "hud.quick_slot_empty";
const HUD_QUICK_SLOT_SELECTED: &str = "hud.quick_slot_selected";

const HUD_TEXTURES: &[(&str, &str)] = &[
    (
        HUD_PLAYER_PANEL,
        "assets/images/ui/hud/hud_player_panel_1.png",
    ),
    (
        HUD_PLAYER_AVATAR,
        "assets/images/ui/hud/hud_player_avatar.png",
    ),
    (
        HUD_QUICKBAR_DOCK,
        "assets/images/ui/hud/hud_quickbar_dock.png",
    ),
    (
        HUD_QUICK_SLOT_EMPTY,
        "assets/images/ui/hud/hud_quick_slot_empty.png",
    ),
    (
        HUD_QUICK_SLOT_SELECTED,
        "assets/images/ui/hud/hud_quick_slot_selected.png",
    ),
];

const PLAYER_PANEL_SOURCE: Vec2 = Vec2::new(1064.0, 592.0);
const PLAYER_PANEL_CROP: Rect = Rect::new(Vec2::new(0.0, 0.0), PLAYER_PANEL_SOURCE);
const QUICKBAR_DOCK_SOURCE: Vec2 = Vec2::new(1024.0, 240.0);
const QUICKBAR_SLOT_SOURCE_SIZE: f32 = 118.0;
const QUICKBAR_SLOT_CENTERS_X: [f32; QUICKBAR_SLOTS] = [138.0, 300.0, 452.0, 603.0, 750.0, 895.5];

const PLAYER_PANEL_BASE_WIDTH: f32 = 620.0;
const QUICKBAR_BASE_WIDTH: f32 = 540.0;

#[derive(Default)]
pub(super) struct FieldHud {
    font: Option<Font<'static>>,
    text_key: String,
    status_lines: Vec<TextSprite>,
    quick_numbers: Vec<TextSprite>,
    quick_counts: Vec<Option<TextSprite>>,
    selected_item: Option<TextSprite>,
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

        let player_width = (PLAYER_PANEL_BASE_WIDTH * scale).min(viewport.x - 36.0 * scale);
        let player_panel = aspect_rect(
            Vec2::new(18.0 * scale, 18.0 * scale),
            player_width,
            PLAYER_PANEL_SOURCE,
        );

        let quickbar_width = (QUICKBAR_BASE_WIDTH * scale).min(viewport.x - 42.0 * scale);
        let quickbar_height = quickbar_width * QUICKBAR_DOCK_SOURCE.y / QUICKBAR_DOCK_SOURCE.x;
        let quickbar = Rect::new(
            Vec2::new(
                (viewport.x - quickbar_width) * 0.5,
                viewport.y - quickbar_height - 20.0 * scale,
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
        let textured = draw_texture_region_rect(
            renderer,
            viewport,
            HUD_PLAYER_PANEL,
            layout.player_panel,
            PLAYER_PANEL_CROP,
            Color::rgba(1.0, 1.0, 1.0, 0.96),
        );
        if !textured {
            draw_status_panel_fallback(renderer, viewport, layout.player_panel);
        }

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
        let portrait_frame = source_rect(
            layout.player_panel,
            PLAYER_PANEL_SOURCE,
            Rect::new(Vec2::new(54.0, 28.0), Vec2::new(336.0, 336.0)),
        );
        let portrait = inset_rect(
            portrait_frame,
            38.0 * layout.player_panel.size.x / PLAYER_PANEL_SOURCE.x,
        );

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
        }
    }

    fn draw_status_text(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &HudLayout) {
        let panel_scale = layout.player_panel.size.x / PLAYER_PANEL_SOURCE.x;
        if let Some(scene) = self.status_lines.first() {
            draw_text(
                renderer,
                scene,
                viewport,
                layout.player_panel.origin.x + 510.0 * panel_scale,
                layout.player_panel.origin.y + 45.0 * panel_scale,
                Color::rgba(0.78, 0.97, 1.0, 0.94),
            );
        }
    }

    fn draw_time_weather_text(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        layout: &HudLayout,
    ) {
        let panel_scale = layout.player_panel.size.x / PLAYER_PANEL_SOURCE.x;
        if let Some(time) = self.status_lines.get(1) {
            let rect = source_rect(
                layout.player_panel,
                PLAYER_PANEL_SOURCE,
                Rect::new(Vec2::new(48.0, 390.0), Vec2::new(336.0, 64.0)),
            );
            draw_text(
                renderer,
                time,
                viewport,
                rect.origin.x + 30.0 * panel_scale,
                rect.origin.y + (rect.size.y - time.size.y).max(0.0) * 0.5,
                Color::rgba(0.82, 1.0, 0.96, 0.96),
            );
        }
        if let Some(weather) = self.status_lines.get(2) {
            let rect = source_rect(
                layout.player_panel,
                PLAYER_PANEL_SOURCE,
                Rect::new(Vec2::new(48.0, 484.0), Vec2::new(336.0, 64.0)),
            );
            draw_text(
                renderer,
                weather,
                viewport,
                rect.origin.x + 30.0 * panel_scale,
                rect.origin.y + (rect.size.y - weather.size.y).max(0.0) * 0.5,
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
        for (index, meter_id) in ["health", "suit", "stamina", "load"].iter().enumerate() {
            let mut rect = meter_source_rect(layout.player_panel, index);
            rect.origin.x -= 2.0;
            rect.origin.y -= 2.0;
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
        let textured = draw_texture_rect(
            renderer,
            viewport,
            HUD_QUICKBAR_DOCK,
            layout.quickbar,
            Color::rgba(1.0, 1.0, 1.0, 0.96),
        );
        if !textured {
            draw_quickbar_fallback(renderer, viewport, layout.quickbar);
        }

        let slots = inventory_scene::inventory_slots(&ctx.save_data.inventory);
        for quick_index in 0..QUICKBAR_SLOTS {
            let slot_index = quickbar_slot_index(ctx, quick_index);
            let slot = quickbar_slot_rect(layout.quickbar, quick_index);
            let selected = ctx.save_data.inventory.selected_slot == slot_index;

            draw_texture_rect(
                renderer,
                viewport,
                HUD_QUICK_SLOT_EMPTY,
                slot,
                Color::rgba(1.0, 1.0, 1.0, 0.96),
            );

            if let Some(Some(item)) = slots.get(slot_index) {
                let icon_rect =
                    inset_rect(slot, 13.0 * layout.quickbar.size.x / QUICKBAR_DOCK_SOURCE.x);
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

            if selected {
                draw_texture_rect(
                    renderer,
                    viewport,
                    HUD_QUICK_SLOT_SELECTED,
                    inflate_rect(slot, 4.0 * layout.quickbar.size.x / QUICKBAR_DOCK_SOURCE.x),
                    Color::rgba(1.0, 1.0, 1.0, 0.98),
                );
            }

            if let Some(label) = self.quick_numbers.get(quick_index) {
                draw_text_centered(
                    renderer,
                    label,
                    viewport,
                    slot.origin.x + slot.size.x * 0.5,
                    layout.quickbar.origin.y
                        + 181.0 * layout.quickbar.size.x / QUICKBAR_DOCK_SOURCE.x,
                    Color::rgba(0.72, 0.90, 0.92, 0.96),
                );
            }
            if let Some(Some(count)) = self.quick_counts.get(quick_index) {
                let badge = Rect::new(
                    Vec2::new(
                        slot.right() - 30.0 * layout.quickbar.size.x / QUICKBAR_DOCK_SOURCE.x,
                        slot.bottom() - 31.0 * layout.quickbar.size.x / QUICKBAR_DOCK_SOURCE.x,
                    ),
                    Vec2::new(
                        26.0 * layout.quickbar.size.x / QUICKBAR_DOCK_SOURCE.x,
                        22.0 * layout.quickbar.size.x / QUICKBAR_DOCK_SOURCE.x,
                    ),
                );
                draw_screen_rect(
                    renderer,
                    viewport,
                    badge,
                    Color::rgba(0.000, 0.014, 0.020, 0.88),
                );
                draw_border(
                    renderer,
                    viewport,
                    badge,
                    1.0,
                    Color::rgba(0.34, 0.80, 0.94, 0.68),
                );
                draw_text_centered(
                    renderer,
                    count,
                    viewport,
                    badge.origin.x + badge.size.x * 0.5,
                    badge.origin.y - 1.0 * layout.scale,
                    Color::rgba(0.88, 1.0, 0.96, 1.0),
                );
            }
        }

        if let Some(selected_item) = &self.selected_item {
            draw_text_centered(
                renderer,
                selected_item,
                viewport,
                layout.quickbar.origin.x + layout.quickbar.size.x * 0.5,
                layout.quickbar.origin.y - 24.0 * layout.scale,
                Color::rgba(0.72, 0.92, 0.96, 0.96),
            );
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
        let selected_item = selected_item_label(ctx);
        let key = format!(
            "{}|{}|{}",
            status_lines.join("\n"),
            quick_counts
                .iter()
                .map(|count| count.clone().unwrap_or_default())
                .collect::<Vec<_>>()
                .join(","),
            selected_item
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
                        0 => 15.0,
                        1 | 2 => 16.0,
                        _ => 12.0,
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.quick_numbers = (1..=QUICKBAR_SLOTS)
            .map(|number| {
                upload_text(
                    renderer,
                    font,
                    &format!("field_hud_quick_number_{number}"),
                    &number.to_string(),
                    12.0,
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
        self.selected_item = Some(upload_text(
            renderer,
            font,
            "field_hud_selected_item",
            &selected_item,
            14.0,
        )?);
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

fn selected_item_label(ctx: &GameContext) -> String {
    let slots = inventory_scene::inventory_slots(&ctx.save_data.inventory);
    let selected_slot = ctx.save_data.inventory.selected_slot;
    let label = match ctx.language {
        Language::Chinese => "当前",
        Language::English => "Selected",
    };
    let item_name = slots
        .get(selected_slot)
        .and_then(|slot| slot.as_ref())
        .map(|item| item.name(ctx.language))
        .unwrap_or_else(|| match ctx.language {
            Language::Chinese => "空槽位",
            Language::English => "Empty Slot",
        });
    format!("{label}: {item_name}")
}

fn quickbar_slot_index(ctx: &GameContext, quick_index: usize) -> usize {
    ctx.save_data
        .inventory
        .quickbar
        .get(quick_index)
        .and_then(|slot| *slot)
        .unwrap_or(quick_index)
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
        "health" => Color::rgba(0.88, 0.24, 0.26, 0.90),
        "stamina" => Color::rgba(0.24, 0.82, 0.46, 0.90),
        "suit" => Color::rgba(0.30, 0.72, 1.0, 0.92),
        "load" => Color::rgba(0.95, 0.70, 0.30, 0.90),
        _ => Color::rgba(0.34, 0.88, 1.0, 0.90),
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
    let gap = (3.0 * scale).max(1.0);
    let segment_count = segments.max(1);
    let segment_w =
        ((rect.size.x - gap * (segment_count as f32 - 1.0)) / segment_count as f32).max(1.0);
    let filled = (ratio.clamp(0.0, 1.0) * segment_count as f32).ceil() as usize;

    for index in 0..filled.min(segment_count) {
        let segment = Rect::new(
            Vec2::new(
                rect.origin.x + index as f32 * (segment_w + gap),
                rect.origin.y,
            ),
            Vec2::new(segment_w, rect.size.y),
        );
        draw_screen_rect(renderer, viewport, segment, color);
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                segment.origin,
                Vec2::new(segment.size.x, (2.0 * scale).max(1.0)),
            ),
            Color::rgba(0.78, 1.0, 1.0, color.a * 0.40),
        );
    }
}

fn draw_status_panel_fallback(renderer: &mut dyn Renderer, viewport: Vec2, panel: Rect) {
    draw_screen_rect(
        renderer,
        viewport,
        panel,
        Color::rgba(0.004, 0.018, 0.022, 0.74),
    );
    draw_border(
        renderer,
        viewport,
        panel,
        1.0,
        Color::rgba(0.36, 0.88, 1.0, 0.46),
    );
}

fn draw_quickbar_fallback(renderer: &mut dyn Renderer, viewport: Vec2, panel: Rect) {
    draw_screen_rect(
        renderer,
        viewport,
        panel,
        Color::rgba(0.006, 0.014, 0.018, 0.72),
    );
    draw_border(
        renderer,
        viewport,
        panel,
        1.0,
        Color::rgba(0.45, 0.82, 0.92, 0.38),
    );
}

fn draw_texture_region_rect(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    texture_id: &str,
    rect: Rect,
    source: Rect,
    tint: Color,
) -> bool {
    if renderer.texture_size(texture_id).is_none() {
        return false;
    }

    renderer.draw_image_region(texture_id, screen_rect(viewport, rect), source, tint);
    true
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

fn meter_source_rect(panel: Rect, index: usize) -> Rect {
    let source = match index {
        0 => Rect::new(Vec2::new(590.0, 122.0), Vec2::new(386.0, 18.0)),
        1 => Rect::new(Vec2::new(590.0, 240.0), Vec2::new(386.0, 18.0)),
        2 => Rect::new(Vec2::new(590.0, 356.0), Vec2::new(386.0, 18.0)),
        _ => Rect::new(Vec2::new(590.0, 472.0), Vec2::new(386.0, 18.0)),
    };
    source_rect(panel, PLAYER_PANEL_SOURCE, source)
}

fn quickbar_slot_rect(quickbar: Rect, index: usize) -> Rect {
    source_rect(
        quickbar,
        QUICKBAR_DOCK_SOURCE,
        Rect::new(
            Vec2::new(
                QUICKBAR_SLOT_CENTERS_X[index] - QUICKBAR_SLOT_SOURCE_SIZE * 0.5,
                52.0,
            ),
            Vec2::new(QUICKBAR_SLOT_SOURCE_SIZE, QUICKBAR_SLOT_SOURCE_SIZE),
        ),
    )
}

fn source_rect(target: Rect, source_size: Vec2, source: Rect) -> Rect {
    let scale_x = target.size.x / source_size.x;
    let scale_y = target.size.y / source_size.y;
    Rect::new(
        Vec2::new(
            target.origin.x + source.origin.x * scale_x,
            target.origin.y + source.origin.y * scale_y,
        ),
        Vec2::new(source.size.x * scale_x, source.size.y * scale_y),
    )
}

fn aspect_rect(origin: Vec2, width: f32, source_size: Vec2) -> Rect {
    Rect::new(
        origin,
        Vec2::new(width, width * source_size.y / source_size.x),
    )
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

fn inflate_rect(rect: Rect, amount: f32) -> Rect {
    Rect::new(
        Vec2::new(rect.origin.x - amount, rect.origin.y - amount),
        Vec2::new(rect.size.x + amount * 2.0, rect.size.y + amount * 2.0),
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
    fn hud_texture_paths_exist() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (_, path) in HUD_TEXTURES {
            assert!(
                project_root.join(path).exists(),
                "{path} should exist for the field HUD"
            );
        }
    }
}
