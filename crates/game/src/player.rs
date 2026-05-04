use std::path::Path;

use anyhow::{Context, Result, bail};
use runtime::{Color, InputState, Rect, Renderer, Vec2};

const PLAYER_SPEED: f32 = 260.0;
const PLAYER_SIZE: Vec2 = Vec2::new(38.0, 54.0);
const PLAYER_SPRITE_SIZE: Vec2 = Vec2::new(96.0, 96.0);
const TOPDOWN_ANIMATION_FPS: f32 = 6.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TopdownAnimation {
    IdleDown,
    WalkDown,
    WalkLeft,
    WalkRight,
    WalkUp,
}

struct TopdownAnimationSpec {
    animation: TopdownAnimation,
    texture_id: &'static str,
    path: &'static str,
}

const TOPDOWN_ANIMATIONS: &[TopdownAnimationSpec] = &[
    TopdownAnimationSpec {
        animation: TopdownAnimation::IdleDown,
        texture_id: "player.topdown.idle_down",
        path: "assets/sprites/player/topdown/idle_down.png",
    },
    TopdownAnimationSpec {
        animation: TopdownAnimation::WalkDown,
        texture_id: "player.topdown.walk_down",
        path: "assets/sprites/player/topdown/walk_down.png",
    },
    TopdownAnimationSpec {
        animation: TopdownAnimation::WalkLeft,
        texture_id: "player.topdown.walk_left",
        path: "assets/sprites/player/topdown/walk_left.png",
    },
    TopdownAnimationSpec {
        animation: TopdownAnimation::WalkRight,
        texture_id: "player.topdown.walk_right",
        path: "assets/sprites/player/topdown/walk_right.png",
    },
    TopdownAnimationSpec {
        animation: TopdownAnimation::WalkUp,
        texture_id: "player.topdown.walk_up",
        path: "assets/sprites/player/topdown/walk_up.png",
    },
];

pub struct Player {
    pub position: Vec2,
    animation_time: f32,
    topdown_animation: TopdownAnimation,
}

impl Player {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            animation_time: 0.0,
            topdown_animation: TopdownAnimation::IdleDown,
        }
    }

    pub fn load_topdown_assets(renderer: &mut dyn Renderer) -> Result<()> {
        for spec in TOPDOWN_ANIMATIONS {
            if renderer.texture_size(spec.texture_id).is_none() {
                renderer.load_texture(spec.texture_id, Path::new(spec.path))?;
            }

            let sheet_size = renderer
                .texture_size(spec.texture_id)
                .with_context(|| format!("player texture {} was not loaded", spec.path))?;
            validate_horizontal_square_sheet(spec.path, sheet_size)?;
        }

        Ok(())
    }

    pub fn update(&mut self, dt: f32, input: &InputState) {
        let movement = input.movement();
        self.position += movement * PLAYER_SPEED * dt;
        self.set_topdown_animation(animation_for_movement(movement));
        self.tick_animation(dt);
    }

    pub fn tick_animation(&mut self, dt: f32) {
        self.animation_time += dt;
    }

    pub fn rect(&self) -> Rect {
        Rect::new(
            Vec2::new(
                self.position.x - PLAYER_SIZE.x * 0.5,
                self.position.y - PLAYER_SIZE.y * 0.5,
            ),
            PLAYER_SIZE,
        )
    }

    pub fn draw(&self, renderer: &mut dyn Renderer) {
        renderer.draw_rect(self.rect(), Color::rgb(0.40, 0.92, 1.00));
    }

    pub fn draw_topdown(&self, renderer: &mut dyn Renderer) {
        let spec = self.topdown_animation.spec();
        let Some(sheet_size) = renderer.texture_size(spec.texture_id) else {
            renderer.draw_rect(self.rect(), Color::rgb(0.40, 0.92, 1.00));
            return;
        };

        let source = topdown_source_rect(sheet_size, self.animation_time);
        renderer.draw_image_region(
            spec.texture_id,
            centered_rect(self.position, PLAYER_SPRITE_SIZE),
            source,
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );
    }

    fn set_topdown_animation(&mut self, animation: TopdownAnimation) {
        if self.topdown_animation != animation {
            self.topdown_animation = animation;
            self.animation_time = 0.0;
        }
    }
}

impl TopdownAnimation {
    fn spec(self) -> &'static TopdownAnimationSpec {
        TOPDOWN_ANIMATIONS
            .iter()
            .find(|spec| spec.animation == self)
            .expect("topdown animation spec must exist")
    }
}

fn validate_horizontal_square_sheet(path: &str, sheet_size: Vec2) -> Result<()> {
    let frame_count = horizontal_square_frame_count(sheet_size);

    if frame_count == 0 {
        bail!(
            "expected {path} to be a horizontal square-frame sprite sheet, got {}x{}",
            sheet_size.x,
            sheet_size.y
        );
    }

    Ok(())
}

fn topdown_source_rect(sheet_size: Vec2, animation_time: f32) -> Rect {
    let frame_size = Vec2::new(sheet_size.y, sheet_size.y);
    let frame_count = horizontal_square_frame_count(sheet_size).max(1);
    let frame_index = ((animation_time * TOPDOWN_ANIMATION_FPS).floor() as usize) % frame_count;

    Rect::new(
        Vec2::new(frame_size.x * frame_index as f32, 0.0),
        frame_size,
    )
}

fn horizontal_square_frame_count(sheet_size: Vec2) -> usize {
    if sheet_size.y <= 0.0 || sheet_size.x < sheet_size.y {
        return 0;
    }

    let frame_count = sheet_size.x / sheet_size.y;
    let rounded = frame_count.round();

    if (frame_count - rounded).abs() > f32::EPSILON {
        0
    } else {
        rounded as usize
    }
}

fn animation_for_movement(movement: Vec2) -> TopdownAnimation {
    if movement.length_squared() <= f32::EPSILON {
        return TopdownAnimation::IdleDown;
    }

    if movement.x.abs() >= movement.y.abs() {
        if movement.x < 0.0 {
            TopdownAnimation::WalkLeft
        } else {
            TopdownAnimation::WalkRight
        }
    } else if movement.y < 0.0 {
        TopdownAnimation::WalkUp
    } else {
        TopdownAnimation::WalkDown
    }
}

fn centered_rect(center: Vec2, size: Vec2) -> Rect {
    Rect::new(
        Vec2::new(center.x - size.x * 0.5, center.y - size.y * 0.5),
        size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_rect_uses_square_frames_from_sheet_height() {
        let source = topdown_source_rect(Vec2::new(512.0, 128.0), 0.17);

        assert_eq!(
            source,
            Rect::new(Vec2::new(128.0, 0.0), Vec2::new(128.0, 128.0))
        );
    }

    #[test]
    fn source_rect_wraps_after_last_frame() {
        let source = topdown_source_rect(Vec2::new(512.0, 128.0), 0.68);

        assert_eq!(
            source,
            Rect::new(Vec2::new(0.0, 0.0), Vec2::new(128.0, 128.0))
        );
    }

    #[test]
    fn movement_selects_matching_topdown_animation() {
        assert_eq!(
            animation_for_movement(Vec2::new(0.0, 1.0)),
            TopdownAnimation::WalkDown
        );
        assert_eq!(
            animation_for_movement(Vec2::new(-1.0, 0.0)),
            TopdownAnimation::WalkLeft
        );
        assert_eq!(
            animation_for_movement(Vec2::new(1.0, 0.0)),
            TopdownAnimation::WalkRight
        );
        assert_eq!(
            animation_for_movement(Vec2::new(0.0, -1.0)),
            TopdownAnimation::WalkUp
        );
        assert_eq!(
            animation_for_movement(Vec2::ZERO),
            TopdownAnimation::IdleDown
        );
    }
}
