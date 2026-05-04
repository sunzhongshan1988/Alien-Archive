use runtime::{Color, InputState, Rect, Renderer, Vec2};

const PLAYER_SPEED: f32 = 260.0;
const PLAYER_SIZE: Vec2 = Vec2::new(38.0, 54.0);

pub struct Player {
    pub position: Vec2,
}

impl Player {
    pub fn new(position: Vec2) -> Self {
        Self { position }
    }

    pub fn update(&mut self, dt: f32, input: &InputState) {
        self.position += input.movement() * PLAYER_SPEED * dt;
    }

    pub fn draw(&self, renderer: &mut dyn Renderer) {
        let origin = Vec2::new(
            self.position.x - PLAYER_SIZE.x * 0.5,
            self.position.y - PLAYER_SIZE.y * 0.5,
        );

        renderer.draw_rect(Rect::new(origin, PLAYER_SIZE), Color::rgb(0.40, 0.92, 1.00));
    }
}
