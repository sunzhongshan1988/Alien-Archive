use anyhow::Result;
use runtime::{Button, Color, InputState, Rect, SceneCommand, Vec2};

use super::{GameContext, RenderContext, Scene, SceneId};

pub struct PauseScene;

impl PauseScene {
    pub fn new() -> Self {
        Self
    }
}

impl Scene for PauseScene {
    fn id(&self) -> SceneId {
        SceneId::Pause
    }

    fn name(&self) -> &str {
        "PauseScene"
    }

    fn update(
        &mut self,
        _ctx: &mut GameContext,
        _dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>> {
        if input.just_pressed(Button::Pause) || input.just_pressed(Button::Confirm) {
            return Ok(SceneCommand::Pop);
        }

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        let viewport = ctx.renderer.screen_size();
        ctx.renderer.draw_rect(
            Rect::new(
                Vec2::new(-viewport.x * 0.5, -viewport.y * 0.5),
                Vec2::new(viewport.x, viewport.y),
            ),
            Color::rgba(0.0, 0.0, 0.0, 0.72),
        );
        ctx.renderer.draw_rect(
            Rect::new(Vec2::new(-180.0, -42.0), Vec2::new(360.0, 84.0)),
            Color::rgba(0.06, 0.18, 0.26, 0.86),
        );
        ctx.renderer.draw_rect(
            Rect::new(Vec2::new(-150.0, -4.0), Vec2::new(300.0, 8.0)),
            Color::rgba(0.32, 0.86, 1.0, 0.88),
        );

        Ok(())
    }
}
