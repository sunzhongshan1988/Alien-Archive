use anyhow::Result;
use runtime::{Camera2d, Color, Rect, Renderer, Vec2, collision::rects_overlap};
use rusttype::Font;

use crate::{
    ui::{
        menu_widgets::{draw_border, draw_screen_rect},
        text::{TextSprite, draw_text, load_ui_font, upload_text},
    },
    world::{MapEntity, World},
};

use super::{GameContext, Language};

const SCAN_RANGE_PADDING: f32 = 40.0;
const SCAN_DURATION: f32 = 1.25;
const SCAN_NOTICE_TIME: f32 = 1.15;

#[derive(Default)]
pub struct ScanState {
    active_entity_id: Option<String>,
    active_codex_id: Option<String>,
    display_title: String,
    display_subtitle: String,
    progress: f32,
    notice_timer: f32,
    font: Option<Font<'static>>,
    text_key: Option<String>,
    title_text: Option<TextSprite>,
    subtitle_text: Option<TextSprite>,
}

#[derive(Default)]
pub struct ScanUpdate {
    pub completed_codex_id: Option<String>,
}

impl ScanState {
    pub fn update(
        &mut self,
        ctx: &mut GameContext,
        dt: f32,
        scan_held: bool,
        target: Option<&MapEntity>,
    ) -> ScanUpdate {
        self.notice_timer = (self.notice_timer - dt).max(0.0);

        let Some(target) = target else {
            self.clear_active();
            return ScanUpdate::default();
        };
        let Some(codex_id) = target.codex_id.as_deref() else {
            self.clear_active();
            return ScanUpdate::default();
        };

        if ctx.scanned_codex_ids.contains(codex_id) {
            self.active_entity_id = Some(target.id.clone());
            self.active_codex_id = Some(codex_id.to_owned());
            self.progress = 1.0;
            self.set_display_text(ctx, codex_id, true);
            return ScanUpdate::default();
        }

        if self.active_entity_id.as_deref() != Some(target.id.as_str()) {
            self.active_entity_id = Some(target.id.clone());
            self.active_codex_id = Some(codex_id.to_owned());
            self.progress = 0.0;
        }

        if !scan_held {
            self.progress = 0.0;
            return ScanUpdate::default();
        }

        self.set_display_text(ctx, codex_id, false);
        let scan_duration = ctx
            .codex_database
            .get(codex_id)
            .and_then(|entry| entry.scan_time)
            .unwrap_or(SCAN_DURATION)
            .max(0.1);

        self.progress = (self.progress + dt / scan_duration).min(1.0);
        if self.progress >= 1.0 {
            let first_scan = ctx.complete_codex_scan(codex_id);
            self.notice_timer = SCAN_NOTICE_TIME;
            self.set_display_text(ctx, codex_id, true);
            if first_scan {
                return ScanUpdate {
                    completed_codex_id: Some(codex_id.to_owned()),
                };
            }
        }

        ScanUpdate::default()
    }

    pub fn should_capture_scan_button(&self) -> bool {
        self.active_codex_id.is_some() && self.progress < 1.0
    }

    pub fn draw(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        if self.active_codex_id.is_none() && self.notice_timer <= 0.0 {
            return Ok(());
        }
        self.upload_textures_if_needed(renderer)?;

        let viewport = renderer.screen_size();
        renderer.set_camera(Camera2d::default());

        let panel = Rect::new(
            Vec2::new(viewport.x * 0.5 - 210.0, viewport.y - 126.0),
            Vec2::new(420.0, 64.0),
        );
        let rail = Rect::new(
            Vec2::new(panel.origin.x + 18.0, panel.origin.y + 46.0),
            Vec2::new(panel.size.x - 36.0, 8.0),
        );
        let fill = Rect::new(
            rail.origin,
            Vec2::new(rail.size.x * self.progress, rail.size.y),
        );
        let glow = if self.notice_timer > 0.0 {
            Color::rgba(0.58, 1.0, 0.70, 0.72)
        } else {
            Color::rgba(0.48, 0.94, 1.0, 0.70)
        };

        draw_screen_rect(
            renderer,
            viewport,
            panel,
            Color::rgba(0.01, 0.06, 0.08, 0.58),
        );
        draw_border(
            renderer,
            viewport,
            panel,
            1.0,
            Color::rgba(0.44, 0.94, 1.0, 0.45),
        );
        draw_screen_rect(
            renderer,
            viewport,
            rail,
            Color::rgba(0.24, 0.52, 0.56, 0.28),
        );
        draw_border(
            renderer,
            viewport,
            rail,
            1.0,
            Color::rgba(0.58, 0.96, 1.0, 0.46),
        );
        draw_screen_rect(renderer, viewport, fill, glow);
        if let Some(title) = &self.title_text {
            draw_text(
                renderer,
                title,
                viewport,
                panel.origin.x + 18.0,
                panel.origin.y + 8.0,
                Color::rgba(0.86, 1.0, 0.98, 1.0),
            );
        }
        if let Some(subtitle) = &self.subtitle_text {
            draw_text(
                renderer,
                subtitle,
                viewport,
                panel.origin.x + 18.0,
                panel.origin.y + 28.0,
                Color::rgba(0.56, 0.86, 0.92, 0.96),
            );
        }

        Ok(())
    }

    fn clear_active(&mut self) {
        self.active_entity_id = None;
        self.active_codex_id = None;
        self.progress = 0.0;
    }

    fn set_display_text(&mut self, ctx: &GameContext, codex_id: &str, completed: bool) {
        let entry = ctx.codex_database.get(codex_id);
        let title = entry
            .map(|entry| entry.title.trim())
            .filter(|title| !title.is_empty())
            .unwrap_or(codex_id);
        let category = entry
            .map(|entry| entry.category.trim())
            .filter(|category| !category.is_empty())
            .unwrap_or("Unknown");
        let subtitle = format!(
            "{} · {}",
            category,
            scan_status_label(ctx.language, completed)
        );

        if self.display_title != title || self.display_subtitle != subtitle {
            self.display_title = title.to_owned();
            self.display_subtitle = subtitle;
            self.text_key = None;
        }
    }

    fn upload_textures_if_needed(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        let key = format!("{}|{}", self.display_title, self.display_subtitle);
        if self.text_key.as_deref() == Some(key.as_str()) {
            return Ok(());
        }
        if self.font.is_none() {
            self.font = Some(load_ui_font()?);
        }
        let font = self.font.as_ref().expect("scan UI font should be loaded");
        self.title_text = Some(upload_text(
            renderer,
            font,
            "scan_overlay_title",
            &self.display_title,
            18.0,
        )?);
        self.subtitle_text = Some(upload_text(
            renderer,
            font,
            "scan_overlay_subtitle",
            &self.display_subtitle,
            14.0,
        )?);
        self.text_key = Some(key);
        Ok(())
    }
}

fn scan_status_label(language: Language, completed: bool) -> &'static str {
    match (language, completed) {
        (Language::Chinese, true) => "已记录",
        (Language::Chinese, false) => "按住 Space 扫描",
        (Language::English, true) => "Recorded",
        (Language::English, false) => "Hold Space to scan",
    }
}

pub fn nearby_scan_target<'a>(world: &'a World, player_rect: Rect) -> Option<&'a MapEntity> {
    world
        .codex_entities()
        .find(|entity| rects_overlap(player_rect, expanded_rect(entity.rect, SCAN_RANGE_PADDING)))
}

fn expanded_rect(rect: Rect, padding: f32) -> Rect {
    Rect::new(
        Vec2::new(rect.origin.x - padding, rect.origin.y - padding),
        Vec2::new(rect.size.x + padding * 2.0, rect.size.y + padding * 2.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::MapEntityKind;

    fn facility_scan_target() -> World {
        World::load("assets/data/maps/facility_ruin_01.ron", Some("entry"))
            .expect("facility map should load")
    }

    fn codex_decoration_world() -> World {
        let mut document = content::MapDocument::new_landing_site();
        document.layers.entities.clear();
        document.layers.entities.push(content::EntityInstance {
            id: "codex_decoration".to_owned(),
            asset: "ow_flora_glowfungus".to_owned(),
            entity_type: "Decoration".to_owned(),
            x: 4.0,
            y: 4.0,
            scale_x: 1.0,
            scale_y: 1.0,
            z_index: 0,
            collision_rect: None,
            depth_rect: None,
            interaction_rect: None,
            unlock: None,
            transition: None,
            flip_x: false,
            rotation: 0,
        });
        let path = std::env::temp_dir().join(format!(
            "alien_archive_codex_decoration_scan_{}.ron",
            std::process::id()
        ));
        let source = ron::ser::to_string_pretty(&document, ron::ser::PrettyConfig::new())
            .expect("test map should serialize");
        std::fs::write(&path, source).expect("test map should write");
        let world = World::load(path.to_str().expect("test map path should be utf-8"), None)
            .expect("test map should load");
        let _ = std::fs::remove_file(path);
        world
    }

    #[test]
    fn holding_scan_completes_codex_entry() {
        let world = facility_scan_target();
        let target = world
            .entities(MapEntityKind::ScanTarget)
            .next()
            .expect("facility should have a scan target");
        let codex_id = target
            .codex_id
            .as_deref()
            .expect("scan target should have a codex id");
        let mut ctx = GameContext::default();
        let mut scan = ScanState::default();

        scan.update(&mut ctx, 0.75, true, Some(&target));
        assert!(!ctx.scanned_codex_ids.contains(codex_id));

        scan.update(&mut ctx, 0.75, true, Some(&target));
        assert!(ctx.scanned_codex_ids.contains(codex_id));
        assert_eq!(scan.progress, 1.0);
    }

    #[test]
    fn releasing_scan_resets_unfinished_progress() {
        let world = facility_scan_target();
        let target = world
            .entities(MapEntityKind::ScanTarget)
            .next()
            .expect("facility should have a scan target");
        let codex_id = target
            .codex_id
            .as_deref()
            .expect("scan target should have a codex id");
        let mut ctx = GameContext::default();
        let mut scan = ScanState::default();

        scan.update(&mut ctx, 0.75, true, Some(&target));
        scan.update(&mut ctx, 0.1, false, Some(&target));

        assert_eq!(scan.progress, 0.0);
        assert!(!ctx.scanned_codex_ids.contains(codex_id));
    }

    #[test]
    fn decorations_with_codex_ids_are_scan_candidates() {
        let world = codex_decoration_world();
        let decoration = world
            .codex_entities()
            .find(|entity| entity.kind == MapEntityKind::Decoration)
            .expect("fixture should have codex-backed decoration");
        let player_rect = expanded_rect(decoration.rect, 1.0);

        let target = nearby_scan_target(&world, player_rect).expect("decoration should scan");

        assert_eq!(target.id, decoration.id);
    }
}
