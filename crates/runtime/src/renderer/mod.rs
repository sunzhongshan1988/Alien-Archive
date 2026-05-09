mod wgpu_renderer;

use std::path::Path;

use anyhow::Result;

use crate::{Camera2d, Color, Rect, Vec2};

pub use wgpu_renderer::WgpuRenderer;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderStats {
    pub queued_commands: usize,
    pub rect_commands: usize,
    pub image_commands: usize,
    pub ground_chunk_commands: usize,
    pub skipped_image_commands: usize,
    pub rect_batches: usize,
    pub image_batches: usize,
    pub draw_calls: usize,
    pub vertex_buffers: usize,
    pub loaded_textures: usize,
}

pub trait Renderer {
    fn load_texture(&mut self, id: &str, path: &Path) -> Result<()>;
    fn load_texture_rgba(&mut self, id: &str, width: u32, height: u32, rgba: &[u8]) -> Result<()>;
    fn texture_size(&self, id: &str) -> Option<Vec2>;
    fn screen_size(&self) -> Vec2;
    fn frame_stats(&self) -> RenderStats {
        RenderStats::default()
    }
    fn visible_world_rect(&self) -> Rect {
        let size = self.screen_size();
        Rect::new(Vec2::new(-size.x * 0.5, -size.y * 0.5), size)
    }
    fn set_camera(&mut self, camera: Camera2d);
    fn draw_rect(&mut self, rect: Rect, color: Color);
    fn draw_image(&mut self, texture_id: &str, rect: Rect, tint: Color);
    fn draw_image_transformed(
        &mut self,
        texture_id: &str,
        rect: Rect,
        tint: Color,
        flip_x: bool,
        rotation: i32,
    ) {
        let _ = (flip_x, rotation);
        self.draw_image(texture_id, rect, tint);
    }
    fn draw_image_region(&mut self, texture_id: &str, rect: Rect, source: Rect, tint: Color);
    fn draw_image_region_transformed(
        &mut self,
        texture_id: &str,
        rect: Rect,
        source: Rect,
        tint: Color,
        flip_x: bool,
        rotation: i32,
    ) {
        let _ = (flip_x, rotation);
        self.draw_image_region(texture_id, rect, source, tint);
    }
}
