use std::collections::HashSet;

use crate::Vec2;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Button {
    Left,
    Right,
    Up,
    Down,
    Scan,
    Interact,
    Inventory,
    Profile,
    Confirm,
    Pause,
}

#[derive(Default)]
pub struct InputState {
    down: HashSet<Button>,
    pressed: HashSet<Button>,
    cursor_position: Option<Vec2>,
    screen_size: Vec2,
    mouse_left_down: bool,
    mouse_left_pressed: bool,
}

impl InputState {
    pub fn is_down(&self, button: Button) -> bool {
        self.down.contains(&button)
    }

    pub fn just_pressed(&self, button: Button) -> bool {
        self.pressed.contains(&button)
    }

    pub fn cursor_position(&self) -> Option<Vec2> {
        self.cursor_position
    }

    pub fn screen_size(&self) -> Vec2 {
        self.screen_size
    }

    pub fn mouse_left_just_pressed(&self) -> bool {
        self.mouse_left_pressed
    }

    pub fn movement(&self) -> Vec2 {
        let mut axis = Vec2::ZERO;

        if self.is_down(Button::Left) {
            axis.x -= 1.0;
        }
        if self.is_down(Button::Right) {
            axis.x += 1.0;
        }
        if self.is_down(Button::Up) {
            axis.y -= 1.0;
        }
        if self.is_down(Button::Down) {
            axis.y += 1.0;
        }

        axis.normalized()
    }

    pub(crate) fn apply_key_event(&mut self, event: &KeyEvent) {
        let PhysicalKey::Code(code) = event.physical_key else {
            return;
        };

        for button in key_to_buttons(code) {
            match event.state {
                ElementState::Pressed => {
                    if self.down.insert(*button) {
                        self.pressed.insert(*button);
                    }
                }
                ElementState::Released => {
                    self.down.remove(button);
                }
            }
        }
    }

    pub(crate) fn apply_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.cursor_position = Some(Vec2::new(position.x as f32, position.y as f32));
    }

    pub(crate) fn apply_window_resized(&mut self, size: PhysicalSize<u32>) {
        self.screen_size = Vec2::new(size.width as f32, size.height as f32);
    }

    pub(crate) fn apply_mouse_event(&mut self, button: MouseButton, state: ElementState) {
        if button != MouseButton::Left {
            return;
        }

        match state {
            ElementState::Pressed => {
                if !self.mouse_left_down {
                    self.mouse_left_pressed = true;
                }
                self.mouse_left_down = true;
            }
            ElementState::Released => {
                self.mouse_left_down = false;
            }
        }
    }

    pub(crate) fn clear_transitions(&mut self) {
        self.pressed.clear();
        self.mouse_left_pressed = false;
    }
}

fn key_to_buttons(code: KeyCode) -> &'static [Button] {
    match code {
        KeyCode::KeyA | KeyCode::ArrowLeft => &[Button::Left],
        KeyCode::KeyD | KeyCode::ArrowRight => &[Button::Right],
        KeyCode::KeyW | KeyCode::ArrowUp => &[Button::Up],
        KeyCode::KeyS | KeyCode::ArrowDown => &[Button::Down],
        KeyCode::KeyE => &[Button::Interact],
        KeyCode::KeyI | KeyCode::Tab => &[Button::Inventory],
        KeyCode::KeyC => &[Button::Profile],
        KeyCode::Enter => &[Button::Confirm],
        KeyCode::Escape => &[Button::Pause],
        KeyCode::Space => &[Button::Scan, Button::Confirm],
        _ => &[],
    }
}
