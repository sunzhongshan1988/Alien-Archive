use runtime::{Camera2d, Color, Rect, Renderer, Vec2, collision::rects_overlap};

use crate::{
    ui::menu_widgets::{draw_border, draw_screen_rect},
    world::{MapEntity, World},
};

use super::GameContext;

const SCAN_RANGE_PADDING: f32 = 40.0;
const SCAN_DURATION: f32 = 1.25;
const SCAN_NOTICE_TIME: f32 = 1.15;

#[derive(Default)]
pub struct ScanState {
    active_entity_id: Option<String>,
    active_codex_id: Option<String>,
    progress: f32,
    notice_timer: f32,
}

impl ScanState {
    pub fn update(
        &mut self,
        ctx: &mut GameContext,
        dt: f32,
        scan_held: bool,
        target: Option<&MapEntity>,
    ) {
        self.notice_timer = (self.notice_timer - dt).max(0.0);

        let Some(target) = target else {
            self.clear_active();
            return;
        };
        let Some(codex_id) = target.codex_id.as_deref() else {
            self.clear_active();
            return;
        };

        if ctx.scanned_codex_ids.contains(codex_id) {
            self.active_entity_id = Some(target.id.clone());
            self.active_codex_id = Some(codex_id.to_owned());
            self.progress = 1.0;
            return;
        }

        if self.active_entity_id.as_deref() != Some(target.id.as_str()) {
            self.active_entity_id = Some(target.id.clone());
            self.active_codex_id = Some(codex_id.to_owned());
            self.progress = 0.0;
        }

        if !scan_held {
            self.progress = 0.0;
            return;
        }

        self.progress = (self.progress + dt / SCAN_DURATION).min(1.0);
        if self.progress >= 1.0 {
            ctx.scanned_codex_ids.insert(codex_id.to_owned());
            self.notice_timer = SCAN_NOTICE_TIME;
        }
    }

    pub fn should_capture_scan_button(&self) -> bool {
        self.active_codex_id.is_some() && self.progress < 1.0
    }

    pub fn draw(&self, renderer: &mut dyn Renderer) {
        if self.active_codex_id.is_none() && self.notice_timer <= 0.0 {
            return;
        }

        let viewport = renderer.screen_size();
        renderer.set_camera(Camera2d::default());

        let panel = Rect::new(
            Vec2::new(viewport.x * 0.5 - 150.0, viewport.y - 104.0),
            Vec2::new(300.0, 30.0),
        );
        let rail = Rect::new(
            Vec2::new(panel.origin.x + 12.0, panel.origin.y + 11.0),
            Vec2::new(panel.size.x - 24.0, 8.0),
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
    }

    fn clear_active(&mut self) {
        self.active_entity_id = None;
        self.active_codex_id = None;
        self.progress = 0.0;
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

    fn overworld() -> World {
        World::load(
            "assets/data/maps/overworld_landing_site.ron",
            Some("player_start"),
        )
        .expect("overworld map should load")
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
        let world = overworld();
        let decoration = world
            .codex_entities()
            .find(|entity| entity.kind == MapEntityKind::Decoration)
            .expect("overworld should have codex-backed decoration");
        let player_rect = expanded_rect(decoration.rect, 1.0);

        let target = nearby_scan_target(&world, player_rect).expect("decoration should scan");

        assert_eq!(target.id, decoration.id);
    }
}
