use anyhow::Result;
use runtime::{Camera2d, Color, Rect, RenderStats, Renderer, Vec2};
use rusttype::Font;

use crate::{
    save::MeterSave,
    ui::{
        menu_widgets::{draw_border, draw_screen_rect},
        text::{TextSprite, draw_text, load_ui_font, upload_text},
    },
};

use super::{GameContext, SceneId};

const DEBUG_TEXT_SIZE: f32 = 14.0;
const DEBUG_LINE_HEIGHT: f32 = 20.0;
const DEBUG_PANEL_PADDING: f32 = 12.0;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SceneDebugSnapshot {
    pub(crate) scene_id: SceneId,
    pub(crate) scene_name: String,
    pub(crate) overlay_scene_name: Option<String>,
    pub(crate) player_position: Option<Vec2>,
    pub(crate) map_path: Option<String>,
    pub(crate) collider_count: Option<usize>,
    pub(crate) scan_target: Option<String>,
}

impl SceneDebugSnapshot {
    pub(crate) fn new(scene_id: SceneId, scene_name: impl Into<String>) -> Self {
        Self {
            scene_id,
            scene_name: scene_name.into(),
            overlay_scene_name: None,
            player_position: None,
            map_path: None,
            collider_count: None,
            scan_target: None,
        }
    }

    pub(crate) fn with_field_state(
        mut self,
        map_path: &str,
        player_position: Vec2,
        collider_count: usize,
        scan_target: Option<String>,
    ) -> Self {
        self.map_path = Some(map_path.to_owned());
        self.player_position = Some(player_position);
        self.collider_count = Some(collider_count);
        self.scan_target = scan_target;
        self
    }
}

#[derive(Default)]
pub(super) struct DebugOverlay {
    visible: bool,
    font: Option<Font<'static>>,
    text_key: String,
    lines: Vec<TextSprite>,
}

impl DebugOverlay {
    pub(super) fn is_visible(&self) -> bool {
        self.visible
    }

    pub(super) fn toggle(&mut self) {
        self.visible = !self.visible;
        self.text_key.clear();
    }

    pub(super) fn draw(
        &mut self,
        renderer: &mut dyn Renderer,
        ctx: &GameContext,
        snapshot: &SceneDebugSnapshot,
    ) -> Result<()> {
        if !self.visible {
            return Ok(());
        }

        let lines = debug_overlay_lines(ctx, snapshot, renderer.frame_stats());
        self.upload_lines(renderer, &lines)?;

        let viewport = renderer.screen_size();
        renderer.set_camera(Camera2d::default());

        let max_text_width = self
            .lines
            .iter()
            .map(|line| line.size.x)
            .fold(0.0_f32, f32::max);
        let viewport_width = (viewport.x - 32.0).max(280.0);
        let panel_width = (max_text_width + DEBUG_PANEL_PADDING * 2.0)
            .max(380.0)
            .min(viewport_width);
        let panel_height =
            DEBUG_PANEL_PADDING * 2.0 + self.lines.len() as f32 * DEBUG_LINE_HEIGHT + 2.0;
        let panel = Rect::new(Vec2::new(16.0, 16.0), Vec2::new(panel_width, panel_height));

        draw_screen_rect(
            renderer,
            viewport,
            panel,
            Color::rgba(0.005, 0.016, 0.020, 0.82),
        );
        draw_border(
            renderer,
            viewport,
            panel,
            1.0,
            Color::rgba(0.40, 0.95, 1.0, 0.50),
        );

        for (index, line) in self.lines.iter().enumerate() {
            draw_text(
                renderer,
                line,
                viewport,
                panel.origin.x + DEBUG_PANEL_PADDING,
                panel.origin.y + DEBUG_PANEL_PADDING + index as f32 * DEBUG_LINE_HEIGHT - 1.0,
                debug_line_color(index),
            );
        }

        Ok(())
    }

    fn upload_lines(&mut self, renderer: &mut dyn Renderer, lines: &[String]) -> Result<()> {
        let key = lines.join("\n");
        if self.text_key == key {
            return Ok(());
        }
        if self.font.is_none() {
            self.font = Some(load_ui_font()?);
        }
        let font = self
            .font
            .as_ref()
            .expect("debug overlay font should be loaded");

        let mut sprites = Vec::with_capacity(lines.len());
        for (index, line) in lines.iter().enumerate() {
            sprites.push(upload_text(
                renderer,
                font,
                &format!("debug_overlay_line_{index}"),
                line,
                DEBUG_TEXT_SIZE,
            )?);
        }

        self.lines = sprites;
        self.text_key = key;
        Ok(())
    }
}

pub(super) fn debug_overlay_lines(
    ctx: &GameContext,
    snapshot: &SceneDebugSnapshot,
    render_stats: RenderStats,
) -> Vec<String> {
    let scene = match snapshot.overlay_scene_name.as_deref() {
        Some(overlay) => format!(
            "scene: {:?} {} + {}",
            snapshot.scene_id, snapshot.scene_name, overlay
        ),
        None => format!("scene: {:?} {}", snapshot.scene_id, snapshot.scene_name),
    };
    let player = snapshot
        .player_position
        .map(|position| format!("player: x={:.1} y={:.1}", position.x, position.y))
        .unwrap_or_else(|| "player: -".to_owned());
    let map = snapshot
        .map_path
        .as_deref()
        .map(|path| format!("map: {path}"))
        .unwrap_or_else(|| "map: -".to_owned());
    let collider_count = snapshot
        .collider_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| "-".to_owned());
    let scan_target = snapshot.scan_target.as_deref().unwrap_or("-");

    vec![
        "F3 Debug Overlay".to_owned(),
        "world layer: collision red | interaction cyan | zones color-coded | scan yellow"
            .to_owned(),
        scene,
        player,
        map,
        format!("colliders: {collider_count}"),
        format!("scan target: {scan_target}"),
        format!(
            "render: commands {} rect {} image {} ground_chunks {}",
            render_stats.queued_commands,
            render_stats.rect_commands,
            render_stats.image_commands,
            render_stats.ground_chunk_commands
        ),
        format!(
            "gpu submit: draw_calls {} batches r{} i{} buffers {} textures {} skipped {}",
            render_stats.draw_calls,
            render_stats.rect_batches,
            render_stats.image_batches,
            render_stats.vertex_buffers,
            render_stats.loaded_textures,
            render_stats.skipped_image_commands
        ),
        format!(
            "save: {} dirty={} requested={} timer={:.1}s",
            ctx.save_path.display(),
            ctx.save_dirty,
            ctx.save_requested,
            ctx.save_timer
        ),
        format!(
            "world save: scene={} spawn={} collected={}",
            ctx.save_data.world.current_scene,
            ctx.save_data.world.spawn_id.as_deref().unwrap_or("-"),
            ctx.save_data.world.collected_entities.len()
        ),
        format!(
            "profile: lv{} xp {}/{} hp {} sta {}",
            ctx.save_data.profile.level,
            ctx.save_data.profile.xp,
            ctx.save_data.profile.xp_next,
            meter_text(ctx, "health"),
            meter_text(ctx, "stamina")
        ),
        format!(
            "meters: suit {} load {} oxygen {} radiation {} spores {}",
            meter_text(ctx, "suit"),
            meter_text(ctx, "load"),
            meter_text(ctx, "oxygen"),
            meter_text(ctx, "radiation"),
            meter_text(ctx, "spores")
        ),
        format!(
            "codex: scanned {}/{} log entries {}",
            ctx.scanned_codex_ids.len(),
            ctx.codex_database.entries().len(),
            ctx.save_data.activity_log.entries.len()
        ),
    ]
}

fn meter_text(ctx: &GameContext, id: &str) -> String {
    ctx.save_data
        .profile
        .meter(id)
        .map(format_meter)
        .unwrap_or_else(|| "-".to_owned())
}

fn format_meter(meter: &MeterSave) -> String {
    format!("{}/{}", meter.value, meter.max)
}

fn debug_line_color(index: usize) -> Color {
    if index == 0 {
        Color::rgba(0.70, 1.0, 0.96, 1.0)
    } else {
        Color::rgba(0.84, 0.96, 1.0, 0.92)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_overlay_lines_include_save_profile_and_scan_state() {
        let mut ctx = GameContext::default();
        ctx.save_dirty = true;
        let snapshot = SceneDebugSnapshot::new(SceneId::Overworld, "OverworldScene")
            .with_field_state(
                "assets/data/maps/test.ron",
                Vec2::new(12.5, -4.0),
                7,
                Some("codex.test.target".to_owned()),
            );

        let lines = debug_overlay_lines(
            &ctx,
            &snapshot,
            RenderStats {
                queued_commands: 12,
                rect_commands: 3,
                image_commands: 9,
                ground_chunk_commands: 2,
                skipped_image_commands: 1,
                rect_batches: 2,
                image_batches: 4,
                draw_calls: 6,
                vertex_buffers: 6,
                loaded_textures: 10,
            },
        );

        assert!(lines.iter().any(|line| line.contains("dirty=true")));
        assert!(lines.iter().any(|line| line.contains("hp 100/100")));
        assert!(lines.iter().any(|line| line.contains("ground_chunks 2")));
        assert!(lines.iter().any(|line| line.contains("draw_calls 6")));
        assert!(lines.iter().any(|line| line.contains("world layer:")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("scan target: codex.test.target"))
        );
    }
}
