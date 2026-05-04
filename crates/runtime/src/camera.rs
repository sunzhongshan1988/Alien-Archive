use crate::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Camera2d {
    pub position: Vec2,
    pub zoom: f32,
}

impl Default for Camera2d {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl Camera2d {
    pub fn follow(position: Vec2) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }
}
