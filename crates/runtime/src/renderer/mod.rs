mod wgpu_renderer;

use std::path::Path;

use anyhow::Result;

use crate::{Color, Rect, Vec2};

pub use wgpu_renderer::WgpuRenderer;

pub trait Renderer {
    fn load_texture(&mut self, id: &str, path: &Path) -> Result<()>;
    fn load_texture_rgba(&mut self, id: &str, width: u32, height: u32, rgba: &[u8]) -> Result<()>;
    fn texture_size(&self, id: &str) -> Option<Vec2>;
    fn screen_size(&self) -> Vec2;
    fn draw_rect(&mut self, rect: Rect, color: Color);
    fn draw_image(&mut self, texture_id: &str, rect: Rect, tint: Color);
}
